# Puddle Core

## Cross compiling

We use [nix][] for dependency management, including cross compilation.
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


## Using the camera

Make sure to enable then camera using `sudo raspi-config` under "Interfacing Options".

Then make sure the `bcm2835-v4l2` video drivers are loaded.
You can either do this at the command line with `sudo modprobe bcm2835-v4l2`, but you'll have to
do it after every boot.
Instead, you can add the line `bcm2835-v4l2` to `/etc/modules/`.
You'll need to reboot once after you do this.


[nix]: https://nixos.org/
