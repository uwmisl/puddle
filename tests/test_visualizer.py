from puddle.arch import Architecture, Droplet
from puddle.visualizer import Visualizer


def test_visualizer(interactive=False):
    arch = Architecture.from_file('tests/arches/01.arch')
    if interactive:
        arch.pause = 0.8

    visualizer = Visualizer(arch)
    if interactive:
        visualizer.start()

    arch.add_droplet(Droplet('0'), (1,1))

    if interactive:
        visualizer.stop()


if __name__ == '__main__':
    test_visualizer(interactive=True)
