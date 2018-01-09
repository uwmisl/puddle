# Puddle

[![Build Status](https://travis-ci.org/uwmisl/puddle.svg?branch=master)](https://travis-ci.org/uwmisl/puddle)

Puddle is a system for high-level, reliable programming of microfluidic devices.

## Installation

You'll need Python 3.6 for this project. For the visualization code, you'll need
`npm` and [Typescript].

We use [pipenv] to manage dependencies. I'm also a fan of [pipsi] to install
Python scripts. You can [install them both][fancy-pipenv] in a way that doesn't
mess with your Python installation.

We use [Git LFS][lfs] to store larger files. If you want to see those larger
files (videos, presentations, etc.) make sure to install it. Once you have it
installed, you can use Git as you usually would.

Once you have Python 3.6 and `pipenv`, do the following to setup the project:
```shell
# clone and enter the repo
git clone git@github.com:uwmisl/puddle.git
cd puddle

# use pipenv to install requirements into a virtualenv
# --dev gets the development dependencies too
pipenv install --dev

# jump in the virtual environment
pipenv shell

# run all the tests
pytest
```

`pipenv shell` starts a shell in the virtual environment.
You can use `pipenv run ...` instead to run individual commands without being in
the `pipenv shell`.

Now you can install the Javascript/Typescript dependencies with `npm install`.
Since we use TypeScript, you'll need to compile those files to Javascript. From
the repo root, run `tsc -p .` to compile all the Typescript in the project.
Once this is set up, many editors will automatically
Hopefully your editor will keep the Javascript up-to-date as you edit the Typescript.

## Running Examples

To run an example program, make sure you install the development dependencies
which include the `puddle` package itself. Then do the following:
```shell
PUDDLE_VIZ=1 python examples/simple.py
```

The environment `PUDDLE_VIZ` controls whether the visualization server runs.
It's off by default.

## Contributing

Check out the [Code of Conduct][cc] and the [Contributing Guidelines][contrib].

[cc]: CODE_OF_CONDUCT.md
[contrib]: CONTRIBUTING.md
[typescript]: https://www.typescriptlang.org/#download-links
[pipenv]: https://docs.pipenv.org
[pipsi]: https://github.com/mitsuhiko/pipsi
[fancy-pipenv]: https://docs.pipenv.org/install.html#fancy-installation-of-pipenv
[lfs]: https://git-lfs.github.com/
