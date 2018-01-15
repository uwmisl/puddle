import random
import time

from itertools import islice, combinations, cycle

import pytest
import networkx as nx

from puddle.arch import Architecture, Droplet, Move
from puddle.execution import Execution
from puddle.util import pairs, alphanum

import logging
log = logging.getLogger(__name__)


class RandomGrid:

    class Failure(Exception):
        pass

    def __init__(self, dim, n_agents, n_obstacles, max_retry=20, seed=None):
        """Create a random grid graph and paths across it.

        Args:
            dim: list of grid dimension sizes
            n_agents: the number of paths to create
            n_obstacles: the number of nodes to remove
            max_retry: number of times to attempt generation

        Returns:
        A tuple (grid, starts, goals) where grid is the created graph, and
        starts and goals are lists of the locations for each agent.
        """

        self.dim = dim
        self.n_agents = n_agents
        self.n_obstacles = n_obstacles
        self.seed = seed

        if seed is not None:
            random.seed(seed)

        try:
            gen = islice(self.gen(), max_retry)
            self.grid, self.starts, self.goals = next(gen)
        except StopIteration:
            raise self.Failure(str(self))

    def __str__(self):
        dims = 'x'.join(str(d) for d in self.dim)
        return f'RandomGrid: {dims}, {self.n_agents} agents, '\
            f'{self.n_obstacles} obstacles, seed={self.seed}'

    def __repr__(self):
        return str(self)

    def dist(self, a, b):
        (ax,ay), (bx,by) = a, b
        return abs(ax-bx) + abs(ay-by)

    def gen(self):
        while True:
            try:
                yield self.gen_one()
            except self.Failure:
                pass

    def gen_one(self):

        grid = nx.grid_2d_graph(*self.dim)

        # Try to pick starts, goals. If you can't, just keep trying.
        starts = []
        for _ in range(self.n_agents):
            # make sure to use list on graph functions, because they return iterators
            v = random.choice(list(grid.nodes()))
            grid.remove_nodes_from(list(grid.neighbors(v)))
            starts.append(v)

        goals = []
        for i in range(self.n_agents):

            # Sort by distance from start, then pick a node some distance away.
            # We assume path lengths are normally distributed.
            nodes = sorted(grid.nodes(),
                           key = (lambda a: self.dist(a, starts[i])))

            idx = random.gauss(0, len(nodes) * .4)
            idx = min(abs(int(idx)), len(nodes) - 1)
            v = nodes[idx]

            grid.remove_nodes_from(list(grid.neighbors(v)))
            goals.append(v)

        # restore grid to choose obstacles
        grid = nx.grid_2d_graph(*self.dim)
        grid.remove_nodes_from(starts + goals)
        obstacles = random.sample(grid.nodes(), self.n_obstacles)

        # restore grid to remove obstacles
        grid = nx.grid_2d_graph(*self.dim)
        grid.remove_nodes_from(obstacles)

        if not all(nx.has_path(grid, s, g) for s,g in zip(starts, goals)):
            raise self.Failure

        return grid, starts, goals


random_grids = [
    RandomGrid(( 8,16), n_agents= 1, n_obstacles=10, seed='a'),
    RandomGrid(( 8,16), n_agents= 1, n_obstacles=10, seed='b'),
    RandomGrid(( 8,16), n_agents= 1, n_obstacles=10, seed='c'),

    RandomGrid((16,16), n_agents= 5, n_obstacles=10, seed='d'),
    RandomGrid((16,16), n_agents= 5, n_obstacles=10, seed='e'),
    RandomGrid((16,16), n_agents= 5, n_obstacles=10, seed='f'),

    RandomGrid((30,30), n_agents=10, n_obstacles=10, seed='g'),
]


@pytest.mark.parametrize('grid', random_grids)
def test_random_move(grid):

    arch = Architecture(grid.grid)
    chars = cycle(alphanum)

    droplets = [
        Droplet(_info=char, _location=start)
        for start, char in zip(grid.starts, chars)
    ]

    for d in droplets:
        arch.add_droplet(d)

    ex = Execution(arch)
    move = Move(arch, droplets, grid.goals)

    ex.go(move)
