from typing import Set

import networkx as nx
from attr import dataclass, Factory

from puddle.arch import Command

# NOTE
# commands need inputs and outputs
# edges are immutable droplets


# invariants:
# all droplets are reachable from commands
# all commands are reachable from droplets source commands
# droplet is invalid iff target command is done

# droplet shims should probably support some kind of immutability (maybe)


@dataclass
class CommandQueue:

    commands: Set[Command]
    todo: Set[Command]

    def push(self, command):
        for d in command.input_droplets:
            pass

    def ready_commands(self):
        for cmd in self.todo:
            if all(d.state == READY for d in cmd.input_droplets):
                yield cmd
