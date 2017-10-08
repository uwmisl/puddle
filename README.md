# puddle
PurpleDrop Language (PDL)

## Installation

You'll need Python 3 for this project. For the visualization code, you'll need
`npm` and [Typescript].

We use [pipenv] to manage dependencies. Do the following to setup the project:
```shell
# clone and enter the repo
git clone git@github.com:mwillsey/puddle.git
cd puddle

# use pipenv to install requirements into a virtualenv
# --dev gets the development dependencies too
pipenv install --dev
```

From there, you can do `pipenv shell` to start a shell in the virtual
environment. You can also just to `pipenv run ...` to run individual commands.

Now you can install the Javascript/Typescript dependencies with `npm install`.
Since we use TypeScript, you'll need to compile those files to Javascript. From
the repo root, run `tsc -p .` to compile all the Typescript in the project.
Once this is set up, many editors will automatically
Hopefully your editor will keep the Javascript up-to-date as you edit the Typescript.

## Running Examples

To run an example program, make sure you install the development dependencies
which include the `puddle` package itself. Then do the following:
```shell
PUDDLE_VIZ=1 python tests/programs/simple.py
```

The environment `PUDDLE_VIZ` controls whether the visualization server runs.

## Developing

We (try to) use the [Feature Branch Workflow][feat-branch], so don't push to the
`master` branch unless you know what you're doing.

[feat-branch]: https://www.atlassian.com/git/tutorials/comparing-workflows#feature-branch-workflow
[typescript]: https://www.typescriptlang.org/#download-links
[pipenv]: https://docs.pipenv.org
