from puddle import mk_session, Session

# session = Session('http://localhost:3000')
# if True:
with mk_session('../../tests/arches/arch01.json') as session:

    a = session.input(1,1)
    b = session.input(3,1)
    c = session.input(4,3)

    ab = session.mix(a, b)
    ab1, ab2 = session.split(ab)
    abc = session.mix(ab1, c)
    ababc = session.mix(abc, ab2)

    print(session.droplets())
