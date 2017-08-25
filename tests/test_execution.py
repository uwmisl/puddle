from puddle.arch import Architecture, Droplet, Mix
from puddle.execution import Execution
from puddle.visualizer import Visualizer


def test_simple_execution(interactive=False):
    arch = Architecture.from_file('tests/arches/01.arch')
    if interactive:
        arch.pause = 0.8

    execution = Execution(arch)
    visualizer = Visualizer(arch)
    if interactive:
        visualizer.start()

    a = Droplet('a')
    b = Droplet('b')

    locations = {
        a: (1,0),
        b: (3,0),
    }

    for drop, loc in locations.items():
        arch.add_droplet(drop, loc)

    mix = Mix(arch, a, b)
    execution.go(mix)

    resulting_droplets = [
        cell.droplet
        for loc, cell in arch.graph.nodes(data=True)
        if cell.droplet
    ]

    assert len(resulting_droplets) == 1
    d = resulting_droplets[0]

    assert d.info == '(a, b)'

    if interactive:
        visualizer.stop()


if __name__ == '__main__':
    test_simple_execution(interactive=True)
