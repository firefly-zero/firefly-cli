# firefly-cli

[ [🐙 github](https://github.com/firefly-zero/firefly-cli) ] [ [📦 crates.io](https://crates.io/crates/firefly-cli) ]

Swiss army knife CLI tool for working with [Firefly Zero](https://fireflyzero.com/): build, upload, and publish apps, control device, etc.

## 📥 Installation

* Grab the binary from the latest [release](https://github.com/firefly-zero/firefly-cli/releases) and put it somewhere into `$PATH`.
* Or install using crates (the [Rust](https://www.rust-lang.org/tools/install) package manager):

    ```bash
    cargo install firefly_cli
    ```

* Or install dev version from the source:

    ```bash
    git clone --depth 1 https://github.com/firefly-zero/firefly-cli.git
    cd firefly-cli
    cargo install --path .
    ```

## 🔧 Usage

Some of the most commonly used commands:

```bash
# build an app and install it into VFS
ff build

# export an app installed in VFS
ff export --author sys --app input-test

# install an exported app into VFS
ff import sys.input-test.zip
```

Run `ff --help` to see the full list of available commands.
