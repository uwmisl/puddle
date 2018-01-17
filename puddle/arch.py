
from itertools import combinations, count
from enum import Enum

import networkx as nx
import yaml

from attr import dataclass, Factory
from typing import Tuple, Any, ClassVar, List, Dict, Set, Optional

from puddle.util import pairs, manhattan_distance

import logging
log = logging.getLogger(__name__)


Location = Tuple[int, int]


_next_collision_group = count()
_next_droplet_id = count()


class DropletStateError(Exception):
    pass

# disable generation of cmp so it uses id-based hashing
@dataclass(cmp=False)
class Droplet:

    # Describes valid droplet states
    class State(Enum):
        VIRTUAL = 1
        REAL = 2
        VIRTUAL_BOUND = 3
        REAL_BOUND = 4
        CONSUMED = 5

        # Valid Transitions
        #
        # VIRTUAL       -> REAL | VIRTUAL_BOUND
        # REAL          -> REAL_BOUND
        # VIRTUAL_BOUND -> REAL_BOUND | CONSUMED
        # REAL_BOUND    -> CONSUMED
        # CONSUMED      -> x
        #
        # droplets in the first four states can all
        # be soft bound 0 or more times

    _location: Optional[Location] = None
    _info: Any = None
    _volume: float = 1.0

    _state: State = State.VIRTUAL
    _soft_bind_counter: int = 0

    _id: int = Factory(_next_droplet_id.__next__)
    _collision_group: int = Factory(_next_collision_group.__next__)
    _destination: Optional[Location] = None

    # todo(@michalp): rn this is very non-idiomatic. Either remove decorators
    # and keep get_ prefixes or remove prefixes and keep decorators...
    # same for the state helpers below
    @property
    def get_info(self):
        if self._is_virtual:
            raise DropletStateError("Cannot get info of virtual droplet")
        if self._is_consumed:
            raise DropletStateError("Cannot get info of consumed droplet")
        return self._info

    @property
    def get_volume(self):
        if self._is_virtual:
            raise DropletStateError("Cannot get volume of virtual droplet")
        if self._is_consumed:
            raise DropletStateError("Cannot get volume of consumed droplet")
        return self._volume

    @property
    def get_location(self):
        if self._is_virtual:
            raise DropletStateError("Cannot get location of virtual droplet")
        if self._is_consumed:
            raise DropletStateError("Cannot get location of consumed droplet")
        return self._location

    # state helpers

    @property
    def _is_bound(self):
        return self._state == self.State.VIRTUAL_BOUND or \
            self._state == self.State.REAL_BOUND

    @property
    def _is_soft_bound(self):
        return self._soft_bind_counter > 0

    @property
    def _is_virtual(self):
        return self._state == self.State.VIRTUAL or \
            self._state == self.State.VIRTUAL_BOUND

    @property
    def _is_real(self):
        return self._state == self.State.REAL or \
            self._state == self.State.REAL_BOUND

    @property
    def _is_consumed(self):
        return self._state == self.State.CONSUMED

    # transition functions

    def _realize(self):
        # An assert is okay here. This lives outside the user's concern
        # I think the promise is that engine will never make a call
        # that realizes an already realized droplet
        # and the user is prevented from making such a situation occur
        # via binding constraints
        assert self._is_virtual
        self._state = self.State.REAL_BOUND if self._is_bound \
            else self.State.REAL

    def _bind(self):
        # move should be able to double bind
        # so should input and heat...
        # todo(@michalp): this requires engine changes

        # here we except, because we don't want the user to bind bound droplets...
        # command initializers should elevate these errors, and the api should as well.
        if self._is_bound:
            raise DropletStateError("This droplet is already bound to a command!")
        if self._is_consumed:
            raise DropletStateError("This droplet has already been consumed!")
        self._state = self.State.VIRTUAL_BOUND if self._is_virtual \
            else self.State.REAL_BOUND

    def _unbind(self):
        # this is used in mix to unbind the first droplet should
        # binding the second fail. Another way to do this is to move
        # binding checks to commands, but that's more code duplication
        #
        # but it does kind of violate the state machine...
        #
        # asserts okay, this is not user facing
        assert self._is_bound
        self._state = self.State.REAL if self._is_real else self.State.VIRTUAL

    def _soft_bind(self):
        # used by non consuming commands like move and input, checks the same stuff
        # as _bind, but doesn't actually bind the droplet
        assert not self._is_bound
        assert not self._is_consumed
        self._soft_bind_counter += 1

    def _soft_unbind(self):
        # run methods of non-consuming commands should call this
        assert self._is_soft_bound
        assert not self._is_consumed
        self._soft_bind_counter -= 1

    def _consume(self):
        # Asserts are okay here. This lives outside the user's concern
        # I think the promise here is that engine will never make
        # a call that attempts to consume a consumed droplet
        # The user is already prevented from doing this via binding
        # constraints
        assert not self._is_consumed
        assert self._is_bound
        assert not self._is_soft_bound
        assert self._is_real
        self._state = self.State.CONSUMED

    def copy(self, **kwargs):
        return self.__class__(
            info=self._info,
            location=self._location,
            **kwargs
        )

    def split(self, result1, result2, ratio=0.5):

        volume = self._volume / 2

        result1._volume = volume
        result1._info = self._info
        result1._location = self._location

        result2._volume = volume
        result2._info = self._info
        result2._location = self._location

        self._consume()
        result1._realize()
        result2._realize()

        return result1, result2

    def mix(self, other: 'Droplet', result):
        log.debug(f'mixing {self} with {other}')

        # for now, they much be in the same place
        # assert self.cell is other.cell
        # FIXME right now we assume cells only have one droplet

        info = f'({self._info}, {other._info})'

        # TODO this logic definitely won't work when droplets are larger
        # it should give back the "union" of both shapes

        result._info = info
        result._location = self._location
        result._volume = self._volume + other._volume

        self._consume()
        other._consume()
        result._realize()

        return result


