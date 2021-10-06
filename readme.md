# plmidi
A command line MIDI player.

# Installation
Grab a binary for your platform from the [releases page](https://github.com/insomnimus/plmidi/releases).

# Feature Flags
You can choose a different audio backend using one of the following feature flags:

-	`--features=jack`: Use the Jack backend.
-	`--features=winrt`: Use the WinRT backend.

# Building From Source
You might want to build from source if, for example you wish to use the jack backend.

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

```shell
# to install after a git clone
git clone https://github.com/insomnimus/plmidi
cd plmidi
git checkout main
cargo install --path . --locked
# To use the `jack` backend:
cargo install --path . --locked --features=jack

# here's a one liner:
# cargo install --locked --branch main --git https://github.com/insomnimus/plmidi
```

# Usage

```
plmidi <file> [OPTIONS]

OPTIONS:
    -d, --device <device>    The index of the MIDI device that will be used for synthesis. [default:
                             0]
    -h, --help               Print help information
    -l, --list               List available MIDI output devices.
    -t, --transpose <n>      Transpose the track by `n` semitones.
    -v, --verbose            Verbosity; specify twice for even more verbose output.
    -V, --version            Print version information
    -x, --speed <speed>      The playback rate, 1.0 = normal. [default: 1.0]
```