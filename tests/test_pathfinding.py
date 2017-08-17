import random
import time

import networkx as nx

from puddle.routing.astar import Router

import logging
log = logging.getLogger(__name__)


def random_grid(dim, n_agents, n_obstacles, max_retry = 20):
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

    for _ in range(max_retry):

        grid = nx.grid_graph(dim)

        def dist(a,b):
            (ax,ay), (bx,by) = a, b
            return abs(ax-bx) + abs(ay-by)

        # Try to pick starts, goals. If you can't, just keep trying.
        try:
            starts = []
            for _ in range(n_agents):
                v = random.choice(grid.nodes())
                grid.remove_nodes_from(grid.neighbors(v))
                starts.append(v)

            goals = []
            for i in range(n_agents):

                # Sort by distance from the start, then pick a node some distance away.
                # We assume path lengths are normally distributed.
                nodes = sorted(grid.nodes(),
                               key = (lambda a: dist(a, starts[i])))

                idx = random.gauss(0, len(nodes) * .4)
                idx = min(abs(int(idx)), len(nodes) - 1)
                v = nodes[idx]

                grid.remove_nodes_from(grid.neighbors(v))
                goals.append(v)

                log.debug(f'choosing {starts[i]!s:>10} -> {v!s:>10}')

        except:
            # too many agents, not enough places to put them
            # keep trying until max_retry
            continue

        # restore grid to choose obstacles
        grid = nx.grid_graph(dim)
        grid.remove_nodes_from(starts + goals)
        obstacles = random.sample(grid.nodes(), n_obstacles)

        # restore grid to remove obstacles
        grid = nx.grid_graph(dim)
        grid.remove_nodes_from(obstacles)

        if not all(nx.has_path(grid, s, g) for s,g in zip(starts, goals)):
            # This grid is already impossible
            continue

        return grid, starts, goals

    # failed to find a solution
    log.warning(f'Failed to find a grid with dim={dim}, '
                f'n_agents={n_agents}, n_obstacles={n_obstacles}')
    return None, None, None


def assert_path(graph, path):
    """Makes sure a path is connected and in the graph."""

    for (src, dst) in zip(path, path[1:]):
        assert src in graph
        assert dst in graph[src]

    return True


def random_grid_run(n_iters, *args, **kwargs):
    """Runs a bunch of tests on random_grid's with the given args."""

    for _ in range(n_iters):

        grid, starts, goals = random_grid(*args, **kwargs)

        agents = {i: (s,g) for i, (s,g) in enumerate(zip(starts, goals))}

        router = Router(grid)

        log.info(f'Routing {len(agents)} agents...')
        t0 = time.time()
        agent_paths = router.route(agents)
        t1 = time.time()

        # make sure all the paths make sense
        for a, path in agent_paths.items():
            start, goal = agents[a]

            assert path[0] == start
            assert path[-1] == goal

            assert_path(grid, path)

        log.info(f'Routed in time: {t1-t0}')


def test_route():

    random.seed(1)

    # TODO
    # these tests are especially weak because the router cannot deal with
    # congestion right now
    random_grid_run(10, [ 8,16], n_agents= 1, n_obstacles=10)
    random_grid_run(10, [16,16], n_agents= 5, n_obstacles=10)
    random_grid_run(10, [30,30], n_agents=10, n_obstacles=10)


def profile():
    import cProfile as profile
    fname = 'stats.log'
    profile.run('test_route()', fname)
    import pstats
    p = pstats.Stats(fname).strip_dirs().sort_stats('cumulative')
    p.print_stats(20)
