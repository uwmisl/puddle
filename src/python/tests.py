
import puddle
import pytest
import time

from subprocess import Popen, check_output, PIPE
import shlex


def call(cmd):
    args = shlex.split(cmd)
    output = check_output(args)
    return output.decode('utf8').strip()


@pytest.fixture(scope='function')
def session():

    # paths written in this file should be relative to the project root
    root = call('git rev-parse --show-toplevel')
    cargo_toml = root + '/src/core/Cargo.toml'
    arch_file = root + '/tests/arches/arch01.json'

    # build the server command and run it
    cmd = 'cargo run --manifest-path {} {}'.format(cargo_toml, arch_file)
    popen = Popen(args=shlex.split(cmd), stdout=PIPE)

    # wait for the server to print 'Listening' so we know it's ready
    line = ''
    while 'Listening' not in line:
        line = popen.stdout.readline() or line
        line = line.decode('utf8')
        time.sleep(0.1)

    session = puddle.Session('http://localhost:3000')

    yield session

    session._flush()

    popen.terminate()
    popen.wait()


def test_easy(session):
    a = session.input(1,1)
    b = session.input(3,3)
    c = a.mix(b)

    droplets = session.droplets()

    # TODO droplet ids should be strings at some point
    assert set(droplets.keys()) == {str(c._id)}
