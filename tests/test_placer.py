import networkx as nx

from puddle.arch import Architecture
from puddle.execution import Placer, Command


def test_place():

    arch = Architecture.from_file('tests/arches/01.arch')
    placer = Placer(arch)

    class TestCommand(Command):
        shape = nx.DiGraph(nx.grid_graph([3, 4]))

    command = TestCommand(input_droplets = [])

    placement = placer.place(command)

    assert placement

    # placement maps command to architecture
    command_nodes, arch_nodes = zip(*placement.items())
    assert all(n in command.shape for n in command_nodes)
    assert all(n in arch.graph for n in arch_nodes)

    # make sure the placement is actually isomorphic
    placement_target = arch.graph.subgraph(arch_nodes)
    assert nx.is_isomorphic(placement_target, command.shape)
