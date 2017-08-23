from puddle.arch import Architecture, Droplet
from puddle.execution import Execution, Mix


def test_simple_execution():
    arch = Architecture.from_file('tests/arches/01.arch')
    execution = Execution(arch)

    a = Droplet('a')
    b = Droplet('b')

    locations = {
        a: (1,0),
        b: (3,0),
    }

    for drop, loc in locations.items():
        arch.add_droplet(drop, loc)

    mix = Mix([a, b])

    execution.go(mix)

    resulting_droplets = [
        cell.droplet
        for loc, cell in arch.graph.nodes(data=True)
        if cell.droplet
    ]

    assert len(resulting_droplets) == 1
    d = resulting_droplets[0]

    assert d.info == '(a, b)'
