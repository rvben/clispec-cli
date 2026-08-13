# clispec

Score CLI tools against [The CLI Spec](https://clispec.dev).

The development branch targets the v0.3 candidate contract and keeps the
published release on frozen v0.2 until v0.3 freezes.

## Install

```
cargo install clispec
brew install rvben/tap/clispec
```

## Usage

```
clispec score proxctl
clispec score gh
clispec score kubectl
clispec score proxctl vm list    # specify subcommand to test
clispec score proxctl --json     # machine-readable output
```

## License

MIT
