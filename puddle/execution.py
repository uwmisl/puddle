from typing import Dict, Any

import networkx as nx

from puddle.arch import Architecture, Command, Move
from puddle.routing.astar import Router, RouteFailure

import logging
log = logging.getLogger(__name__)

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

        log.info(f'Executing {command}')

        # mapping of command nodes onto architecture nodes
        placement = self.placer.place(command)
        # command.add_placement(placement)
        self.arch.push_command(command)

        goals = {}
        for droplet, input_loc in zip(command.input_droplets,
                                      command.input_locations):
            (location,) = droplet.locations
            goals[droplet] = (location, placement[input_loc])

        try:
            paths = self.router.route(goals)
        except RouteFailure:
            raise ExcecutionFailure(f'Could not execute {command}')

        # actually route the droplets by controlling the architecture
        for droplet, path in paths.items():
            for loc in path:
                droplet.locations = {loc}
                self.arch.wait()

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

        if isinstance(command, Move):
            return {(0,0): command.location}

        # TODO this should allow droplets that are to be used in the reaction

        # copy the architecture graph so we can modify it
        graph = nx.DiGraph(self.arch.graph)

        # remove all cells that don't have empty neighborhoods
        # must use a list here because the graph is being modified

        too_close = set()

        for droplet in self.arch.droplets:
            for loc in droplet.locations:
                too_close.add(loc)
                too_close.update(graph.neighbors_iter(loc))

        graph.remove_nodes_from(too_close)

        # a strict placement doesn't allow bending, so do a dumber placement
        if command.strict:
            for oy,ox in graph:
                # test if all of the command's locations are left in `graph`, which
                # are all OK nodes to place in
                if all((y+oy, x+ox) in graph
                       for y,x in command.shape):
                    d =  {
                        (y,x): (y+oy, x+ox)
                        for y,x in command.shape
                    }
                    return d
        else:
            matcher = nx.isomorphism.DiGraphMatcher(graph, command.shape)

            # for now, just return the first match because we don't care
            for match in matcher.subgraph_isomorphisms_iter():
                # flip the dict so the result maps command nodes to the architecture
                return {cn: an for an, cn in match.items()}

        # couldn't place the command
        raise PlaceError(f'Failed to place {command}')
