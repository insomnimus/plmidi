# plmidi
A command line MIDI player with an embedded synthesizer.

# Installation
## Windows: With Scoop
First add [my bucket](https://github.com/insomnimus/scoop-bucket) to scoop:\
`scoop bucket add insomnia https://github.com/inssomnimus/scoop-bucket`

Update scoop:\
`scoop update`

Install the app:\
`scoop install plmidi`

## Download a pre-built release binary
Grab a binary for your platform from the [releases page](https://github.com/insomnimus/plmidi/releases).

## BYOB: Build Your Own Binary
# Feature Flags

- `--features=system`: Enable playback through MIDI out devices registered on the system.
- `--features=system-jack`: Same with `system` but uses the Jack backend.
- `--features=winrt`: Same with `system` except it uses the WinRT backend. Note that currently WinRT does not recognize OmniMidi or Virtual Midi Synth so I wouldn't recommend it.
- `--features=fluid`: Enable [fluidlite](https://github.com/divideconcept/FluidLite) as a built-in MIDI synthesizer (requires libfluidlite and pkg-config to be present on your system).
- `--features=fluid-bundled`: Enable [fluidlite](https://github.com/divideconcept/FluidLite) as a built-in MIDI synthesizer; use the bundled library. This feature is enabled by default.

You need an up to date rust toolchain installed.

On *NIX systems, you also need alsa development libraries:

```sh
# Debian and derivatives
apt install libasound2-dev

# RHEL and derivatives
dnf install alsa-lib-devel
```

To use the jack backend, you also need jack development libraries:

```sh
# Debian and derivatives
apt install libjack-jackd2-dev
# RHEL and derivatives
dnf install jack-audio-connection-kit-devel
```

You can install from crates.io:
`cargo install plmidi --features system`

Or, you can clone it:

```shell
# to install after a git clone
git clone https://github.com/insomnimus/plmidi
cd plmidi
git checkout main
cargo install --path .
# To enable the system apis via the `jack` backend:
cargo install --path . --features system-jack
# To disable built-in fluidsynth support:
cargo install --path . --features system --no-default-features
```

# Usage
- `plmidi foo.mid`
- (If the `system` feature is enabled) `plmidi --device 2 foo.mid`
- (If the `fluid` feature is enabled) `plmidi --fluidsynth ~/soundfonts/some-soundfont.sf2 foo.mid`

# Troubleshooting
- When playing with the system midi devices, playing next/previous track might leave playing notes hanging even though a "system reset" message is sent; if you have a solution please do a PR or create an issue.
- With any synth, pausing the track will leave previously playing notes on; there isn't any fix for the system synthesizers that I'm aware of but I'm waiting for an upstream feature for the built-in Fluidsynth.
