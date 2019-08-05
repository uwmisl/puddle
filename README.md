# Puddle

[![Build Status](https://travis-ci.org/uwmisl/puddle.svg?branch=master)](https://travis-ci.org/uwmisl/puddle)

Puddle is a system for high-level, reliable programming of microfluidic devices.
Check out the [project page] on the [MISL] website.

## Installation

We use [Git LFS][lfs] to store larger files. If you want to see those larger
files (videos, presentations, etc.) make sure to install it. Once you have it
installed, you can use Git as you usually would.

Run the [Makefile](Makefile) to build and test stuff.

The [core][] is written in [Rust][]. Go in there to see how to build it.

## Running Examples

The frontends all have their own examples. Checkout the [Python frontend][py].

## Contributing

Check out the [Code of Conduct][cc] and the [Contributing Guidelines][contrib].

[cc]: CODE_OF_CONDUCT.md
[core]: puddle-core/
[py]: puddle-python/
[contrib]: CONTRIBUTING.md
[lfs]: https://git-lfs.github.com/
[project page]: http://misl.cs.washington.edu/projects/puddle.html
[misl]: http://misl.cs.washington.edu/
[rust]: https://www.rust-lang.org/
