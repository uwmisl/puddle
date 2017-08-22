import numpy as np
import matplotlib.pyplot as plt

class Visualizer:

    def __init__(self):
        '''Set up a matplotlib figure for future calls to the visualizer.'''
        plt.ion()
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
        for (r, c), data in graph.nodes(data=True):
            grid[r,c,:] = data.get('color', [1,1,1])

        # plot color grid on axes
        ax.imshow(grid)

        # annotate grid with droplet ids
        for (r, c), data in graph.nodes(data=True):
            drop_id = data.get('drop_id', '')
            offset = 0.1 * len(drop_id)
            ax.text(c-offset, r+0.1, drop_id, fontsize=16)

