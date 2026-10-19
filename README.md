# random-demo

`say-it` — a small command-line text-to-speech program written in Rust.

It uses the [`tts`](https://crates.io/crates/tts) crate (MIT), which drives the
operating system's built-in speech engine:

| OS      | Backend                |
|---------|------------------------|
| macOS   | AVFoundation           |
| Windows | WinRT / SAPI           |
| Linux   | speech-dispatcher      |

No API keys, network access, or model downloads are needed.

## Build

```sh
cargo build --release
# binary: ./target/release/say-it
```

On Linux you need speech-dispatcher installed (e.g. `sudo apt install speech-dispatcher libspeechd-dev`).

## Usage

```
say-it [OPTIONS] [TEXT]...

Options:
  -v, --voice <VOICE>          Voice name or id to use (see --list-voices)
  -r, --rate <RATE>            Speech rate multiplier, e.g. 0.5 = slower, 2.0 = faster
  -l, --list-voices [FILTER]   List available voices and exit (optionally filtered by name/language)
  -h, --help                   Print help
  -V, --version                Print version
```

### Mode 1: speak a string

Pass text as arguments; it is spoken and the program exits when speech finishes.

```sh
say-it "Hello world"
say-it -v Daniel -r 1.3 "Hello in a different voice, a bit faster"
```

### Mode 2: interactive reading mode

Run with no text. Each line you type is spoken when you press Enter.
Lines starting with `/` are commands and are not spoken.

```
$ say-it
Reading mode. Type text and press Enter to hear it. /help for commands.
> hello there
> /voices en-GB
   1. Daniel                       en-GB    com.apple.voice.compact.en-GB.Daniel
   2. Eddy                         en-GB    com.apple.eloquence.en-GB.Eddy
   ...
Select voice number (Enter to cancel): 1
Voice set to Daniel. Back to reading mode.
> this is now spoken by Daniel
> /quit
```

| Command            | Effect                                                                 |
|--------------------|------------------------------------------------------------------------|
| `/voices [filter]` | List voices (filter by name, language or id, e.g. `/voices en-US`), pick one by number, then return to reading mode. Enter cancels. |
| `/voice <name>`    | Switch voice directly by name or id                                    |
| `/rate <mult>`     | Set speed multiplier (`1.0` = normal)                                  |
| `/stop`            | Stop speaking                                                          |
| `/help`            | Show commands                                                          |
| `/quit`            | Exit (Ctrl-D also works)                                               |

Notes:

- Lines typed while speech is still playing are queued, not interrupted. Use `/stop` to cut it off.
- Input can be piped: `cat notes.txt | say-it` reads the file line by line.
- `-v` and `-r` set the starting voice and rate for either mode.

### Listing voices

```sh
say-it --list-voices          # all voices
say-it -l en-GB               # filter by language
say-it -l samantha            # filter by name
```

macOS ships 180+ voices, so filtering helps. For much better quality on macOS,
download "Premium" or "Enhanced" voices in System Settings → Accessibility →
Spoken Content → System Voice → Manage Voices; they then appear in the list.

## Other free TTS options

For more natural neural voices (offline, but needs model files of ~20–60 MB per voice):

- [Piper](https://github.com/rhasspy/piper) via `sherpa-onnx` or `piper-rs`
- `any-tts` — open TTS models on Candle
