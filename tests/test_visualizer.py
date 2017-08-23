from puddle.visualizer import Visualizer
import networkx as nx


def test_visualizer():
    visualize = Visualizer(interactive=False)

    g = nx.grid_graph(dim = [4,8])
    g.node[2,3]['drop_id'] = '1'
    g.node[2,3]['color'] = (0.1, 0.4, 0.3)

    visualize(g)

    g.node[1,3] = g.node[2,3]
    g.node[2,3] = {}

    visualize(g)
