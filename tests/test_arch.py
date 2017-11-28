import pytest

from puddle.arch import CollisionError


def test_arch_parse(arch):
    """ Test that parsing and printing an arch gets you the same thing back. """
    with open(arch.source_file) as f:
        gstr = f.read()
    assert arch.spec_string().rstrip() == gstr.rstrip()


def test_collision(session01):
    """ Test to make sure that inputting droplets next to each other fails."""

    session01.input_droplet((3,1))
    with pytest.raises(CollisionError):
        session01.input_droplet((3,2))


def test_mix(session01):
    # Test that mix succeeds as normal
    session = session01

    a = session.input_droplet((1,1), info='a')
    b = session.input_droplet((3,3), info='b')

    ab = session.mix(a, b)
    assert ab.info == '(a, b)'


def test_split(session01):

    session = session01

    a = session.input_droplet((0,0))
    b = session.input_droplet((3,3))

    session.split(b)
    assert len(session.arch.droplets) == 3
