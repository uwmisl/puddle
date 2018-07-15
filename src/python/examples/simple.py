from puddle import mk_session, project_path

arch_path = project_path('tests/arches/purpledrop.json')
with mk_session(arch_path) as session:

    a = session.create(location=(1,1), volume=1.0, dimensions=None)
    b = session.create(location=(1,4), volume=1.0, dimensions=None)
    c = session.create(location=(1,7), volume=1.0, dimensions=None)

    ab = session.mix(a, b)
    ab1, ab2 = session.split(ab)
    abc = session.mix(ab1, c)
    ababc = session.mix(abc, ab2)

    print(session.droplets())
