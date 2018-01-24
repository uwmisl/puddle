
# Contributing

First of all, read the [code of conduct][cc] for this project.
Then, follow the instructions in the [README][readme] to get everything installed.

## Git

### Committing

We (try to) use the [Feature Branch Workflow][feat-branch], so don't push to the
`master` branch unless you know what you're doing.

Do your best to use descriptive branch names, but more importantly, use good
commit messages!
Here's a [good post][commit] to write about commit messages.

### Large Files

We use [Git LFS][lfs] to store larger files. If you want to see those larger
files (videos, presentations, etc.) make sure to install it. Once you have it
installed, you can use Git as you usually would.

For files tracked using Git LFS, note that _any changes_ will result in the
whole file being re-uploaded. So ask before you start tracking any huge files
that will need to be changed.

If you need to commit a video, try to compress it first.
For mostly static videos (like of droplets) try this:
```shell
ffmpeg -i input.mp4 -vcodec libx264 -crf 28 output.mp4
```
A higher `crf` will be more compressed.

## Code Style

Please watch your trailing whitespace! There should basically be no whitespace
at the end of any lines. Look up how to configure your editor to automatically
do this, and you'll make the world a better place.

Aside from that, for Python, [flake8][] and [mypy][] will take care of most code
style things. Make sure to install a flake8 plugin for your favorite text
editor.

[cc]: CODE_OF_CONDUCT.md
[readme]: README.md
[flake8]: http://flake8.pycqa.org/en/latest/
[mypy]: http://mypy-lang.org/
[commit]: https://chris.beams.io/posts/git-commit/
[feat-branch]: https://www.atlassian.com/git/tutorials/comparing-workflows#feature-branch-workflow
[lfs]: https://git-lfs.github.com/
