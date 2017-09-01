from pathlib import Path
import pytest

from puddle.arch import Architecture


arch_paths = Path('tests/arches').glob('*.arch')


@pytest.mark.parametrize('path', arch_paths)
def test_arch(path):
    with open(path) as f:
        gstr = f.read()

    arch = Architecture.from_file(path)
    assert arch.spec_string().rstrip() == gstr.rstrip()
