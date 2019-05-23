import puddle

offsets = [(y, x) for y in range(2) for x in range(2)]

corner1 = [(1 + y, 0 + x) for y, x in offsets]
corner2 = [(1 + y, 7 - x) for y, x in offsets]
corner3 = [(12 - y, 7 - x) for y, x in offsets]
corner4 = [(12 - y, 0 + x) for y, x in offsets]

corners = [corner1, corner2, corner3, corner4]


def try_move(d, corner):
    try:
        old_id = d._id
        d.move(corner[0])
        session._flush()
    except puddle.SessionError:
        corner.pop(0)
        d._id = old_id
        d.valid = True
        print("python: ERROR")


arch_path = puddle.project_path('tests/arches/purpledrop.json')


def endurance(session):
    globals().update(session.prelude())

    d = create((6, 6))

    for i in range(100):
        for j, c in enumerate(corners):
            if not c:
                print("python: corners{} is empty!".format(j))
                return

        try_move(d, corner1)
        try_move(d, corner2)
        try_move(d, corner3)
        try_move(d, corner4)

        print("python: Completed cycle {}".format(i))


if __name__ == '__main__':
    # endurance(puddle.Session('http://localhost:3000', 'test'))
    with puddle.mk_session(arch_path) as session:
        endurance(session)
