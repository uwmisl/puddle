from typing import Any, Dict

from puddle.arch import Command
from puddle.execution import Execution

class Engine:

    def __init__(self, execution: Execution) -> None:
        self.execution = execution
        self.commands = []

    def virtualize(self, command: Command) -> Any:
        self.commands.append(command)
        return command.output_droplets

    def realize(self, command: Command) -> Any:
        for cmd in self.commands:
            self.execution.go(cmd)
        self.commands = []
        return command.output_droplets

    def flush(self, droplet=None) -> None:
        for cmd in self.commands:
            self.execution.go(cmd)
        self.commands = []
