# firefly-cli

[ [🐙 github](https://github.com/firefly-zero/firefly-cli) ] [ [📦 crates.io](https://crates.io/crates/firefly-cli) ]

Swiss army knife CLI tool for working with [Firefly Zero](https://fireflyzero.com/): build, upload, and publish apps, control device, etc.

## 📥 Installation

Install using crates (the [Rust](https://www.rust-lang.org/tools/install) package manager):

```bash
cargo install firefly_cli
```

Or alternatively, install the latest version from the source:

```bash
git clone --depth 1 https://github.com/firefly-zero/firefly-cli.git
cd firefly-cli
cargo install --path .
```

## 🔧 Usage

```bash
# build an app and install it into VFS
firefly_cli build

# export an app installed in VFS
firefly_cli export --author sys --app input-test

# install an exported app into VFS
firefly_cli import sys.input-test.zip
```
