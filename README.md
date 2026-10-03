# clispec

Score CLI tools against [The CLI Spec](https://clispec.dev).

The development branch targets the v0.3 candidate contract and keeps the
published release on frozen v0.2 until v0.3 freezes.

## Install

```
cargo install clispec
brew install rvben/tap/clispec
```

### Nix

With Nix’s `nix-command` and `flakes` features enabled, packages are available
for Linux x64/ARM64 and Apple Silicon macOS:

```sh
nix run github:rvben/clispec-cli -- --help
nix build github:rvben/clispec-cli
```

Both `Cargo.lock` and `flake.lock` are committed. The package runs the Rust test
suite and checks the installed command and Bash, Fish, and Zsh completions.
Intel Macs are not supported by the pinned nixpkgs; use the other installation
methods above.

For NixOS or Home Manager, add the input to your flake:

```nix
inputs.clispec = {
  url = "github:rvben/clispec-cli";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

Then add `inputs.clispec.packages.${pkgs.stdenv.hostPlatform.system}.default`
to `environment.systemPackages` (NixOS) or `home.packages` (Home Manager).
Pass `inputs` through `specialArgs` for `nixosSystem` or `extraSpecialArgs` for
`homeManagerConfiguration`. The overlay `inputs.clispec.overlays.default`
also provides `pkgs.clispec` using your package set and Rust toolchain overrides.
Following your own nixpkgs uses its toolchain; it must meet the Rust version
required by `Cargo.toml` and support your platform.

For development:

```sh
nix develop
nix flake check                  # build, Rust tests, installed-command checks
nix fmt -- --check flake.nix nix/*.nix
```

`direnv allow` is optional and requires nix-direnv. The development shell
includes the package's native build dependencies and Rust development tools.
Update Nix inputs deliberately with `nix flake update`, review the lockfile,
and run the checks before committing it.

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

## Releasing

Vership owns versioning, changelog generation, release commits, and tags. See
[the release runbook](docs/releases.md) for the verified workflow and recovery policy.
