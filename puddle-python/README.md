# Puddle Python frontend

This the Python front end for the [Puddle] system.

## Installation

You'll need Python 3.6 for this project.
We use [pipenv] to manage dependencies.
Make sure you have [Git LFS][lfs] installed before cloning.

Once you have Python 3.6 and `pipenv`, do the following to setup the project:
```shell
# clone and enter the repo
git clone git@github.com:uwmisl/puddle.git
cd puddle-python

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

## Running Examples

To run an example program, make sure you install the development dependencies
which include the `puddle` package itself. Then do the following:
```shell
PUDDLE_VIZ=1 python examples/simple.py
```

The environment `PUDDLE_VIZ` controls whether the visualization server runs.
It's off by default.

[pipenv]: https://docs.pipenv.org
[puddle]: http://misl.cs.washington.edu/projects/puddle.html
[lfs]: https://git-lfs.github.com/
