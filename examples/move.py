from puddle import Session, Architecture

arch = Architecture.from_file('tests/arches/arch01.yaml')

with Session(arch) as session:

    a = session.input_droplet(location = (1,1))

    target = 4,4

    b = session.move(a, target)
    session.flush()

    print(b.location, target)
    assert b.location == target
