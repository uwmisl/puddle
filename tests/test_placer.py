import networkx as nx

from puddle.arch import Architecture, Mix, Droplet
from puddle.execution import Placer


def test_place():

    arch = Architecture.from_file('tests/arches/01.arch')
    placer = Placer(arch)

    a = Droplet('a')
    b = Droplet('b')

    command = Mix(arch, a, b)

    placement = placer.place(command)

    assert placement

    # placement maps command to architecture
    command_nodes, arch_nodes = zip(*placement.items())
    assert all(n in command.shape for n in command_nodes)
    assert all(n in arch.graph for n in arch_nodes)

    # make sure the placement is actually isomorphic
    placement_target = arch.graph.subgraph(arch_nodes)
    assert nx.is_isomorphic(placement_target, command.shape)
