from puddle import mk_session, project_path

arch_path = project_path('tests/arches/purpledrop.yaml')
with mk_session(arch_path) as session:

    a = (5, 1)
    b = (5, 6)
    droplet = session.create(location=a, volume=1.0, dimensions=None)
    droplet.move(b)
    droplet.move(a)

    print(session.droplets())
