import networkx as nx

from puddle.arch import Architecture
from puddle.execution import Placer


def test_place():

    arch = Architecture.from_file('tests/arches/01.arch')
    placer = Placer(arch)

    module = nx.DiGraph(nx.grid_graph([3, 4]))
    placement = placer.place(module)

    assert placement

    # placement maps module to architecture
    placement_target = arch.graph.subgraph(placement.keys())

    # need to make both placement and module are undirected graphs
    assert nx.is_isomorphic(placement_target, module)
