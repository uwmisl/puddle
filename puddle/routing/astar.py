
import itertools
import heapq

from typing import Dict, Set, Tuple, List, Any

import networkx as nx

import logging
log = logging.getLogger(__name__)


Node  = Any
Agent = Any
Path  = List[Node]


class Router:

    def __init__(self, graph: nx.DiGraph) -> None:
        self.graph = graph
        self.avoid: Set = set()

    def route(
            self,
            agents: Dict[Agent, Tuple[Node, Node]]
    ) -> Dict[Agent, Path]:

        self.avoid = set()
        paths = {}

        # do a-star for each one individually, making sure you don't cross any
        # of the previous paths
        for a, (src, dst) in agents.items():
            log.debug(f'Routing {a}: {src} -> {dst}')
            path = self.a_star(src, dst)
            paths[a] = path

            # add the neighborhood of every step in the path
            for time, node in enumerate(path):
                self.avoid.add((node, time))
                self.avoid.update((nbr, time)
                                  for nbr in self.graph[node])

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

    def a_star(self, src: Node, dst: Node) -> Path:
        # mostly taken from the networkx implementation for now

        pop  = heapq.heappop
        push = heapq.heappush

        # Heap elements are (priority, count, node, distance, time, parent).
        # A counter is to break ties in a stable way.
        count = itertools.count()
        todo = [(0, next(count), src, 0, 0, None)]

        # Maps enqueued nodes to distance of discovered paths and the
        # computed heuristics to target. Saves recomputing heuristics.
        enqueued: Dict[Node, Tuple[int, int]] = {}

        # Maps explored nodes to its predecessor on the shortest path.
        explored: Dict[Node, Node] = {}

        while todo:
            _, _, current, distance, time, parent = pop(todo)

            explored[current] = parent

            if current == dst:
                return self.build_path(explored, dst)

            for nbr, edge in self.graph[current].items():

                if nbr in explored or (nbr, time) in self.avoid:
                    continue

                nbr_cost = distance + edge.get('weight', 1)

                if nbr in enqueued:
                    q_cost, h = enqueued[nbr]
                    # If q_cost < nbr_cost, we already enqueued a better path
                    # to nbr, so just skip this one and do that one instead.
                    if q_cost <= nbr_cost:
                        continue
                else:
                    h = 0 # FIXME heuristic(neighbor, target)

                enqueued[nbr] = nbr_cost, h
                item = nbr_cost + h, next(count), nbr, nbr_cost, time + 1, current
                push(todo, item)

        raise RuntimeError(f'No path between {src} and {dst}')

    # def dest_nbrs_empty(self, src, dst):
    #     """Default move legality check.

    #     Check to see if the destination's neighborhood is empty, except for the
    #     neighbor `src`.
    #     """

    #     other_nbrs = self.graph.neighbors(dst)
    #     other_nbrs.remove(src)
    #     return all(self.ok[nbr] for nbr in other_nbrs)

    # def shortest_path_len(src, dst):
    #     """A multi-agent heuristic that's just the length of the single-agent shortest path."""

    #     return self.shortest_path_lens[src][dst]
