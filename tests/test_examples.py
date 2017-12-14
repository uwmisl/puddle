import os
from pathlib import Path
import pytest


example_paths = Path('examples').glob('*.py')


@pytest.mark.parametrize('example', example_paths)
def test_example(example):
    # make sure the test doesn't start the visualization
    os.environ['PUDDLE_VIZ'] = str(0)

    with example.open() as f:
        # empty dicts for globals, used a initial locals
        exec(f.read(), {})
    return True
