# brew-outdated

A small CLI that lists your Homebrew packages grouped by update status: need to update, up to date, and unknown.

It checks installed formulae (`brew leaves`) or casks (`brew list --cask`) in parallel using `brew info --json=v2`.

## Requirements

- [Homebrew](https://brew.sh)
- Rust (edition 2024)

## Install

Download a prebuilt binary from the [Releases](../../releases) page. Pick the archive for your platform:

- macOS (Apple Silicon): `brew-outdated-<version>-aarch64-apple-darwin.tar.gz`
- macOS (Intel): `brew-outdated-<version>-x86_64-apple-darwin.tar.gz`
- Linux (x86_64): `brew-outdated-<version>-x86_64-unknown-linux-gnu.tar.gz`

Extract and move the binary onto your `PATH`:

```sh
tar -xzf brew-outdated-<version>-<target>.tar.gz
sudo mv brew-outdated-<version>-<target>/brew-outdated /usr/local/bin/
```

On macOS, if Gatekeeper blocks the binary:

```sh
xattr -d com.apple.quarantine /usr/local/bin/brew-outdated
```

## Build

```sh
cargo build --release
```

The binary is written to `target/release/brew-outdated`.

## Usage

```sh
brew-outdated [flags]
```

- `--cask`: check installed casks instead of formulae
- `--no-update`: skip running `brew update` first
- `--help`: show usage

## License

[MIT](LICENSE)
