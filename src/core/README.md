# Puddle Core

## Cross compiling


Check out [this guide][rust-cross] for more information.

Install the right rust toolchain for the Raspberry Pi.
```shell
rustup toolchain add stable-armv7-unknown-linux-gnueabihf
```

Install the corresponding C toolchain so you can get a linker. On Debian (and probably Ubuntu):
```shell
sudo apt install gcc-arm-linux-gnueabihf
```

Add this to `$HOME/.cargo/config`, where the value of `linker` is the linker you just installed.
```toml
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

Now you should be able to go ahead and build!
```shell
cargo build --target armv7-unknown-linux-gnueabihf
```
[rust-cross]: https://github.com/japaric/rust-cross#cross-compiling-with-cargo
