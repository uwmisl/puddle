import networkx as nx


class Cell:

    symbol = '.'

    def __init__(self, location):

        # locations are tuples (y,x)
        assert len(location) == 2

        self.location = location


class Heater(Cell):
    symbol = 'H'


class Input(Cell):
    symbol = 'I'


cell_types = (
    Cell,
    Heater,
    Input
)


class Architecture:

    def __init__(self, graph):

        # only bidirectional, single-edge graphs supported
        assert type(graph) is nx.Graph

        # only works for graphs with nodes (y, x)
        assert all(len(n) == 2 for n in graph)

        ys, xs = zip(*graph)

        self.y_min, self.y_max = min(ys), max(ys)
        self.x_min, self.x_max = min(xs), max(xs)

        self.height = self.y_max - self.y_min + 1
        self.width  = self.x_max - self.x_min + 1

        self.graph = graph

    @classmethod
    def from_string(cls, string):
        """ Parse an arch specification string to create an Architecture.

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

        # pad out each line to make a rectangle
        lines = [ line + ' ' * (w - len(line)) for line in lines ]

        graph = nx.grid_graph([h,w])
        for r in range(h):
            for c in range(w):
                sym = lines[r][c]
                loc = (r,c)

                if sym == ' ':
                    graph.remove_node(loc)
                    continue

                try:
                    graph.node[loc] = next(
                        cls(loc)
                        for cls in cell_types
                        if cls.symbol == sym)
                except StopIteration:
                    raise ValueError(f'invalid arch spec character: {sym}')

        return cls(graph)

    @classmethod
    def from_file(cls, filename):
        with open(filename) as f:
            string = f.read()

        return cls.from_string(string)

    def spec_string(self):
        """ Return the specification string of this Architecture. """

        lines = [ [' '] * self.width for _ in range(self.height) ]

        for (r,c), cell in self.graph.nodes(data = True):
            lines[r][c] = cell.symbol

        return "\n".join("".join(line).rstrip() for line in lines) + "\n"
