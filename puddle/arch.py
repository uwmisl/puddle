
from itertools import combinations, count
from enum import Enum

import networkx as nx
import yaml

from attr import dataclass, Factory, ib
from typing import Tuple, Any, ClassVar, List, Dict, Set, Optional

from puddle.util import pairs, manhattan_distance, shape_neighborhood

import logging
log = logging.getLogger(__name__)


Location = Tuple[int, int]


_next_collision_group = count()
_next_droplet_id = count()



# Shape helpers
_default_shape = lambda: {(0, 0)}

class DropletStateError(Exception):
    pass

# disable generation of cmp so it uses id-based hashing
@dataclass(cmp=False)
class Droplet:

    _location: Optional[Location] = None
    # Require that a new droplet shape must include (0, 0)
    _shape: Set[Location] = ib(default=Factory(_default_shape))
    _info: Any = None
    _volume: float = 1.0
    _concentration: float = 0.0

    _id: int = Factory(_next_droplet_id.__next__)
    _collision_group: int = Factory(_next_collision_group.__next__)
    _destination: Optional[Location] = None

    _producer: Optional["Command"] = None
    _consumer: Optional["Command"] = None

    def _produced_by(self, cmd):
        if self._producer:
            raise DropletStateError
        self._producer = cmd

    def _consumed_by(self, cmd):
        if self._consumer:
            raise DropletStateError
        self._consumer = cmd

    @property
    def _is_virtual(self):
        prod = self._producer
        return not prod or not prod.done

    @property
    def _is_real(self):
        prod = self._producer
        return prod and prod.done and not self._is_consumed

    @property
    def _is_consumed(self):
        cons = self._consumer
        return cons and cons.done

    @_shape.validator
    def _check_shape(self, attr, shape):
        if (0,0) not in shape:
            raise ValueError("shape must contain (0, 0)")
        # shape should be contiguous
        if len(shape) > 1:
            g = nx.Graph()
            g.add_nodes_from(shape)
            g.add_edges_from(
                (o1, o2)
                for o1, o2 in combinations(shape, 2)
                if manhattan_distance(o1, o2) == 1
            )
            if not nx.is_connected(g):
                raise ValueError("shape {} must be contiguous".format(shape))

    def copy(self, **kwargs):
        return self.__class__(
            info=self._info,
            location=self._location,
            shape=self._shape,
            **kwargs
        )

    def mix(self, other: 'Droplet'):
        # TODO: move the shape logic into Mix command
        log.debug(f'mixing {self} with {other}')

        # for now, they must be in the same place; use shape with location
        other_shape = {(offset[0] + other._location[0] - self._location[0],
            offset[1] + other._location[1] - self._location[1]) for offset in other._shape}
        assert self._shape.intersection(other_shape)

        info = f'({self.info}, {other.info})'

        # it should give back the "union" of both shapes
        return Droplet(
            _info = info,
            _location = self._location,
            _shape = self._shape.union(other_shape),
            _volume = self._volume + other._volume
        )

    def locations(self):
        return {} if not self._location else {(self._location[0] + offset[0], self._location[1] + offset[1])
                for offset in self._shape}


@dataclass(cmp=False)
class DropletShim:

    _droplet : Droplet = None

    def __init__(self, droplet: Droplet):
        self._droplet = droplet

    @property
    def info(self):
        if self._droplet._is_virtual:
            raise DropletStateError("Cannot get info of virtual droplet")
        if self._droplet._is_consumed:
            raise DropletStateError("Cannot get info of consumed droplet")
        return self._droplet._info

    @property
    def volume(self):
        if self._droplet._is_virtual:
            raise DropletStateError("Cannot get volume of virtual droplet")
        if self._droplet._is_consumed:
            raise DropletStateError("Cannot get volume of consumed droplet")
        return self._droplet._volume

    @property
    def concentration(self):
        if self._droplet._is_virtual:
            raise DropletStateError("Cannot get concentration of virtual droplet")
        if self._droplet._is_consumed:
            raise DropletStateError("Cannot get concentration of consumed droplet")
        return self._droplet._concentration

    @property
    def location(self):
        if self._droplet._is_virtual:
            raise DropletStateError("Cannot get location of virtual droplet")
        if self._droplet._is_consumed:
            raise DropletStateError("Cannot get location of consumed droplet")
        return self._droplet._location

    @property
    def shape(self):
        if self._droplet._is_virtual:
            raise DropletStateError("Cannot get location of virtual droplet")
        if self._droplet._is_consumed:
            raise DropletStateError("Cannot get location of consumed droplet")
        return self._droplet._shape


@dataclass
class Cell:
    pin: int
    location: Location


