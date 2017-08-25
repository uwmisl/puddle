from itertools import tee


def pairs(iterable):
    """ Get adjacent pairs from an iterable.

    >>> list(pairs([1,2,3,4]))
    [(1, 2), (2, 3), (3, 4)]

    """
    a, b = tee(iterable)
    next(b, None)
    return zip(a, b)
