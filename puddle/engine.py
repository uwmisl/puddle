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

class Engine:

    def __init__(self, execution: Execution) -> None:
        self.execution = execution
        # Associates droplet ids to their source commands
        self.dependencies = {}

    def virtualize(self, command: Command) -> Any:

        for droplet in command.output_droplets:
            self.dependencies[droplet.id] = command

        return command.output_droplets

    def realize(self, command: Command) -> Any:

        # BFS up the DAG to get a valid execution order.

        visited = [command]

        for cmd in visited:
            for droplet in cmd.input_droplets:
                if droplet.virtual and droplet.id in self.dependencies:
                        dependency = self.dependencies[droplet.id]
                        visited.append(dependency)

        for cmd in reversed(visited):
            # Only execute commands with non-virtual outputs.
            if all(d.virtual for d in cmd.output_droplets):
                self.execution.go(cmd)

        return command.output_droplets

    def flush(self) -> None:

        for droplet_id in self.dependencies:
            cmd = self.dependencies[droplet_id]
            if all(d.virtual for d in cmd.output_droplets):
                _  = self.realize(cmd)