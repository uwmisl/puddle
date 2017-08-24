import networkx as nx

from typing import Optional, Tuple, Any

Node = Any


class Droplet:

    def __init__(self, info='a', cell=None):
        self.info = info
        self.cell = cell
        self.valid = True

    # def __eq__(self, other):
    #     return (self.info == other.info and
    #             self.valid == other.valid)

    def __str__(self):
        invalid_str = '' if self.valid else 'INVALID, '
        return f'Droplet({invalid_str}{self.info!r})'

    def __repr__(self):
        return f'{self} at 0x{id(self):x}'

    def copy(self):
        return self.__class__(self.info, self.cell)

    def split(self, ratio=0.5):
        assert self.valid
        a = self.copy()
        b = self.copy()
        self.valid = False
        return a, b

    def mix(self, other: 'Droplet'):
        assert self.valid
        assert other.valid
        self.valid  = False
        other.valid = False
        info = f'({self.info}, {other.info})'
        return Droplet(info = info, cell = self.cell)


class Cell:

    symbol = '.'

    def __init__(self, location: Tuple[Node, Node]) -> None:

        self.location = location
        self.droplet: Optional[Droplet] = None

    def copy(self):
        # networkx will sometimes call copy on the data objects

        return self.__class__(self.location)

    def add_droplet(self, droplet: Droplet):
        if self.droplet:
            self.droplet = self.droplet.mix(droplet)
        else:
            self.droplet = droplet
            droplet.cell = self

    def send(self, other: 'Cell'):
        """ Send contents of self to the other cell. """

        droplet = self.droplet
        self.droplet = None

        # only works if we had a droplet to begin with
        assert droplet
        other.add_droplet(droplet)


class Heater(Cell):
    symbol = 'H'


class Input(Cell):
    symbol = 'I'


cell_types = (
    Cell,
    Heater,
    Input
)


class Architecture2:

    def __init__(self, topology: nx.DiGraph) -> None:
        pass

    def move(self, edge) -> bool:
        pass

    def split(self, node, edge1, edge2) -> None:
        pass


class Architecture:
    """ An interface to a (maybe) physical board. """

    def __init__(self, graph):

        # only directed, single-edge graphs supported
        if type(graph) is nx.Graph:
            graph = nx.DiGraph(graph)
        assert type(graph) is nx.DiGraph

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

        arch = cls.from_string(string)
        arch.source_file = filename
        return arch

    def spec_string(self):
        """ Return the specification string of this Architecture. """

        lines = [ [' '] * self.width for _ in range(self.height) ]

        for (r,c), cell in self.graph.nodes(data = True):
            lines[r][c] = cell.symbol

        return "\n".join("".join(line).rstrip() for line in lines) + "\n"

    def add_droplet(self, droplet, location):

        # make sure this a space in the graph that's valid but empty
        assert type(location) is tuple and len(location) == 2
        assert location in self.graph.node
        cell = self.graph.node[location]
        assert not cell.droplet

        cell.add_droplet(droplet)

    def move(self, edge):

        # make sure this is actually an edge in the graph
        (src, dst) = edge
        assert dst in self.graph[src]

        src_cell = self.graph.node[src]
        dst_cell = self.graph.node[dst]

        # make sure that the source cell actually has something
        assert src_cell.droplet

        src_cell.send(dst_cell)

    def split(self, location):
        """ Split a droplet into two droplets.

        Requires two spaces on either side of location.

        FIXME Right now this only works horizontally.
        """

        src_cell = self.graph.node[location]
        droplet = src_cell.droplet
        y, x = location

        cell_l1 = self.graph.node[y, x - 1]
        cell_l2 = self.graph.node[y, x - 2]
        cell_r1 = self.graph.node[y, x + 1]
        cell_r2 = self.graph.node[y, x + 2]

        # make sure there's something in the original cell
        # and nothing in the rest of them
        assert droplet
        assert not any((
            cell_l1.droplet,
            cell_l2.droplet,
            cell_r1.droplet,
            cell_r2.droplet
        ))

        cell_l2.drop, cell_r2.drop = droplet.split()
