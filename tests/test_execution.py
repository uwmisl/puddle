from puddle.arch import Architecture, Droplet, Mix
from puddle.execution import Execution


def test_simple_execution(interactive=False):
    arch = Architecture.from_file('tests/arches/01.arch')
    if interactive:
        arch.pause = 0.8

    execution = Execution(arch)

    a = Droplet('a', {(0,0)})
    b = Droplet('b', {(2,0)})

    arch.add_droplet(a)
    arch.add_droplet(b)

    mix = Mix(arch, a, b)
    execution.go(mix)

    assert len(arch.droplets) == 1
    (d,) = arch.droplets

    assert d.info == '(a, b)'


if __name__ == '__main__':
    test_simple_execution(interactive=True)
