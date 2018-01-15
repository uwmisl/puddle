import pytest

from puddle.arch import Architecture, CollisionError, Droplet


def test_arch_parse(arch_path):
    """ Test that parsing doesn't crash.

    This test doesn't use the `arch` fixture because it's testing parsing.

    TODO this test could be make much stronger
    """
    assert Architecture.from_file(arch_path)


def test_droplet_copy():
    a = Droplet(_info='a', _location=(1,1))
    a2 = a.copy()
    # copy gets you a fresh collision group
    assert a.collision_group != a2.collision_group


def test_add_droplet(arch01):
    arch = arch01

    a = Droplet(_info='a', _location=(1,1))
    b = Droplet(_info='b', _location=(3,3))
    c = Droplet(_info='c', _location=(3,5))

    # these are overlapping and too close, respectively
    b2 = Droplet(_info='b2', _location=(3,3))
    c2 = Droplet(_info='c2', _location=(4,5))

    # this one should be okay to overlap with b
    b_ok = Droplet(_info='b_ok', _location=(3,3))
    b_ok.collision_group = b.collision_group

    # hack to manually add droplets
    a._state = Droplet._State.REAL
    b._state = Droplet._State.REAL
    c._state = Droplet._State.REAL
    b2._state = Droplet._State.REAL
    c2._state = Droplet._State.REAL
    b_ok._state = Droplet._State.REAL

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


def test_lazy_input(session01):
    s = session01

    a = s.input_droplet(_location=(1, 1))
    b = s.input_droplet()

    assert s.arch.droplets == {a, b}

    with pytest.raises(KeyError):
        s.input_droplet(_location=(-1324, 9999))


def test_mix(session01):
    # Test that mix succeeds as normal
    session = session01

    a = session.input_droplet(_location=(1,1), _info='a')
    b = session.input_droplet(_location=(3,3), _info='b')

    ab = session.mix(a, b)
    session.flush()

    assert len(session.arch.droplets) == 1
    assert ab._info == '(a, b)'


def test_split(session01):

    session = session01

    a = session.input_droplet(_location=(0,0), _info='a')
    session.input_droplet(_location=(3,3), _info='b')

    a1, a2 = session.split(a)
    session.flush()

    assert len(session.arch.droplets) == 3
    assert a1._info == a2._info == 'a'
