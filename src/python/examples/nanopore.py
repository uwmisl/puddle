# no test

from puddle import mk_session, project_path

arch_path = project_path('tests/arches/purpledrop-nanopore.json')
with mk_session(arch_path) as session:

    dna = session.create(location=(1, 0), volume=10.0, dimensions=(1, 1))
    session._flush()
    buf = session.input('buffer', volume=350.0, dimensions=(2, 2))
    session._flush()
    mixture = session.mix(dna, buf)
    session._flush()
    session.output('minion', mixture)
    session._flush()
