from puddle import Session, Architecture

arch = Architecture.from_file('tests/arches/arch01.yaml')

with Session(arch) as session:

    a = session.input_droplet(location = (1,1))
    b = session.input_droplet(location = (3,1))
    c = session.input_droplet(location = (4,3))

    ab = session.mix(a, b)
    ab1, ab2 = session.split(ab)
    abc = session.mix(ab1, c)
    ababc = session.mix(abc, ab2)

    session.flush()
