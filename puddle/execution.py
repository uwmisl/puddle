from typing import Dict, Any

import networkx as nx

from puddle.arch import Architecture, Command, Move
from puddle.routing.astar import Router, RouteFailure
from puddle.util import neighborhood

import logging
log = logging.getLogger(__name__)


class ExcecutionFailure(Exception):
    pass


class Execution:

    def __init__(self, arch: Architecture) -> None:
        self.arch = arch
        self.placer = Placer(arch)
        self.router = Router(arch)

    def go(self, command: Command) -> Any:

        log.info(f'Executing {command}')

        # mapping of command nodes onto architecture nodes
        placement = self.placer.place(command)
        self.arch.push_command(command)

        for droplet, input_loc in zip(command.input_droplets,
                                      command.input_locations):
            # only works for single location droplets right now
            droplet.destination = placement[input_loc]

        try:
            paths = self.router.route(self.arch.droplets)
        except RouteFailure:
            raise ExcecutionFailure(f'Could not execute {command}')

        # actually route the droplets setting their location
        longest = max(len(path) for path in paths.values())
        log.info(f"Routing {longest} steps")
        for i in range(longest):
            for droplet, path in paths.items():
                j = min(len(path)-1, i)
                droplet.location = path[j]
            self.arch.wait()

        # execute the command
        result = command.run(placement)
        self.arch.pop_command()

        # clear the destinations, as no one has anywhere to go now
        for d in self.arch.droplets:
            if d.destination:
                assert d.destination == d.location
            d.destination = None

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
            # just return the identity mapping, we are trusting the user here
            return {loc: loc for loc in command.input_locations}

        # NOTE this assumption of only one collision group allows us to place
        # over droplets that are to be used in the reaction
        c_groups = set(d.collision_group for d in command.input_droplets)
        assert len(c_groups) == 1
        (c_group,) = c_groups

        # copy the architecture graph so we can modify it
        graph = nx.DiGraph(self.arch.graph)

        # remove all cells that don't have empty neighborhoods
        # must use a list here because the graph is being modified

        too_close = set()

        for droplet in self.arch.droplets:
            if droplet.collision_group == c_group:
                continue
            for loc2 in neighborhood(droplet.location):
                if loc2 in graph:
                    too_close.add(loc2)

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
