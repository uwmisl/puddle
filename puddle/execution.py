import itertools
from typing import Dict, List, ClassVar, Any

import networkx as nx

from puddle.arch import Architecture, Droplet
from puddle.routing.astar import Router

# simple type aliases
Node = Any


class Command:
    shape: ClassVar[ nx.DiGraph ]
    input_locations: ClassVar[ List[Node] ]

    def __init__(self, input_droplets: List[Droplet]) -> None:
        self.input_droplets = input_droplets

    def run(self):
        # FIXM
        print(f'running {self}')


class Mix(Command):

    shape = nx.DiGraph(nx.grid_graph([2,3]))

    @property
    def input_locations(self):
        return itertools.repeat((0,0), times=len(self.input_droplets))


class Execution:

    def __init__(self, arch: Architecture) -> None:
        self.arch = arch
        self.placer = Placer(arch)
        self.router = Router(arch.graph)

    def go(self, command: Command) -> None:

        # mapping of command nodes onto architecture nodes
        placement = self.placer.place(command)

        paths = self.router.route({
            droplet: (droplet.cell.location, placement[input_loc])
            for droplet, input_loc in zip(command.input_droplets,
                                          command.input_locations)
        })

        # actually route the droplets by controlling the architecture
        for droplet, path in paths.items():
            edges = zip(path, path[1:])
            for edge in edges:
                self.arch.move(edge)

        # execute the command
        command.run()


class PlaceError(Exception):
    pass


class Placer:

    def __init__(self, arch):
        self.arch = arch

    def place(self, command: Command) -> Dict:
        """ Returns a mapping of command nodes onto architecture nodes.

        Also makes sure the "neighborhood" surrounding the command is empty.
        """

        # TODO this should allow droplets that are to be used in the reaction

        # copy the architecture graph so we can modify it
        graph = nx.DiGraph(self.arch.graph)

        # remove all cells that don't have empty neighborhoods
        # must use a list here because the graph is being modified
        graph.remove_nodes_from([
            loc
            for loc, nbrs in graph.adjacency_iter()
            if graph.node[loc].droplet
            or any(graph.node[nbr].droplet for nbr in nbrs)
        ])

        matcher = nx.isomorphism.DiGraphMatcher(graph, command.shape)

        # for now, just return the first match because we don't care
        for match in matcher.subgraph_isomorphisms_iter():
            # flip the dict so the result maps command nodes to the architecture
            return {cn: an for an, cn in match.items()}

        # couldn't place the command
        raise PlaceError(f'Failed to place {command}')
