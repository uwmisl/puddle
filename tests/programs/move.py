from puddle import Session, Architecture

arch = Architecture.from_file('tests/arches/01.arch')

with Session(arch) as session:

    a = session.input_droplet((1,1))

    target = 4,4

    session.move(a, target)

    (loc,) = a.locations
    assert loc == target
