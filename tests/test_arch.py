import pytest

from puddle.arch import Architecture, CollisionError, Droplet


def test_arch_parse(arch_path):
    """ Test that parsing doesn't crash.

    This test doesn't use the `arch` fixture because it's testing parsing.

    TODO this test could be make much stronger
    """
    assert Architecture.from_file(arch_path)


def test_droplet_copy():
    a = Droplet(info='a', location=(1,1))
    a2 = a.copy()
    # copy gets you a fresh collision group
    assert a.collision_group != a2.collision_group


def test_add_droplet(arch01):
    arch = arch01

    a = Droplet(info='a', location=(1,1))
    b = Droplet(info='b', location=(3,3))
    c = Droplet(info='c', location=(3,5))

    # these are overlapping and too close, respectively
    b2 = Droplet(info='b2', location=(3,3))
    c2 = Droplet(info='c2', location=(4,5))

    # this one should be okay to overlap with b
    b_ok = Droplet(info='b_ok', location=(3,3))
    b_ok.collision_group = b.collision_group

    arch.add_droplet(a)
    arch.add_droplet(b)
    arch.add_droplet(c)
    assert len(arch.droplets) == 3

    # these should fail harmlessly
    with pytest.raises(CollisionError):
        arch.add_droplet(b2)
    with pytest.raises(CollisionError):
        arch.add_droplet(c2)

    # caught errors shouldn't modify the droplets set
    assert len(arch.droplets) == 3

    # this one should be added as normal
    arch.add_droplet(b_ok)


def test_mix(session01):
    # Test that mix succeeds as normal
    a = session01.input_droplet(location=(1,1), info='a')
    b = session01.input_droplet(location=(3,3), info='b')

    ab = session01.mix(a, b)
    assert len(session01.arch.droplets) == 1
    assert ab.info == '(a, b)'


def test_split(session01):
    a = session01.input_droplet(location=(0,0), info='a')
    b = session01.input_droplet(location=(3,3), info='b')

    a1, a2 = session01.split(a)
    assert len(session01.arch.droplets) == 3
    assert a1.info == a2.info == 'a'


def test_multi_location_droplet(arch01):
    """ Basic adding and getting of multi-location droplets """
    arch = arch01
    a = Droplet(info='a', location=(0,0), shape=set([(0,0), (1,0)]))
    arch.add_droplet(a)
    assert len(arch.droplets) == 1
    assert a.info == arch.get_droplet((1,0)).info

    # Test invalid shapes
    with pytest.raises(ValueError):
        bad = Droplet(location=(0,0), shape=set([(1,0)]))
    with pytest.raises(ValueError):
        bad = Droplet(location=(0,0), shape=set([(0,0), (2,0), (1,1)]))
    with pytest.raises(ValueError):
        bad = Droplet(location=(0,0), shape=set([(0,0), (0,1),
                                                 (2,0), (2,1)]))

@pytest.mark.xfail(reason="routing multi droplets doesn't work yet")
def test_multi_location_droplet_routing(session01):

    a = session01.input_droplet(location=(1,3), shape=set([(0,0), (1,0)]))
    b = session01.input_droplet(location=(3,3))

    session01.move(a, (5,3))
