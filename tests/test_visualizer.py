from puddle.arch import Architecture, Droplet
from puddle.visualizer import Visualizer


def test_visualizer(interactive=False):
    visualize = Visualizer(interactive)
    arch = Architecture.from_file('tests/arches/01.arch')
    arch.add_droplet(Droplet('0'), (1,1))
    visualize(arch.graph)


if __name__ == '__main__':
    test_visualizer(interactive=True)
    input()
