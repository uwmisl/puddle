import pytest
from puddle import Session, Architecture


def test_input_fail():
    """
    Test to make sure that inputting droplets next to each other fails
    """
    arch = Architecture.from_file('tests/arches/01.arch')

    with Session(arch) as session:
        a = session.input_droplet((3,1))
        with pytest.raises(Exception):
            session.input_droplet((3,2))

        arch.remove_droplet(a)


def test_mix():
    """
    Test that mix succeeds as normal
    """
    arch = Architecture.from_file('tests/arches/01.arch')
    with Session(arch) as session:
        a = session.input_droplet((1,1))
        b = session.input_droplet((3,3))

        session.mix(a, b)


@pytest.mark.skip(reason="split currently does not check for collisions")
def test_split():
    """
    Test that splitting that would result in an overlap throws an
    exception
    """
    arch = Architecture.from_file('tests/arches/01.arch')
    with Session(arch) as session:
        a = session.input_droplet((0,0))
        b = session.input_droplet((3,3))

        with pytest.raises(Exception):
            b1, b2 = session.split(b)

        arch.remove_droplet(a)
        arch.remove_droplet(b)
