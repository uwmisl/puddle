from itertools import tee


def pairs(iterable):
    """ Get adjacent pairs from an iterable.

    >>> list(pairs([1,2,3,4]))
    [(1, 2), (2, 3), (3, 4)]

    """
    a, b = tee(iterable)
    next(b, None)
    return zip(a, b)


def grid_string(locs):
    locs = set(locs)
    ys, xs = zip(*locs)
    h = max(ys) + 1
    w = max(xs) + 1

    l = '\n'.join(
        ''.join(
            'X' if (y,x) in locs else '.'
            for x in range(w)
        )
        for y in range(h)
    )

    return l
