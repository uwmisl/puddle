
import itertools
import heapq

from typing import Dict, Set, Tuple, List, Any, Optional

from puddle.util import manhattan_distance

import networkx as nx

import logging
log = logging.getLogger(__name__)


Node  = Any
Path  = List[Node]


class RouteFailure(Exception):
    pass


class Agent:

    def __init__(self, item, source, target,
                 collision_group: Optional[int] = None):
        """ An agent to be routed.

        collision_group of None can never collide with anything.
        """

        self.item = item
        self.source = source
        self.target = target
        if collision_group is None:
            # create a guaranteed unique group
            self.collision_group = object()
        else:
            self.collision_group = collision_group


def neighborhood(pos):
    (y,x) = pos

    yield (y+1, x+1)
    yield (y+1, x-1)
    yield (y+1, x  )
    yield (y-1, x+1)
    yield (y-1, x-1)
    yield (y-1, x  )
    yield (y  , x+1)
    yield (y  , x-1)
    yield (y  , x  )


class Router:

    def __init__(self, graph: nx.DiGraph) -> None:
        self.graph = graph

    def route(
            self,
            agents: List[Agent]
    ) -> Dict[Agent, Path]:

        self.avoid = {}
        self.final_places = {}
        paths = {}

        # do the easiest paths first
        agents = sorted(agents,
                        key=lambda a: manhattan_distance(a.source, a.target))

        # do a-star for each one individually, making sure you don't cross any
        # of the previous paths
        for agent in agents:
            log.debug(f'Routing {agent.item}: {agent.source} -> {agent.target}')
            path = self.a_star(agent)
            paths[agent] = path

            # add the 3-dimensional (with time) neighborhood of every step in
            # the path to avoid collisions
            for time, node in enumerate(path):
                for t in (-1, 0, 1):
                    self.avoid.update(((nbr, time + t), agent.collision_group)
                                      for nbr in neighborhood(node))

            # add the end points of the path
            end = path[-1]
            time = len(path)-1
            self.final_places.update((nbr, time)
                                     for nbr in neighborhood(end))

        return paths

    @staticmethod
    def build_path(predecessors: Dict[Node, Node], last) -> Path:
        """Reconstruct a path from the destination and a predecessor map."""
        path = []
        node = last

        while node is not None:
            path.append(node)
            node = predecessors[node]

        path.reverse()
        return path

    def is_legal(self, agent, pos, time):
        g = agent.collision_group

        # if this space is finally occupied, lookup at that last time instead
        time = self.final_places.get(pos, time)

        return self.avoid.get((pos, time), g) == g

    def a_star(self, agent) -> Path:
        # mostly taken from the networkx implementation for now

        pop  = heapq.heappop
        push = heapq.heappush

        # Heap elements are (priority, count, node, distance, time, parent).
        # A counter is to break ties in a stable way.
        count = itertools.count()
        todo = [(0, next(count), agent.source, 0, 0, None)]

        # Maps enqueued nodes to distance of discovered paths and the
        # computed heuristics to target. Saves recomputing heuristics.
        enqueued: Dict[Node, Tuple[int, int]] = {}

        # Maps explored nodes to its predecessor on the shortest path.
        explored: Dict[Node, Node] = {}

        while todo:
            _, _, current, distance, time, parent = pop(todo)

            explored[current] = parent

            if current == agent.target:
                return self.build_path(explored, agent.target)

            for nbr, edge in self.graph[current].items():

                if nbr in explored or not self.is_legal(agent, nbr, time):
                    continue

                nbr_cost = distance + edge.get('weight', 1)

                if nbr in enqueued:
                    q_cost, h = enqueued[nbr]
                    # If q_cost > nbr_cost, we already enqueued a better path
                    # to nbr, so just skip this one and do that one instead.
                    if q_cost <= nbr_cost:
                        continue
                else:
                    h = manhattan_distance(nbr, agent.target)

                enqueued[nbr] = nbr_cost, h
                item = nbr_cost + h, next(count), nbr, nbr_cost, time + 1, current
                push(todo, item)

        raise RouteFailure(f'No path between {agent.source} and {agent.target}')
