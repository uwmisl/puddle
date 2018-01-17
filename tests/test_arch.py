import pytest

from puddle.arch import Architecture, CollisionError, Droplet, Command


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
    assert a._collision_group != a2._collision_group


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
    b_ok._collision_group = b._collision_group

    # hack to manually add droplets
    done_cmd = Command()
    done_cmd.done = True

    a._produced_by(done_cmd)
    b._produced_by(done_cmd)
    c._produced_by(done_cmd)
    b2._produced_by(done_cmd)
    c2._produced_by(done_cmd)
    b_ok._produced_by(done_cmd)

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

    a = s.input_droplet(location=(1, 1))
    b = s.input_droplet()

    assert s.arch.droplets == {a, b}

    with pytest.raises(KeyError):
        s.input_droplet(location=(-1324, 9999))


def test_mix(session01):
    # Test that mix succeeds as normal
    session = session01

    a = session.input_droplet(location=(1,1), info='a')
    b = session.input_droplet(location=(3,3), info='b')

    ab = session.mix(a, b)
    session.flush()

    assert len(session.arch.droplets) == 1
    assert ab._info == '(a, b)'


def test_split(session01):

    session = session01

    a = session.input_droplet(location=(0,0), info='a')
    session.input_droplet(location=(3,3), info='b')

    a1, a2 = session.split(a)
    session.flush()

    assert len(session.arch.droplets) == 3
    assert a1._info == a2._info == 'a'
