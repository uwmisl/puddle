from typing import Dict, Any

import networkx as nx

from puddle.arch import Architecture, Command, Move, Mix
from puddle.routing.astar import Router, RouteFailure, Agent
from puddle.util import neighborhood

import logging
log = logging.getLogger(__name__)


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
        self.arch.push_command(command)

        agents = []
        for droplet, input_loc in zip(command.input_droplets,
                                      command.input_locations):
            # only works for single location droplets right now
            (location,) = droplet.locations

            agent = Agent(
                item = droplet,
                source = location,
                target = placement[input_loc]
            )
            agents.append(agent)

            if isinstance(command, Mix):
                # TODO this only works for a single collision group per execution
                agent.collision_group = 1

        # route those droplets who aren't in the command as well
        for droplet in self.arch.droplets:
            if droplet in command.input_droplets:
                continue
            (location,) = droplet.locations

            agent = Agent(
                item = droplet,
                source = location,
                target = location
            )
            agents.append(agent)

        try:
            paths = self.router.route(agents)
        except RouteFailure:
            raise ExcecutionFailure(f'Could not execute {command}')

        # actually route the droplets setting their location
        longest = max(len(path) for path in paths.values())
        for i in range(longest):
            for agent, path in paths.items():
                j = min(len(path)-1, i)
                droplet = agent.item
                droplet.locations = set((path[j],))
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
                for loc2 in neighborhood(loc):
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
