# no test
import code
import readline
import atexit
import os

from puddle import mk_session, project_path


# from https://docs.python.org/3.6/library/readline.html#example
class HistoryConsole(code.InteractiveConsole):
    def __init__(self,
                 locals=None,
                 filename="<console>",
                 histfile=os.path.expanduser("~/.console-history")):
        code.InteractiveConsole.__init__(self, locals, filename)
        self.init_history(histfile)

    def init_history(self, histfile):
        readline.parse_and_bind("tab: complete")
        if hasattr(readline, "read_history_file"):
            try:
                readline.read_history_file(histfile)
            except IOError:
                pass
            atexit.register(self.save_history, histfile)

    def save_history(self, histfile):
        readline.set_history_length(1000)
        readline.write_history_file(histfile)


arch_path = project_path('tests/arches/purpledrop.json')
with mk_session(arch_path) as session:

    HistoryConsole(locals=session.prelude()).interact()

    print(session.droplets())
