import invoke
import time

root = invoke.run(
    'git rev-parse --show-toplevel', hide=True).stdout.strip() + '/'


def banner(name):

    columns, rows = invoke.terminals.pty_size()
    w = columns - 2
    print()
    print(f"+{'-' * w}+")
    print(f"|{name.center(w)}|")
    print(f"+{'-' * w}+")
    print()


def time_run(self, cmd, *args, **kwargs):
    start = time.perf_counter()
    self.run(cmd, *args, **kwargs)
    duration = time.perf_counter() - start
    print(f"Command '{cmd}' took {duration:.1f}s")


invoke.Context.time_run = time_run

# load the task modules after providing the common functionality above
# make sure to relative import to not conflict with system
from . import test  # noqa
from . import sync  # noqa

namespace = invoke.Collection(test, sync)
namespace.configure({'run': {'echo': True}})
