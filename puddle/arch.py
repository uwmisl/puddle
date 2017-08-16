import networkx as nx

def parse_file(filename):
    with open(filename) as f:
        string = f.read()

    return parse(string)

def parse(string):
    """ Parse an arch specification string and return its graph representation.
    Arch specification strings are newline-separated and contain periods (`.`)
    for electrodes, `I` for input electrodes, and `H` for heaters. Spots where
    there are no electrodes are given by a space (` `).
    Example:
     .....
    I......H
     .....
    """

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

def pretty_print(graph):
    """ Do the inverse of `parse`, i.e. take a graph representation
    of an architecture specification and return its string representation.
    """

    h, w = map(max, zip(*graph.nodes()))
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
