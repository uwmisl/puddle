
import itertools
import heapq
import random

from typing import Dict, Tuple, List

from puddle.arch import Location, Droplet
from puddle.util import manhattan_distance, neighborhood, shape_neighborhood

import networkx as nx

import logging
log = logging.getLogger(__name__)


Path  = List[Location]


class RouteFailure(Exception):
    pass


# TODO make droplets move out of the way sooner rather than later by adding
# distance to others as a secondary minimization goal

class Router:

    graph = nx.DiGraph

    def __init__(self, arch) -> None:
        self.arch = arch

    def route(self, droplets, max_tries=10):

        for i in range(max_tries):
            try:
                shuffle = False if i == 0 else True
                paths = self._route(droplets, shuffle)
                if i > 0:
                    log.warning(f"Took {i} tries to route.")
                return paths
            except RouteFailure as e:
                if i == max_tries - 1:
                    raise e

    def _route(
            self,
            droplets: List[Droplet],
            shuffle: bool
    ) -> Dict[Droplet, Path]:

        self.avoid = {}
        self.final_places = {}
        paths = {}

        def difficulty(a):
            return manhattan_distance(a._location, a._destination)

        # do the easiest paths first, followed by those without a dest
        droplets_with_dest = [d for d in droplets if d._destination]
        droplets_without_dest = [d for d in droplets if not d._destination]
        if shuffle:
            random.shuffle(droplets_without_dest)
            random.shuffle(droplets_with_dest)
        else:
            list.sort(droplets_with_dest, key=difficulty)

        # FIXME we'd like to let unconstrained droplet move anywhere they need
        # to, but we can't do that yet without potentially ruining prior
        # placement of commands.
        for d in droplets_without_dest:
            d._destination = d._location

        droplets = droplets_with_dest + droplets_without_dest

        # do a-star for each one individually, making sure you don't cross any
        # of the previous paths
        goal_time = 0
        for d in droplets:
            log.debug(f'Routing {d}: {d._location} -> {d._destination}')
            path = self.a_star(d, goal_time)
            paths[d] = path

            assert len(path) >= goal_time
            goal_time = max(len(path) - 1, goal_time)

            # add the 3-dimensional (with time) neighborhood of every step in
            # the path to avoid collisions
            for time, node in enumerate(path):
                for t in (-1, 0, 1):
                    # We want to avoid the entire neighborhood of the shape of
                    # each droplet
                    self.avoid.update(
                        ((nbr, time + t), d._collision_group)
                        for nbr in shape_neighborhood(node, d._shape)
                    )

            # add the end points of the path
            end = path[-1]
            time = len(path)-1
            self.final_places.update((nbr, time)
                                     for nbr in neighborhood(end))

        return paths

    @staticmethod
    def build_path(predecessors: Dict[Tuple[Location, int], Location],
                   last, time) -> Path:
        """Reconstruct a path from the destination and a predecessor map."""
        path = []
        node = last

        while node is not None:
            path.append(node)
            node = predecessors[node, time]
            time -= 1

        path.reverse()
        return path

    def is_legal(self, droplet, pos, time):
        g = droplet._collision_group

        # if this space is finally occupied, lookup at that last time instead
        time = self.final_places.get(pos, time)

        return self.avoid.get((pos, time), g) == g

    def a_star(self, droplet, goal_time) -> Path:
        # mostly taken from the networkx implementation for now

        pop  = heapq.heappop
        push = heapq.heappush
        graph = self.arch.graph

        def nbrs(node, time):
            # don't give the option to sit still if we've passed the goal time
            if time <= goal_time:
                yield (node, 0)
            for nbr, edge in graph[node].items():
                yield nbr, edge.get('weight', 1)

        n_popped = 0
        n_nodes = len(graph) * (goal_time + 1)

        # Heap elements are (priority, count, node, distance, time, parent).
        # A counter is to break ties in a stable way.
        count = itertools.count()
        todo = [(0, next(count), droplet._location, 0, 0, None)]

        # Maps enqueued nodes to distance of discovered paths and the
        # computed heuristics to destination. Saves recomputing heuristics.
        enqueued: Dict[Tuple[Location, int], Tuple[int, int]] = {}

        # Maps explored nodes to its predecessor on the shortest path.
        explored: Dict[Tuple[Location, int], Location] = {}

        while todo:
            item = pop(todo)
            _, _, current, distance, time, parent = item
            n_popped += 1

            if (droplet._destination is None or current == droplet._destination) \
               and time >= goal_time:
                # explored[(current, time)] = parent
                log.info("Explored {} of {} nodes ({})"
                         .format(n_popped, n_nodes, n_popped / n_nodes))
                explored[(current, time)] = parent
                return self.build_path(explored, current, time)

            for nbr, nbr_only_cost in nbrs(current, time):
                if (nbr, time) in explored:
                    continue
                if not self.is_legal(droplet, nbr, time):
                    continue

                # if nbr_only_cost == 0:
                #     print("taking 0")
                    # import pdb; pdb.set_trace()
                nbr_cost = distance + nbr_only_cost

                if (nbr, time + 1) in enqueued:
                    q_cost, h = enqueued[(nbr, time + 1)]
                    # If q_cost > nbr_cost, we already enqueued a better path
                    # to nbr, so just skip this one and do that one instead.
                    if q_cost <= nbr_cost:
                        continue
                else:
                    if droplet._destination:
                        h = manhattan_distance(nbr, droplet._destination)
                    else:
                        assert False # FIXME we aren't doing anywhere nodes right now
                        h = 0

                enqueued[(nbr, time + 1)] = nbr_cost, h
                item = nbr_cost + h, next(count), nbr, nbr_cost, time + 1, current
                push(todo, item)

            explored[(current, time)] = parent


        raise RouteFailure(f'No path between {droplet._location} and {droplet._destination}')