class Command:
    shape: ClassVar[nx.DiGraph]
    input_locations: ClassVar[List[Location]]
    input_droplets: List[Droplet]
    output_droplets: List[Droplet]

    done: bool = False

    strict: ClassVar[bool] = False
    locations_given: ClassVar[bool] = False

    # FIXME this isn't running
    def run(self, mapping: Dict[Location, Location]):
        for d,l in zip(self.input_droplets, self.input_locations):
            assert d._location == mapping[l]
        self.done = True


class Input(Command):

    # TODO mwillsey: make this the shape of the droplet to be inputted
    shape: ClassVar = nx.DiGraph(nx.grid_2d_graph(1, 1))
    locations_given: ClassVar = True
    input_locations: ClassVar = []

    def __init__(self, arch, droplet):
        self.arch = arch
        self.input_droplets = [droplet]
        self.output_droplets = [droplet]

        loc = droplet._location
        if loc and loc not in self.arch.graph:
            raise KeyError("Location {} is not in the architecture".format(loc))
        droplet._produced_by(self)

        self.arch.add_droplet(droplet)

    def run(self, mapping):
        # this is a bit of a hack to do manual placement here
        droplet = self.input_droplets[0]

        if droplet._location is None:
            shape = nx.DiGraph()
            shape.add_node((0,0))
            placement = self.arch.session.execution.placer.place_shape(shape)
            droplet._location = placement[(0,0)]

        # do this instead of calling super
        self.done = True
        # return self.output_droplets[0]


class Move(Command):

    locations_given: ClassVar = True

    def __init__(self, arch, droplets, locations):
        self.arch = arch
        self.input_droplets = droplets
        self.input_locations = locations
        self.output_droplets = [d.copy() for d in droplets]

        for d in self.input_droplets:
            d._consumed_by(self)
        for d, loc in zip(self.output_droplets, locations):
            d._produced_by(self)
            d._location = loc


class Mix(Command):

    shape: ClassVar = nx.DiGraph(nx.grid_2d_graph(2, 3))
    input_locations: ClassVar = [(0,0), (0,0)]

    n_mix_loops = 1
    loop = [(0,0), (1,0), (1,1), (1,2), (0,2), (0,1), (0,0)]

    def __init__(self, arch, droplet1, droplet2):
        self.arch = arch
        self.input_droplets = [droplet1, droplet2]
        self.output_droplets = [Droplet(None)]

        # we are going to mix, so set them all to the same collision group.
        collision_group = min(d._collision_group for d in self.input_droplets)
        for d in self.input_droplets:
            d._collision_group = collision_group
            d._consumed_by(self)
        for d in self.output_droplets:
            d._produced_by(self)

    def run(self, mapping):
        super().run(mapping)

        droplet1, droplet2 = self.input_droplets
        result = self.output_droplets[0]

        arch_loop_edges = list(pairs(mapping[node] for node in self.loop))

        assert droplet1._location == droplet2._location

        self.arch.remove_droplet(droplet1)
        self.arch.remove_droplet(droplet2)

        result._info = f'({droplet1._info}, {droplet2._info})'
        result._location = droplet1._location
        result._volume = droplet1._volume + droplet2._volume

        m1 = droplet1._volume * droplet1._concentration
        m2 = droplet2._volume * droplet2._concentration

        result._concentration = (m1 + m2) / result._volume

        self.arch.add_droplet(result)

        self.arch.wait()
        for _ in range(self.n_mix_loops):
            for src, dst in arch_loop_edges:
                result._location = dst
                self.arch.wait()


class Split(Command):

    shape: ClassVar = nx.DiGraph(nx.grid_2d_graph(1,5))
    input_locations: ClassVar = [(0,2)]
    strict: ClassVar = True

    def __init__(self, arch, droplet):
        self.arch = arch
        self.input_droplets = [droplet]
        self.output_droplets = [Droplet(None), Droplet(None)]

        for d in self.input_droplets:
            d._consumed_by(self)
        for d in self.output_droplets:
            d._produced_by(self)

    def run(self, mapping):
        super().run(mapping)

        # use the mapping to get the edges in the architecture we have to take
        nodes1 = [(0,1), (0,0)]
        nodes2 = [(0,3), (0,4)]

        droplet = self.input_droplets[0]
        d1 = self.output_droplets[0]
        d2 = self.output_droplets[1]

        self.arch.remove_droplet(droplet)

        volume = droplet._volume / 2
        for d in self.output_droplets:
            d._volume = volume
            d._concentration = droplet._concentration
            d._info = droplet._info
            d._location = droplet._location

        # allow collisions
        cg2 = d2._collision_group
        d2._collision_group = d1._collision_group

        # For these adds we are okay with adjacent droplets
        self.arch.add_droplet(d1)
        self.arch.add_droplet(d2)
        self.arch.wait()

        for n1, n2 in zip(nodes1, nodes2):
            d1._location = mapping[n1]
            d2._location = mapping[n2]
            self.arch.wait()

        # don't allow collisions
        d2._collision_group = cg2


