import networkx as nx
import numpy as np

def parse_arch_file(filename):
    with open(filename) as f:
        string = f.read()

    return parse_arch(string)

def parse_arch(string):
    lines = string.split('\n')

    h = len(lines)
    w = max(len(line) for line in lines)
    lines = [ line + ' ' * (w - len(line)) for line in lines ]

    graph = nx.grid_graph([h,w])
    for r in range(h):
        for c in range(w):
            spec = lines[r][c]
            if spec == ' ':
                graph.remove_node((r,c))
            elif spec == 'I':
                graph.node[r,c] = 'input'
            elif spec == '.':
                graph.node[r,c] = 'normal'
            elif spec == 'H':
                graph.node[r,c] = 'heater'
            else:
                raise ValueError("invalid arch spec character: %s" % spec)

    return graph

def pp_arch(graph):
    nodelist = np.array(graph.nodes())
    h = nodelist[:,0].max()
    w = nodelist[:,1].max()
    lines = [ [' '] * (w+1) for _ in range(h+1) ]

    for (r,c), label in graph.nodes(data = True):
        if label == 'input':
            lines[r][c] = 'I'
        elif label == 'normal':
            lines[r][c] = '.'
        elif label == 'heater':
            lines[r][c] = 'H'
        else:
            raise ValueError("invalid arch spec attribute %s" % label)

    return "\n".join(["".join(line).rstrip() for line in lines]) + "\n"
