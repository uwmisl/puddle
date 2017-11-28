import pytest

from pathlib import Path

from puddle.arch import Architecture
from puddle.api import Session


arch_paths = Path('tests/arches').glob('*.arch')


@pytest.fixture(scope='function', params=arch_paths)
def arch(request):
    return Architecture.from_file(request.param)


@pytest.fixture(scope='function')
def session(arch):
    s = Session(arch)
    yield s
    s.close()


@pytest.fixture(scope='function')
def arch01():
    """ A hack to get a fixture for a specific architecture. """
    return Architecture.from_file('tests/arches/01.arch')


@pytest.fixture(scope='function')
def session01(arch01):
    """ A hack to get a fixture for a specific session.

    In the current version of pytest, you can't use only some of the
    parameterizations of fixtures.
    See https://github.com/pytest-dev/pytest/issues/652.
    So tests that *must* refer to specific locations can use this instead.
    """
    s = Session(arch01)
    yield s
    s.close()
