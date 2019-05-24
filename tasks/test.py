import invoke
import os

from tasks import root, banner


@invoke.task
def git(c):
    ls = c.run("git ls-files", hide=True).stdout.strip()
    failed = False
    for filename in ls.splitlines():
        kb = os.path.getsize(filename) / 1024
        if kb > 500:
            failed = True
            print(f"{filename} is too big: {kb:.1f}KB")
    if failed:
        print("Git check failed!")
        exit(1)


@invoke.task
def tasks(c):
    with c.cd(root):
        c.run("pyflakes tasks")
        c.run("yapf --recursive --diff tasks")
    print("Tasks checked!")


@invoke.task
def rust(c, release=False):
    banner('rust')
    with c.cd(root + 'src'):
        profile = '--release' if release else ''
        c.time_run(pty=True, cmd=f'cargo build {profile}')
        c.time_run(pty=True, cmd=f'cargo test {profile}')
    print("Rust checked!")


@invoke.task
def python(c):
    banner('python')
    c.run("python --version")
    c.run("yapf --version")
    with c.cd(root + 'src/python'):
        c.run("./setup.py --version")
        c.run("pyflakes puddle")
        c.run("pyflakes tests")
        c.run("pyflakes examples")
        c.time_run("./setup.py test")
        c.run("yapf --recursive --diff .")
    print("Python checked!")


@invoke.task(default=True, pre=[git, tasks, rust, python])
def all(c):
    print("Everything checked!")
