# clispec

Score CLI tools against [The CLI Spec](https://clispec.dev).

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