class ArchitectureError(Exception):
    pass


class CollisionError(ArchitectureError):
    pass


class Architecture:
    """ An interface to a (maybe) physical board. """

    def __init__(self, graph):

        # only directed, single-edge graphs supported
        if type(graph) is nx.Graph:
            graph = nx.DiGraph(graph)
        assert type(graph) is nx.DiGraph

        self.source_file = None

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
        self.session = None

        self.droplets: Set[Droplet] = set()

    def __str__(self):
        return '\n'.join(
            str(cell)
            for cell in self.cells()
            if cell.droplet
        )

    def get_droplet(self, location):
        for droplet in self.droplets:
            if location in droplet.locations():
                return droplet
        return None

    def add_droplet(self, droplet: Droplet):
        if droplet._is_real and droplet._location not in self.graph:
            raise KeyError("Location {} is not in the architecture"
                            .format(droplet._location))

        assert droplet not in self.droplets
        self.droplets.add(droplet)

        # remove the droplet if there was a collision
        try:
            self.check_collisions()
        except CollisionError as e:
            self.droplets.remove(droplet)
            raise e

    def remove_droplet(self, droplet: Droplet):
        assert droplet in self.droplets
        self.droplets.remove(droplet)

    def check_collisions(self):
        """
        Checks for single-cell collisions. Adjacency of cells also counts
        as a collision.
        Throws a CollisionError if there is collision on the board.
        """
        for d1, d2 in combinations(self.droplets, 2):
            # For each pair of droplets, we don't want adjacency, so we use shape_neighborhood
            if (d1._collision_group != d2._collision_group and
                shape_neighborhood(d1._location, d1._shape).intersection(d2.locations())):
                log.debug('colliding')
                raise CollisionError('Multiple droplets colliding')

    def cells(self):
        return (data['cell'] for _, data in self.graph.nodes(data=True))

    def wait(self):

        # print(self.spec_string(with_droplets=True))
        self.check_collisions()

        if self.session and self.session.rendered:
            event = self.session.rendered
            if event:
                event.wait()
                event.clear()

    def push_command(self, command):
        self.active_commands.append(command)

    def pop_command(self):
        self.active_commands.pop()

    @classmethod
    def from_string(cls, string, **kwargs):
        """ Parse an arch specification string to create an Architecture.

        Arch specification strings are newline-separated and contain periods (`.`)
        for electrodes, `I` for input electrodes, and `H` for heaters. Spots where
        there are no electrodes are given by a space (` `).

        Example:
        .....
        I......H
        .....
        """

        data = yaml.load(string)
        board = data['board']

        h = len(board)
        w = max(len(row) for row in board)

        empty_values = ['_', None]

        # cells keyed by id
        cells = {}

        locs_to_add = []

        for y, row in enumerate(board):
            for x, elem in enumerate(row):
                if elem in empty_values:
                    continue

                if type(elem) is int:
                    id = elem
                    if id in cells:
                        raise ArchitectureError("Duplicate ids in arch file")
                    cells[id] = Cell(id, (y,x))

                elif elem == 'a':
                    locs_to_add.append((y,x))

                else:
                    raise ArchitectureError("Unrecognized board element '{}'".format(elem))

        try_id = 0
        for loc in locs_to_add:
            while try_id in cells:
                try_id += 1

            assert try_id not in cells
            cells[try_id] = Cell(try_id, loc)

        # make sure ids are consecutive from 0
        assert set(cells.keys()) == set(range(len(cells)))

        locs = set(c.location for c in cells.values())

        graph = nx.grid_2d_graph(h, w)
        graph.remove_nodes_from([n for n in graph if n not in locs])

        for cell in cells.values():
            graph.node[cell.location]['cell'] = cell

        return cls(graph, **kwargs)

    @classmethod
    def from_file(cls, filename, **kwargs):
        with open(filename) as f:
            string = f.read()

        arch = cls.from_string(string, **kwargs)
        arch.source_file = filename
        return arch

    def to_yaml_string(self, with_droplets=False):
        """ Dump the Architecture to YAML string. """

        lines = [ [' '] * self.width for _ in range(self.height) ]

        for cell in self.cells():
            r,c = cell.location
            if with_droplets and cell.droplet:
                lines[r][c] = 'o'
            else:
                lines[r][c] = cell.symbol

        return "\n".join("".join(line).rstrip() for line in lines) + "\n"
