from itertools import product

import networkx as nx


class Execution:

    def __init__(self, arch):
        self.arch = arch
        self.placer = Placer(arch)

    def go(self, command):

        placed_nodes = self.placer.place(command.shape)


class Placer:

    def __init__(self, arch):
        self.arch = arch

    def place(self, module):
        """ Returns a mapping of module nodes onto architecture nodes.

        Also makes sure the "neighborhood" surrounding the module is empty.
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

        matcher = nx.isomorphism.DiGraphMatcher(graph, module)

        # for now, just return the first match because we don't care
        for match in matcher.subgraph_isomorphisms_iter():
            return match

        # couldn't place the rectangle
        return None
