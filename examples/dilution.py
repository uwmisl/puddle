from collections import Counter

import networkx as nx

from puddle import Session, Architecture

arch = Architecture.from_file('tests/arches/arch02.yaml')

min_volume = 1.0
max_volume = 4.0

def plan(low, high, target, epsilon=0.01):
    graph = nx.DiGraph()
    mid = (low + high) / 2

    while abs(mid - target) > epsilon:
        graph.add_edge(low, mid)
        graph.add_edge(high, mid)
        if target < mid:
            high = mid
        else:
            low = mid
        mid = (low + high) / 2

    rev_topo = reversed(list(nx.topological_sort(graph)))
    result = next(rev_topo)
    for _, _, data in graph.in_edges(result, data=True):
        data['weight'] = 1

    for node in rev_topo:
        print(node)
        total = sum(w for _,_,w in graph.out_edges(node, data='weight'))
        graph.node[node]['total'] = total
        in_w = (total + 1) // 2
        for _,_,data in graph.in_edges(node, data=True):
            data['weight'] = in_w

    print(list(graph.nodes(data='total'))),
    print(list(graph.edges(data='weight')))

    for node in graph:
        ins = graph.in_edges(node, data='weight')
        outs = graph.out_edges(node, data='weight')
        out_amt = sum(w for _,_,w in outs)
        in_amt = sum(w for _,_,w in ins)
        print(ins, outs, in_amt, out_amt)
        assert not ins or out_amt <= in_amt

    return graph

def test_plan():
    g = plan(0, 1, 0.1, epsilon=.001)

def dilute(session, d_low_factory, d_high_factory, c_target,
           epsilon = 0.001):

    def dilute_rec(d0, d1):
        session.flush()
        con0 = d0.concentration
        con1 = d1.concentration

        assert d0.concentration <= d1.concentration

        # print(len(session.arch.droplets),
        #       d0.concentration, d1.concentration, c_target)

        if abs(d0.concentration - c_target) < epsilon:
            # session.arch.remove_droplet(d1)
            return d0
        if abs(d1.concentration - c_target) < epsilon:
            # session.arch.remove_droplet(d0)
            return d1

        session.flush()

        d = session.mix(d0, d1)

        # FIXME account for volume when picking
        da, db = session.split(d)
        session.flush()
        d_next = da
        # session.arch.remove_droplet(db)
        # print(d_next.concentration)

        if abs(d_next.concentration - c_target) < epsilon:
            return d_next

        if d_next.concentration < c_target:
            d1_again = dilute(session, d_low_factory, d_high_factory,
                              con1, epsilon)
            return dilute_rec(d_next, d1_again)
        else:
            d0_again = dilute(session, d_low_factory, d_high_factory,
                              con0, epsilon)
            return dilute_rec(d0_again, d_next)

    return dilute_rec(d_low_factory(), d_high_factory())


with Session(arch) as session:

    c_low = 0
    c_high = 1

    c_target = .10
    eps = 0.1

    def d_low_factory():
        return session.input_droplet(
            location = (5,5),
            volume = 1,
            concentration = c_low
        )

    def d_high_factory():
        return session.input_droplet(
            location = (3,3),
            volume = 1,
            concentration = c_high
        )

    # FIXME this doesnt work yet
    # d = dilute(session, d_low_factory, d_high_factory,
    #             c_target, epsilon = eps)

    # assert abs(d.concentration - c_target) < eps
