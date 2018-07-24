from pathlib import Path
import pytest

# we have to exclude the interactive examples from testing
example_paths = [
    path for path in
    Path('examples').glob('*.py')
    if 'no test' not in open(path).readline()
]


@pytest.mark.parametrize('example', example_paths)
def test_example(example):
    # this will stall if visualization is enabled
    with example.open() as f:
        # empty dicts for globals, used a initial locals
        exec(f.read(), {})
    return True
