# brew-outdated

A small CLI that lists your Homebrew packages grouped by update status: need to update, up to date, and unknown.

It checks installed formulae (`brew leaves`) or casks (`brew list --cask`) in parallel using `brew info --json=v2`.

## Requirements

- [Homebrew](https://brew.sh)
- Rust (edition 2024)

## Build

```sh
cargo build --release
```

The binary is written to `target/release/brew-outdated`.

## Usage

```sh
brew-outdated [flags]
```

| Flag          | Description                                |
| ------------- | ------------------------------------------ |
| `--cask`      | Check installed casks instead of formulae  |
| `--no-update` | Skip running `brew update` first           |
| `--help`      | Show usage                                 |

## License

[MIT](LICENSE)
