use std::{
    error::Error,
    io::{self, BufRead, IsTerminal, Write},
    process::ExitCode,
    thread,
    time::Duration,
};

use clap::Parser;
use tts::{Tts, Voice};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Read text aloud using the operating system's built-in speech engine.
///
/// With TEXT, speaks it and exits. Without TEXT, starts interactive mode:
/// each line typed on stdin is spoken; lines starting with '/' are commands
/// (type /help).
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Text to speak (multiple words are joined with spaces)
    text: Vec<String>,

    /// Voice name or id to use (see --list-voices)
    #[arg(short, long)]
    voice: Option<String>,

    /// Speech rate multiplier, e.g. 0.5 = slower, 2.0 = faster
    #[arg(short, long)]
    rate: Option<f32>,

    /// List available voices and exit (optionally filtered by name/language)
    #[arg(short, long, value_name = "FILTER", num_args = 0..=1, default_missing_value = "")]
    list_voices: Option<String>,
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<()> {
    let mut tts = Tts::default()?;

    if let Some(filter) = &args.list_voices {
        for (i, v) in filtered_voices(&tts, filter)?.iter().enumerate() {
            print_voice(i + 1, v);
        }
        return Ok(());
    }

    if let Some(wanted) = &args.voice {
        let voice = find_voice(&tts, wanted)?;
        tts.set_voice(&voice)?;
    }
    if let Some(mult) = args.rate {
        set_rate(&mut tts, mult)?;
    }

    if args.text.is_empty() {
        interactive(&mut tts)?;
    } else {
        tts.speak(args.text.join(" "), false)?;
    }
    wait_until_done(&tts)
}

/// Read lines from stdin; speak plain text, handle '/' commands.
fn interactive(tts: &mut Tts) -> Result<()> {
    let tty = io::stdin().is_terminal();
    if tty {
        println!("Reading mode. Type text and press Enter to hear it. /help for commands.");
    }
    let mut lines = io::stdin().lock().lines();

    loop {
        if tty {
            print!("> ");
            io::stdout().flush()?;
        }
        let Some(line) = lines.next() else { break };
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some(cmd) = line.strip_prefix('/') else {
            tts.speak(line, false)?;
            continue;
        };

        let (name, arg) = cmd.split_once(' ').unwrap_or((cmd, ""));
        let arg = arg.trim();
        // Errors from commands are reported but don't end the session.
        let result = match name {
            "help" | "h" | "?" => {
                print_help();
                Ok(())
            }
            "voices" | "v" => pick_voice(tts, arg, &mut lines),
            "voice" => find_voice(tts, arg).and_then(|v| {
                tts.set_voice(&v)?;
                println!("Voice set to {}.", v.name());
                Ok(())
            }),
            "rate" | "r" => arg
                .parse::<f32>()
                .map_err(|_| "usage: /rate <multiplier>, e.g. /rate 1.5".into())
                .and_then(|m| set_rate(tts, m)),
            "stop" | "s" => tts.stop().map(|_| ()).map_err(Into::into),
            "quit" | "q" | "exit" => break,
            _ => Err(format!("unknown command '/{name}' (try /help)").into()),
        };
        if let Err(e) = result {
            eprintln!("error: {e}");
        }
    }
    Ok(())
}

/// List voices (optionally filtered), let the user pick one by number.
fn pick_voice(
    tts: &mut Tts,
    filter: &str,
    lines: &mut impl Iterator<Item = io::Result<String>>,
) -> Result<()> {
    let voices = filtered_voices(tts, filter)?;
    if voices.is_empty() {
        return Err(format!("no voices match '{filter}'").into());
    }
    let current = tts.voice().ok().flatten().map(|v| v.id());
    for (i, v) in voices.iter().enumerate() {
        let marker = if current.as_deref() == Some(&*v.id()) { "*" } else { " " };
        print!("{marker}");
        print_voice(i + 1, v);
    }

    print!("Select voice number (Enter to cancel): ");
    io::stdout().flush()?;
    let Some(choice) = lines.next() else { return Ok(()) };
    let choice = choice?;
    let choice = choice.trim();
    if choice.is_empty() {
        println!("Unchanged.");
        return Ok(());
    }

    let voice = choice
        .parse::<usize>()
        .ok()
        .and_then(|n| voices.get(n.wrapping_sub(1)))
        .ok_or_else(|| format!("'{choice}' is not a number between 1 and {}", voices.len()))?;
    tts.set_voice(voice)?;
    println!("Voice set to {}. Back to reading mode.", voice.name());
    tts.speak(format!("Hello, I am {}.", voice.name()), false)?;
    Ok(())
}

fn print_help() {
    println!(
        "Type any text and press Enter to hear it. Commands:
  /voices [filter]  list voices (filter by name or language, e.g. /voices en-GB) and pick one
  /voice <name>     switch voice directly by name or id
  /rate <mult>      set speed multiplier (1.0 = normal)
  /stop             stop speaking
  /help             show this help
  /quit             exit (or Ctrl-D)"
    );
}

/// All voices whose name, language or id contains `filter` (case-insensitive).
fn filtered_voices(tts: &Tts, filter: &str) -> Result<Vec<Voice>> {
    let filter = filter.to_lowercase();
    Ok(tts
        .voices()?
        .into_iter()
        .filter(|v| {
            filter.is_empty()
                || v.name().to_lowercase().contains(&filter)
                || v.language().to_string().to_lowercase().contains(&filter)
                || v.id().to_lowercase().contains(&filter)
        })
        .collect())
}

fn find_voice(tts: &Tts, wanted: &str) -> Result<Voice> {
    let wanted = wanted.to_lowercase();
    tts.voices()?
        .into_iter()
        .find(|v| v.name().to_lowercase() == wanted || v.id().to_lowercase() == wanted)
        .ok_or_else(|| format!("voice '{wanted}' not found (try --list-voices)").into())
}

fn print_voice(n: usize, v: &Voice) {
    println!("{n:>3}. {:<28} {:<8} {}", v.name(), v.language(), v.id());
}

fn set_rate(tts: &mut Tts, mult: f32) -> Result<()> {
    let rate = (tts.normal_rate() * mult).clamp(tts.min_rate(), tts.max_rate());
    tts.set_rate(rate)?;
    Ok(())
}

/// speak() is async; block until the engine finishes.
fn wait_until_done(tts: &Tts) -> Result<()> {
    thread::sleep(Duration::from_millis(100));
    while tts.is_speaking()? {
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}
