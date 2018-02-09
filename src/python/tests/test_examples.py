import os
from pathlib import Path
import pytest


example_paths = Path('examples').glob('*.py')


@pytest.mark.parametrize('example', example_paths)
def test_example(example):
    # this will stall if visualization is enabled
    with example.open() as f:
        # empty dicts for globals, used a initial locals
        exec(f.read(), {})
    return True
