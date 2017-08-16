from puddle.arch import Architecture
import glob


def test_arch():
    tests = glob.glob('tests/arches/*')
    assert len(tests) > 0
    for test in tests:
        with open(test) as f:
            gstr = f.read()

        arch = Architecture.from_file(test)
        assert arch.spec_string() == gstr
