from typing import Dict, Any

import networkx as nx

from puddle.arch import Architecture, Command
from puddle.routing.astar import Router, RouteFailure
from puddle.util import pairs

# simple type aliases
Node = Any


class ExcecutionFailure(Exception):
    pass


class Execution:

    def __init__(self, arch: Architecture) -> None:
        self.arch = arch
        self.placer = Placer(arch)
        self.router = Router(arch.graph)

    def go(self, command: Command) -> Any:

        # mapping of command nodes onto architecture nodes
        placement = self.placer.place(command)
        # command.add_placement(placement)
        self.arch.push_command(command)

        try:
            paths = self.router.route({
                droplet: (droplet.cell.location, placement[input_loc])
                for droplet, input_loc in zip(command.input_droplets,
                                              command.input_locations)
            })
        except RouteFailure:
            raise ExcecutionFailure(f'Could not execute {command}')

        # actually route the droplets by controlling the architecture
        for droplet, path in paths.items():
            for edge in pairs(path):
                self.arch.move(edge)

        # execute the command
        result = command.run(placement)
        self.arch.pop_command()

        return result


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

        def ok_nbrhood(loc, nbrs):
            locs = [loc] + nbrs
            droplets = (graph.node[l]['cell'].droplet for l in locs)
            return all(
                not droplet or droplet in command.input_droplets
                for droplet in droplets
            )

        graph.remove_nodes_from([
            loc
            for loc, nbrs in graph.adjacency_iter()
            if not ok_nbrhood(loc, list(nbrs.keys()))
        ])

        matcher = nx.isomorphism.DiGraphMatcher(graph, command.shape)

        # for now, just return the first match because we don't care
        for match in matcher.subgraph_isomorphisms_iter():
            # flip the dict so the result maps command nodes to the architecture
            return {cn: an for an, cn in match.items()}

        # couldn't place the command
        raise PlaceError(f'Failed to place {command}')
