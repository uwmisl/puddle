import numpy as np

# we need to use a virtualenv-friendly backend
import matplotlib
matplotlib.use('TkAgg')

import matplotlib.pyplot as plt
from . import arch

cell_color    = [1.0, 1.0, 1.0]
heater_color  = [0.8, 0.5, 0.2]
droplet_color = [0.6, 0.6, 0.6]


class Visualizer:

    def __init__(self, interactive=True):
        '''Set up a matplotlib figure for future calls to the visualizer.'''
        if interactive:
            plt.ion()
        else:
            plt.ioff()
        self.fig = plt.figure()

    def __call__(self, graph):
        '''Visualize a grid graph of droplets with matplotlib.'''

        # parse graph properties
        rs, cs = zip(*graph.nodes())
        h, w = max(rs) + 1, max(cs) + 1

        # set up axes
        ax = self.fig.gca()
        ax.cla()
        ax.vlines(np.arange(-0.5, w+0.5), -0.5, h+0.5, colors='gray')
        ax.hlines(np.arange(-0.5, h+0.5), -0.5, w+0.5, colors='gray')
        ax.set_xticks(())
        ax.set_yticks(())

        # turn graph into color grid
        grid = np.zeros((h, w, 3))
        for (r, c), cell in graph.nodes(data=True):
            assert isinstance(cell, arch.Cell)
            if isinstance(cell, arch.Heater):
                colors = heater_color
            else:
                colors = cell_color

            if cell.droplet:
                colors = np.mean([colors, droplet_color], 0)

            grid[r,c,:] = colors

        # plot color grid on axes
        ax.imshow(grid)

        # annotate grid with droplet ids
        for (r, c), cell in graph.nodes(data=True):
            if cell.droplet:
                info = cell.droplet.info
                offset = 0.1 * len(info)
                ax.text(c-offset, r+0.1, info, fontsize=16)
