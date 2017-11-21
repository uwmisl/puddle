from puddle import Session, Architecture

arch = Architecture.from_file('tests/arches/01.arch')
with Session(arch) as session:
    # Test to make sure that inputting droplets next to each other fails
    exception_thrown = False

    a = session.input_droplet((3,1))
    try:
        b = session.input_droplet((3,2))
    except Exception:
        exception_thrown = True

    assert exception_thrown
    arch.remove_droplet(a)

arch = Architecture.from_file('tests/arches/01.arch')
with Session(arch) as session:
    # Test that mix succeeds as normal
    a = session.input_droplet((1,1))
    b = session.input_droplet((3,3))

    ab = session.mix(a, b)
