from itertools import tee, cycle, chain


def pairs(iterable):
    """ Get adjacent pairs from an iterable.

    >>> list(pairs([1,2,3,4]))
    [(1, 2), (2, 3), (3, 4)]

    """
    a, b = tee(iterable)
    next(b, None)
    return zip(a, b)

alphanum = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'


def grid_string(*args, height=0, width=0, empty=' '):
    """ Get a string visualization of grid coordinates.

    >>> a = [(1,3), (2,6)]
    >>> b = [(0,4), (2,6), (3,1)]
    >>> print(grid_string((a, '1'), (b, 'x'), empty='-'))
    ----x--
    ---1---
    ------x
    -x-----

    """
    default_chars = chain('.', cycle(alphanum))
    chars = []
    loc_lists = []

    for arg in args:
        if type(arg) is tuple and len(arg) == 2 and type(arg[1]) is str:
            loc_list, char = arg
        else:
            loc_list = arg
            char = next(default_chars)

        loc_lists.append(loc_list)
        chars.append(char)

        ys, xs = zip(*loc_list)
        height = max(height, max(ys) + 1)
        width  = max(width,  max(xs) + 1)

    grid = [ [empty for x in range(width)]
             for y in range(height) ]

    for loc_list, char in zip(loc_lists, chars):
        for y,x in loc_list:
            grid[y][x] = char

    return '\n'.join(''.join(row) for row in grid)
