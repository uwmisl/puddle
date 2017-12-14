from collections import Counter

from puddle import Session, Architecture

arch = Architecture.from_file('tests/arches/arch02.yaml')

min_volume = 1.0
max_volume = 4.0


def dilute(session, d_low_factory, d_high_factory, c_target,
           epsilon = 0.001):

    def dilute_rec(d0, d1):
        assert d0.concentration <= d1.concentration

        # print(len(session.arch.droplets),
        #       d0.concentration, d1.concentration, c_target)

        if abs(d0.concentration - c_target) < epsilon:
            session.arch.remove_droplet(d1)
            return d0
        if abs(d1.concentration - c_target) < epsilon:
            session.arch.remove_droplet(d0)
            return d1

        d = session.mix(d0, d1)

        # FIXME account for volume when picking
        da, db = session.split(d)
        d_next = da
        session.arch.remove_droplet(db)
        # print(d_next.concentration)

        if abs(d_next.concentration - c_target) < epsilon:
            return d_next

        if d_next.concentration < c_target:
            d1_again = dilute(session, d_low_factory, d_high_factory,
                              d1.concentration, epsilon)
            return dilute_rec(d_next, d1_again)
        else:
            d0_again = dilute(session, d_low_factory, d_high_factory,
                              d0.concentration, epsilon)
            return dilute_rec(d0_again, d_next)

    return dilute_rec(d_low_factory(), d_high_factory())


with Session(arch) as session:

    c_low = 0
    c_high = 1

    c_target = .10
    eps = 0.001

    def d_low_factory():
        return session.input_droplet(
            location = None,
            volume = 1,
            concentration = c_low
        )

    def d_high_factory():
        return session.input_droplet(
            location = None,
            volume = 1,
            concentration = c_high
        )

    d = dilute(session, d_low_factory, d_high_factory,
                c_target, epsilon = eps)

    assert abs(d.concentration - c_target) < eps
