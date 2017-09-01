import os
from pathlib import Path
import pytest


program_paths = Path('tests/programs').glob('*.py')


@pytest.mark.parametrize('path', program_paths)
def test_program(path):
    # make sure the test doesn't start the visualization
    os.environ['PUDDLE_VIZ'] = str(0)

    with path.open() as f:
        # empty dicts for globals, locals
        exec(f.read(), {}, {})
    return True
