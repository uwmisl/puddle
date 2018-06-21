# Puddle Core

## Cross compiling


We use [`cross`][] for cross compilation.
`cross` spins up a docker container with the necessary (cross-compiled!)
dependencies and build it in there, dropping the result in the `target` folder
on the host machine just like regular compiling. Here's how to install it and get cross-compiling:

First install Docker however you want, and then install `cross`:
```shell
cargo install cross
```
Now you should be able to go ahead and build! Note that you use `cross` instead of `cargo` when cross compiling, and you still need to put the target flag:
```shell
cross build --target armv7-unknown-linux-gnueabihf
```
[cross]: https://github.com/japaric/cross
