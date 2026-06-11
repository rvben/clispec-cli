# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).






## [0.2.0](https://github.com/rvben/clispec-cli/compare/v0.1.5...v0.2.0) - 2026-06-11

### Added

- score against spec v0.2 ([42f26d5](https://github.com/rvben/clispec-cli/commit/42f26d56fbe9682e2a01e308a0e92f97b94b031e))
- **schema_cmd**: declare output_fields for score command ([8de7a63](https://github.com/rvben/clispec-cli/commit/8de7a63f75987626d7d2dc9c3723942895098e4d))
- **schema**: validate target output against bundled clispec v0.1 schema ([ccef4c1](https://github.com/rvben/clispec-cli/commit/ccef4c1195da1a4a2e7188e51ebe299d10d6b22e))

### Fixed

- **checks**: accept empty global_args and require non-empty text output ([27262b2](https://github.com/rvben/clispec-cli/commit/27262b2b1232f42ea57dc2e1ccbd71bfa172eae9))
- **schema_cmd**: emit type field for args in self-described schema ([0967e04](https://github.com/rvben/clispec-cli/commit/0967e04c82254040729ea5dffc26b4c428fc9e34))
- **scorer**: use checked_div for percentage calculation ([742c6ed](https://github.com/rvben/clispec-cli/commit/742c6ed1ed1c579f4890a4edcf9b1a5b0019a8f5))
- revert to standard x-access-token git clone for homebrew ([2929826](https://github.com/rvben/clispec-cli/commit/2929826aea73dc3cbd00ba99b0f8b5a4072b0bc2))
- use GitHub API instead of git push for homebrew formula updates ([29db86f](https://github.com/rvben/clispec-cli/commit/29db86f933774b0913304d0a0efd39aa26546020))
- use username:token format for homebrew tap push ([7f6f948](https://github.com/rvben/clispec-cli/commit/7f6f948cfadc3fb708d7b45eaeefca152f0cb28d))

## [0.1.5](https://github.com/rvben/clispec-cli/compare/v0.1.4...v0.1.5) - 2026-04-03

### Fixed

- prefer simple noun-list commands in subcommand discovery ([93fdc23](https://github.com/rvben/clispec-cli/commit/93fdc23d1772e8803ecaf6a054dc5992f3f5b74b))

## [0.1.4](https://github.com/rvben/clispec-cli/compare/v0.1.3...v0.1.4) - 2026-04-03

### Fixed

- check subcommand help for flags, probe init command directly ([b145038](https://github.com/rvben/clispec-cli/commit/b1450385432073da53f197e6f75c8a823920248d))

## [0.1.3](https://github.com/rvben/clispec-cli/compare/v0.1.2...v0.1.3) - 2026-04-03

### Fixed

- remove duplicate JSON in TTY mode, improve subcommand discovery ([bb0f0df](https://github.com/rvben/clispec-cli/commit/bb0f0df17b947a50420290b91386fbd4477b3ec0))

## [0.1.2](https://github.com/rvben/clispec-cli/compare/v0.1.1...v0.1.2) - 2026-04-03

### Added

- add PyPI publishing via maturin ([1f86828](https://github.com/rvben/clispec-cli/commit/1f86828698e3b209f792e9199a9a62452dc92da6))

## [0.1.1] - 2026-04-03

### Added

- support --output/-o/--format as JSON flag alternatives ([407edaa](https://github.com/rvben/clispec-cli/commit/407edaaa6090a49d423109241cafaeccb22dae84))
- add integration tests and README ([6af0e86](https://github.com/rvben/clispec-cli/commit/6af0e86dfb966d272b49f9468a4a78e58e258d79))
- add schema command for self-compliance ([ab79875](https://github.com/rvben/clispec-cli/commit/ab7987592fa21ce9fb9f46aafa6139af59374639))
- add scorer and display modules ([194f73a](https://github.com/rvben/clispec-cli/commit/194f73a84f8c38c7204d00f5d22498baef909b4a))
- implement all 6 principle check modules ([945df11](https://github.com/rvben/clispec-cli/commit/945df110973406a1b77edd27f00c4e5657aadfb5))
- add check types and stub modules for all 6 principles ([dab9b47](https://github.com/rvben/clispec-cli/commit/dab9b47d449a091367a5100f5511cb70764d1cf9))
- add help parser for detecting flags and subcommands ([1cbff7b](https://github.com/rvben/clispec-cli/commit/1cbff7bf9397d002c8c84f23067f25e98f6ad045))
- add runner module for executing target binaries ([f1f6829](https://github.com/rvben/clispec-cli/commit/f1f68295ac4fd89e295ef9c7705319599ffaa724))
- scaffold clispec-cli project ([46dfb59](https://github.com/rvben/clispec-cli/commit/46dfb596430705d67fa12ff38e16fcc7ba6d3f10))
