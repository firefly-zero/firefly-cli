# firefly-cli

[ [🐙 github](https://github.com/firefly-zero/firefly-cli) ] [ [📦 crates.io](https://crates.io/crates/firefly-cli) ]

Swiss army knife CLI tool for working with [Firefly Zero](https://fireflyzero.com/): build, upload, and publish apps, control device, etc.

## 📥 Installation

* Grab the binary from the latest [release](https://github.com/firefly-zero/firefly-cli/releases) and put it somewhere into `$PATH`.
* Or install using cargo (the [Rust](https://www.rust-lang.org/tools/install) package manager):

    ```bash
    cargo install firefly_cli
    ```

* Or install dev version from the source:

    ```bash
    git clone --depth 1 https://github.com/firefly-zero/firefly-cli.git
    cd firefly-cli
    cargo install --path .
    ```

See the [Installation](https://docs.fireflyzero.com/user/installation/) page in the Firefly Zero docs for a more detailed guide.

## 🔧 Usage

```bash
# build an app and install it into VFS
firefly_cli build

# export an app installed in VFS
firefly_cli export --id sys.input-test

# install an exported app into VFS
firefly_cli import sys.input-test.zip
```

There are more commands. Most of them are covered in the [dev docs](https://docs.fireflyzero.com/dev/). Specifically, in [Getting Started](https://docs.fireflyzero.com/dev/getting-started/) and [Debugging](https://docs.fireflyzero.com/dev/debugging/). Run `firefly_cli --help` to get the full list of available commands.
