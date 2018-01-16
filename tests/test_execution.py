import pytest
import networkx as nx

from puddle.api import Session
from puddle.arch import Architecture, Droplet, Mix, Split
from puddle.execution import Execution, Placer


# NOTE this does a little bit of badness by creating droplets
# where the location doesn't matter. It works fine for now.
@pytest.mark.parametrize('shape', [
    Mix.shape,
    Split.shape,
])
def test_place_command(arch, shape):

    placer = Placer(arch)
    placement = placer.place_shape(shape)

    assert placement

    # placement maps command to architecture
    command_nodes, arch_nodes = zip(*placement.items())
    assert all(n in shape for n in command_nodes)
    assert all(n in arch.graph for n in arch_nodes)

    # make sure the placement is actually isomorphic
    placement_target = arch.graph.subgraph(arch_nodes)
    assert nx.is_isomorphic(placement_target, shape)


def test_simple_execution(arch01):

    arch = arch01
    execution = Execution(arch)

    a = Droplet(info='a', location=(0,0))
    b = Droplet(info='b', location=(2,0))

    arch.add_droplet(a)
    arch.add_droplet(b)

    # hack to manually add droplets
    a._state = Droplet._State.REAL
    b._state = Droplet._State.REAL

    mix = Mix(arch, a, b)
    execution.go(mix)

    assert len(arch.droplets) == 1
    (d,) = arch.droplets

    assert d._info == '(a, b)'


def test_lots_of_movement(session01):

    session = session01
    n = 5

    droplets = [
        session.input_droplet(location=(0, 2*i))
        for i in range(n)
    ]

    for i in range(5):
        # mix all of the droplets
        mega_droplet = droplets[0]
        for d in droplets[1:]:
            mega_droplet = session.mix(mega_droplet, d)
        droplets = [mega_droplet]

        # now split them recursive into 2 ** n_splits droplets
        n_splits = 2
        for _ in range(n_splits):
            old_droplets = droplets
            droplets = []
            for d in old_droplets:
                a,b = session.split(d)
                droplets.append(a)
                droplets.append(b)


@pytest.fixture
def lollipop_board_session():
    arch = Architecture.from_string("""
    board: [
    [_, _, _, a, a, a],
    [a, a, a, a, a, a],
    [_, _, _, a, a, a]
    ]
    """)

    with Session(arch) as s:
        yield s


@pytest.mark.xfail(reason="Not implemented yet.")
def test_same_collision_group_mix(lollipop_board_session):
    s = lollipop_board_session

    a = s.input_droplet(location=(1,0), info='a')
    b = s.input_droplet(location=(1,3), info='b')

    a._collision_group = 1
    b._collision_group = 1

    s.move(a, (1,4))

    assert len(s.arch.droplets) == 1


def test_lazy_mix(session01):
    s = session01

    a = s.input_droplet(location=(1,1), info='a')
    b = s.input_droplet(location=(1,3), info='b')

    s.flush()

    # abc will depend on c twice
    ab = s.mix(a, b)

    # make sure nothing is executed yet
    assert s.arch.droplets == set([a,b])
    s.flush()
    assert s.arch.droplets == set([ab])


def test_lazy_move(session01):
    s = session01
    a = s.input_droplet(location=(1,1), info='a')

    s.move(a, (3,3))

    assert a._location == (1,1)


def test_lazy_mix_consumed(session01):
    s = session01
    a = s.input_droplet(location=(1,1), info='a')
    b = s.input_droplet(location=(1,3), info='b')

    s.mix(a,b)

    # the droplet should be bound at this point
    assert a._is_bound


def test_double_consume(session01):
    s = session01
    a = s.input_droplet(location=(1,1), info='a')
    b = s.input_droplet(location=(1,3), info='b')
    c = s.input_droplet(location=(1,5), info='c')

    s.mix(a,b)

    # TODO make this exception more granular
    # this should fail because b is consumed, and it should fail even if we are
    # using lazy execution
    with pytest.raises(Exception):
        s.mix(b,c)


def test_lazy_double_dependency(session01):
    s = session01

    a = s.input_droplet(location=(1,1), info='a')
    b = s.input_droplet(location=(1,3), info='b')
    c = s.input_droplet(location=(1,5), info='c')

    s.flush()

    c1, c2 = s.split(c)

    ac1 = s.mix(a, c1)
    bc2 = s.mix(b, c2)

    # abc will depend on c twice
    abc = s.mix(ac1, bc2)

    # make sure nothing is executed yet
    assert s.arch.droplets == set([a,b,c])
    s.flush()
    assert s.arch.droplets == set([abc])
