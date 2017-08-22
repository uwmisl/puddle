from itertools import product


class Execution:

    def __init__(self):
        pass

    def go(command):

        placer.place(command.shape)


class Placer:

    def __init__(self, arch):
        self.arch = arch

    def place(self, width, height):
        """ A very dumb placement algorithm. """

        # TODO could use isomorphism here
        # FIXME only works for grid graphs

        g = self.arch.graph

        # collect all the places with empty neighborhoods
        open_locations = {
            loc
            for loc, nbrs in g.adjacency_iter()
            if not (g.node[loc].droplet and
                    all(g.node[nbr].droplet for nbr in nbrs))
        }

        # try to find a width * height rectangle
        for (y0, x0) in open_locations:

            candidate_locations = []

            # add all the rest of the things in this rectangle that have open
            # neighborhoods
            for y, x in product(range(height), range(width)):
                loc = (y0 + y, x0 + x)
                if loc in open_locations:
                    candidate_locations.append(loc)
                else:
                    break

            if len(candidate_locations) == width * height:
                return candidate_locations

        # couldn't place the rectangle
        return None