@dataclass
class Cell:
    pin: int
    location: Location


class Command:
    shape: ClassVar[nx.DiGraph]
    input_locations: ClassVar[List[Location]]
    input_droplets: List[Droplet]
    output_droplets: List[Droplet]

    strict: ClassVar[bool] = False
    locations_given: ClassVar[bool] = False

    #todo(@michalp): move binding checks to super constructor?

    def run(self, mapping: Dict[Location, Location]):
        for d,l in zip(self.input_droplets, self.input_locations):
            assert d._location == mapping[l]


class Input(Command):

    # TODO mwillsey: make this the shape of the droplet to be inputted
    shape: ClassVar = nx.DiGraph(nx.grid_2d_graph(1, 1))
    locations_given: ClassVar = True
    input_locations: ClassVar = []

    def __init__(self, arch, droplet):

        self.arch = arch
        self.droplet = droplet
        self.input_droplets = []
        self.output_droplets = [droplet]

        loc = self.droplet._location
        if loc and loc not in self.arch.graph:
            raise KeyError("Location {} is not in the architecture".format(loc))

        self.arch.add_droplet(self.droplet)

    def run(self, mapping):
        # this is a bit of a hack to do manual placement here
        if self.droplet._location is None:
            shape = nx.DiGraph()
            shape.add_node((0,0))
            placement = self.arch.session.execution.placer.place_shape(shape)
            self.droplet._location = placement[(0,0)]

        self.droplet._realize()


class Move(Command):

    locations_given: ClassVar = True

    def __init__(self, arch, droplets, locations):

        for d in droplets:
            d._bind()

        self.arch = arch
        self.input_droplets = droplets
        self.input_locations = locations
        self.output_droplets = droplets

    def run(self, mapping):
        for d in self.input_droplets:
            d._unbind()

class Mix(Command):

    shape: ClassVar = nx.DiGraph(nx.grid_2d_graph(2, 3))
    input_locations: ClassVar = [(0,0), (0,0)]

    n_mix_loops = 1
    loop = [(0,0), (1,0), (1,1), (1,2), (0,2), (0,1), (0,0)]

    def __init__(self, arch, droplet1, droplet2):

        droplet1._bind()

        try:
            droplet2._bind()
        except DropletStateError as e:
            droplet1._unbind()
            raise e

        self.arch = arch
        self.droplet1 = droplet1
        self.droplet2 = droplet2
        self.input_droplets = [droplet1, droplet2]
        self.output_droplets = [Droplet(None)]

        # we are going to mix, so set them all to the same collision group.
        collision_group = min(d._collision_group for d in self.input_droplets)
        for d in self.input_droplets:
            d._collision_group = collision_group

    def run(self, mapping):

        super().run(mapping)

        # use the mapping to get the edges in the architecture we have to take
        arch_loop_edges = list(pairs(mapping[node] for node in self.loop))

        assert self.droplet1._location == self.droplet2._location

        self.arch.remove_droplet(self.droplet1)
        self.arch.remove_droplet(self.droplet2)
        result = Droplet.mix(self.droplet1, self.droplet2, *self.output_droplets)
        self.arch.add_droplet(result)

        self.arch.wait()
        for _ in range(self.n_mix_loops):
            for src, dst in arch_loop_edges:
                result._location = dst
                self.arch.wait()

        return result


class Split(Command):

    shape: ClassVar = nx.DiGraph(nx.grid_2d_graph(1,5))
    input_locations: ClassVar = [(0,2)]
    strict: ClassVar = True

    def __init__(self, arch, droplet):

        droplet._bind()

        self.arch = arch
        self.droplet = droplet
        self.input_droplets = [droplet]
        self.output_droplets = [Droplet(None), Droplet(None)]

    def run(self, mapping):

        super().run(mapping)

        # use the mapping to get the edges in the architecture we have to take
        nodes1 = [(0,1), (0,0)]
        nodes2 = [(0,3), (0,4)]

        self.arch.remove_droplet(self.droplet)
        d1, d2 = self.droplet.split(*self.output_droplets)

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

        return d1, d2


class CollisionError(Exception):
    pass


class ArchitectureError(Exception):
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
            if location == droplet._location:
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
        real_droplets = (d for d in self.droplets if d._is_real)
        for d1, d2 in combinations(real_droplets, 2):
            if d1._collision_group != d2._collision_group and \
               manhattan_distance(d1._location, d2._location) <= 1:
                raise CollisionError('Multiple droplets colliding')
                log.debug('colliding')

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
