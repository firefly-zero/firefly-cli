# firefly-cli

[ [github](https://github.com/firefly-zero/firefly-cli) ] [ [crates.io](https://crates.io/crates/firefly-cli) ]

Swiss army knife CLI tool for working with [Firefly Zero](https://fireflyzero.com/): build, upload, and publish apps, control device, etc.

## Installation

```bash
cargo install firefly-cli
```

## Usage

```bash
# build an app and install it into VFS
firefly-cli build

# export an app installed in VFS
firefly-cli export --author sys --app input-test

# install an exported app into VFS
firefly-cli import sys.input-test.zip
```
