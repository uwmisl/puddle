from pathlib import Path
import pytest

program_path = Path('tests/programs')
programs = program_path.glob('*.py')


@pytest.mark.parametrize('path', programs)
def test_program(path):
    with path.open() as f:
        exec(f.read())
    return True
