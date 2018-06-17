
import puddle
import pytest


@pytest.fixture(scope='function')
def session():

    arch_path = puddle.project_path('tests/arches/arch01.json')
    with puddle.mk_session(arch_path) as sess:
        yield sess


def test_easy(session):
    a = session.input((1,1), 1.0, (1,1))
    b = session.input(None, 1.0, (1,1))
    c = a.mix(b)

    droplets = session.droplets()

    # TODO droplet ids should be strings at some point
    assert set(droplets.keys()) == {c._id}


def test_consumed(session):

    a = session.input(None, 1.0, (1,1))
    b = session.input(None, 1.0, (1,1))
    c = a.mix(b)
    assert c

    with pytest.raises(puddle.DropletConsumed):
        a.mix(b)


def test_volume(session):

    a = session.input(None, 1.0, (1,1))
    b = session.input(None, 2.0, (1,1))

    ab = session.mix(a, b)

    (a_split, b_split) = session.split(ab)

    assert session.droplets()[a_split._id]['volume'] == 1.5
    assert session.droplets()[b_split._id]['volume'] == 1.5
