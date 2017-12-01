import pytest
import networkx as nx

from puddle.arch import Droplet, Mix, Split
from puddle.execution import Execution, Placer


# NOTE this does a little bit of badness by creating droplets without
# locations. It works fine for now.
@pytest.mark.parametrize('command_cls, droplets', [
    (Mix,   [Droplet('a', set()), Droplet('b', set())]),
    (Split, [Droplet('a', set())]),
])
def test_place(arch, command_cls, droplets):

    placer = Placer(arch)

    command = command_cls(arch, *droplets)

    placement = placer.place(command)

    assert placement

    # placement maps command to architecture
    command_nodes, arch_nodes = zip(*placement.items())
    assert all(n in command.shape for n in command_nodes)
    assert all(n in arch.graph for n in arch_nodes)

    # make sure the placement is actually isomorphic
    placement_target = arch.graph.subgraph(arch_nodes)
    assert nx.is_isomorphic(placement_target, command.shape)


def test_simple_execution(arch01, interactive=False):

    arch = arch01
    if interactive:
        arch.pause = 0.8

    execution = Execution(arch)

    a = Droplet('a', {(0,0)})
    b = Droplet('b', {(2,0)})

    arch.add_droplet(a)
    arch.add_droplet(b)

    mix = Mix(arch, a, b)
    execution.go(mix)

    assert len(arch.droplets) == 1
    (d,) = arch.droplets

    assert d.info == '(a, b)'
