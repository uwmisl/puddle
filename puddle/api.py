import itertools
from typing import Tuple

from puddle.arch import Architecture, Droplet, Mix, Split
from puddle.execution import Execution
from puddle.visualizer import Visualizer


class Session:

    droplet_id_counter = itertools.count()

    def __init__(self, arch: Architecture, visualize=False)  -> None:
        # create graph, connect to hw
        self.arch = arch
        # self.queue = cmd.CommandQueue()
        self.execution = Execution(arch)
        self.visualizer = Visualizer() if visualize else None

    def input_droplet(self, location, info=None) -> Droplet:
        """bind location to new droplet"""

        info = info or next(self.droplet_id_counter)

        # make sure no droplet at this location already
        assert self.arch.graph.node[location].droplet is None

        droplet = Droplet(info)
        self.arch.add_droplet(droplet, location)

        if self.visualizer:
            self.visualizer(self.arch.graph)
        return droplet

    def mix(self, droplet1: Droplet, droplet2: Droplet) -> Droplet:

        mix_cmd = Mix(self.arch, droplet1, droplet2)
        return self.execution.go(mix_cmd)

    def split(self, droplet: Droplet) -> Tuple[Droplet, Droplet]:

        split_cmd = Split(self.arch, droplet)
        return self.execution.go(split_cmd)

    def heat(self, droplet, temp, time):
        # route droplet to heater
        pass

    def flush(self) -> None:
        raise NotImplementedError('ahhhh')
        # for cmd in self.queue:
        #     self.execution.go(cmd)
