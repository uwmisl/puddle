import time
import networkx as nx

from typing import Optional, Tuple, Any, ClassVar, List

from puddle.util import pairs

import logging
log = logging.getLogger(__name__)


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
        log.debug(f'mixing {self} with {other}')
        assert self.valid
        assert other.valid

        # for now, they much be in the same place
        assert self.cell is other.cell
        # FIXME right now we assume cells only have one droplet

        self.valid  = False
        other.valid = False
        info = f'({self.info}, {other.info})'

        result =  Droplet(info = info, cell = self.cell)
        self.cell.droplet = result

        return result


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
            # self.droplet = self.droplet.mix(droplet)
            log.warning(f'overlapping but not mixing '
                        f'{self.droplet} with {droplet} at {self.location}')

        self.droplet = droplet
        droplet.cell = self

    def send(self, other: 'Cell'):
        """ Send contents of self to the other cell. """
        log.debug(f'sending {self.droplet} from {self.location} to {other.location}')

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


class Command:
    shape: ClassVar[nx.DiGraph]
    input_locations: ClassVar[List[Node]]
    result: Any


class Mix(Command):

    shape: ClassVar[nx.DiGraph] = nx.DiGraph(nx.grid_graph([2, 3]))
    input_locations: ClassVar[List[Node]] = [(0,0), (0,0)]

    n_mix_loops = 1
    loop = [(0,0), (1,0), (1,1), (1,2), (0,2), (0,1), (0,0)]

    def __init__(self, arch, droplet1, droplet2):
        self.arch = arch
        self.droplet1 = droplet1
        self.droplet2 = droplet2
        self.input_droplets = [droplet1, droplet2]

    def run(self, mapping):

        # use the mapping to get the edges in the architecture we have to take
        arch_loop_edges = list(pairs(mapping[node] for node in self.loop))

        for _ in range(self.n_mix_loops):
            for edge in arch_loop_edges:
                self.arch.move(edge)

        # location of resulting droplet is set by that of droplet1
        return Droplet.mix(self.droplet1, self.droplet2)


class Split(Command):

    shape: ClassVar[nx.DiGraph] = nx.DiGraph(nx.path_graph(6))
    input_locations: ClassVar[List[Node]] = [2]

    def __init__(self, arch, droplet):
        self.arch = arch
        self.droplet = droplet
        self.input_droplets = [droplet]

    def run(self, mapping):

        # use the mapping to get the edges in the architecture we have to take
        edges1 = list(pairs(mapping[node] for node in range(2, -1, -1)))
        edges2 = list(pairs(mapping[node] for node in range(3,  6,  1)))

        assert len(edges1) == len(edges2)

        # "split" the droplet, and just magically move one to the adjacent cell
        droplet1, droplet2 = self.droplet.split()

        self.arch.graph.node[mapping[2]].add_droplet(droplet1)
        self.arch.graph.node[mapping[3]].add_droplet(droplet2)

        for e1, e2 in zip(edges1, edges2):
            # TODO this should be in parallel
            self.arch.move(e1)
            self.arch.move(e2)

        # make sure the results are where they should be
        assert droplet1.cell == self.arch.graph.node[mapping[0]]
        assert droplet2.cell == self.arch.graph.node[mapping[5]]

        return droplet1, droplet2


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

        # for visualization
        self.active_commands = []

        self.pause = 0

    def push_command(self, command):
        self.active_commands.append(command)

    def pop_command(self):
        self.active_commands.pop()

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
        time.sleep(self.pause)

    def move(self, edge):

        # make sure this is actually an edge in the graph
        (src, dst) = edge
        assert dst in self.graph[src]

        src_cell = self.graph.node[src]
        dst_cell = self.graph.node[dst]

        # make sure that the source cell actually has something
        assert src_cell.droplet

        src_cell.send(dst_cell)
        time.sleep(self.pause)
