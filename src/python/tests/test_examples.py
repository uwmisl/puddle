import unittest
from pathlib import Path

# test module is empty, we dynamically generate these tests


class TestExamples(unittest.TestCase):
    pass


example_paths = [
    path for path in Path('examples').glob('*.py')
    # we have to exclude the interactive examples from testing
    if 'no test' not in open(path).readline()
]


def mk_test(path):
    def example(self):
        with path.open() as f:
            # empty dicts for globals, used a initial locals
            exec(f.read(), {})

    return example


for p in example_paths:
    filename = p.name
    no_extension = filename[:-len(p.suffix)]

    test = mk_test(p)
    test.__name__ = "test_example_" + no_extension
    print("Registering", test.__name__)

    setattr(TestExamples, test.__name__, test)

if __name__ == '__main__':
    unittest.main()
