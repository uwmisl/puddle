
import puddle
import pytest


@pytest.fixture(scope='function')
def session():

    with puddle.mk_session() as sess:
        yield sess


def test_easy(session):
    a = session.input((1,1))
    b = session.input((3,3))
    c = a.mix(b)

    droplets = session.droplets()

    # TODO droplet ids should be strings at some point
    assert set(droplets.keys()) == {c._id}


def test_consumed(session):

    a = session.input((1,1))
    b = session.input((3,3))
    c = a.mix(b)

    with pytest.raises(puddle.DropletConsumed):
        a.mix(b)
