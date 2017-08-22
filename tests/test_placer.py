import networkx as nx

from puddle.arch import Architecture
from puddle.execution import Placer


def test_place():

    arch = Architecture.from_file('tests/arches/01.arch')
    placer = Placer(arch)

    placed_nodes = placer.place(width = 3, height = 4)
    module = nx.grid_graph([3, 4])

    assert placed_nodes

    placement = arch.graph.subgraph(placed_nodes)

    # need to make both placement and module are undirected graphs
    assert nx.is_isomorphic(nx.Graph(placement), module)
