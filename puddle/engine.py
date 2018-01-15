from typing import Any, Dict

from puddle.arch import Command
from puddle.execution import Execution

# NOTE
# commands need inputs and outputs
# edges are immutable droplets


# invariants:
# all droplets are reachable from commands
# all commands are reachable from droplets source commands
# droplet is invalid iff target command is done

# droplet shims should probably support some kind of immutability (maybe)
# some droplet validity checks should now be done at command reification (maybe)

# todo: an actual dag using nx
# todo: optimizations on said dag

# todo: rewrite to deal with mutliple bindings via soft_bind

class Engine:

    def __init__(self, execution: Execution) -> None:
        self.execution = execution
        # Associates droplet ids to their source commands
        self.dependencies = {}
        self.commands = []

    def virtualize(self, command: Command) -> Any:
        # for droplet in command.output_droplets:
        #     self.dependencies[droplet._id] = command
        self.commands.append(command)

        return command.output_droplets

    def realize(self, command: Command) -> Any:
        for cmd in self.commands:
            self.execution.go(cmd)
        self.commands = []

        # # BFS up the DAG to get a valid execution order.

        # visited = [command]

        # for cmd in visited:
        #     for droplet in cmd.input_droplets:
        #         if droplet._is_virtual and droplet._id in self.dependencies:
        #                 dependency = self.dependencies[droplet._id]
        #                 visited.append(dependency)

        # # make sure visited is unique so we don't do anything twice
        # assert len(set(visited)) == len(visited)

        # for cmd in reversed(visited):
        #     # Only execute commands with non-virtual outputs.
        #     if all(d._is_virtual for d in cmd.output_droplets):
        #         self.execution.go(cmd)

        return command.output_droplets

    def flush(self, droplet=None) -> None:
        for cmd in self.commands:
            self.execution.go(cmd)
        self.commands = []
        # if droplet is None:
        #     # flush all
        #     for droplet_id in self.dependencies:
        #         cmd = self.dependencies[droplet_id]
        #         if all(d._is_virtual for d in cmd.output_droplets):
        #             self.realize(cmd)
        # else:
        #     # only evaluate dependencies for given droplet
        #     cmd = self.dependencies[droplet._id]
        #     if all(d._is_virtual for d in cmd.output_droplets):
        #         self.realize(cmd)
