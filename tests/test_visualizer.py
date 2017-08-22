from puddle.visualizer import Visualizer
import networkx as nx

visualize = Visualizer()

g = nx.grid_graph(dim = [4,8])
g.node[2,3]['drop_id'] = '1'
g.node[2,3]['color'] = (0.1, 0.4, 0.3)

visualize(g)
try:
    input()
except OSError:
    pass

g.node[1,3] = g.node[2,3]
g.node[2,3] = {}

visualize(g)
try:
    input()
except OSError:
    pass
