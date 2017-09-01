from puddle import Session, Architecture

session = Session(
    Architecture.from_file('tests/arches/01.arch')
)

a = session.input_droplet((1,1))
b = session.input_droplet((3,1))
c = session.input_droplet((4,4))

ab = session.mix(a, b)
ab1, ab2 = session.split(ab)
abc = session.mix(ab1, c)
ababc = session.mix(abc, ab2)
