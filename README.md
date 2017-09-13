# puddle
PurpleDrop Language (PDL)

## Installation

You'll need Python 3 for this project. For the visualization code, you'll need
`npm` and [Typescript].

To download the project and setup the Python, do the following:
```shell
# clone and enter the repo
git clone git@github.com:mwillsey/puddle.git
cd puddle

# create and activate a virtualenv for isolation
virtualenv . --python python3
source bin/activate

# install the python dependencies
pip install -r requirements.txt
```

You can also use [virtualenvwrapper] for the isolation step. In that case, just
run `mkvirtualenv puddle` to make and enter the virtual environment.

Now you can install the Javascript/Typescript dependencies with `npm install`.
Since we use TypeScript, you'll need to compile those files to Javascript. From
the repo root, run `tsc -p .` to compile all the Typescript in the project.
Once this is set up, many editors will automatically
Hopefully your editor will keep the Javascript up-to-date as you edit the Typescript.

## Running Examples

To run an example program, you must point python to the `puddle`
package, and enable the visualization if you want.
```shell
PYTHONPATH=. PUDDLE_VIZ=1 python tests/programs/simple.py
```

## Developing

We (try to) use the [Feature Branch Workflow][feat-branch], so don't push to the
`master` branch unless you know what you're doing.

[feat-branch]: https://www.atlassian.com/git/tutorials/comparing-workflows#feature-branch-workflow
[typescript]: https://www.typescriptlang.org/#download-links
[virtualenvwrapper]: https://virtualenvwrapper.readthedocs.io/en/latest/
