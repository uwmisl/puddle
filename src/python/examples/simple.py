from puddle import mk_session, Session

# session = Session('http://localhost:3000')
# if True:
with mk_session('../../tests/arches/arch02.json') as session:

    a = session.input(location=None)
    b = session.input(location=None)
    c = session.input(location=None)

    ab = session.mix(a, b)
    ab1, ab2 = session.split(ab)
    abc = session.mix(ab1, c)
    ababc = session.mix(abc, ab2)

    print(session.droplets())
