# no test

from puddle import mk_session, project_path

# def thermocycle(droplet, temps_and_times):

arch_path = project_path('tests/arches/purpledrop-thermocycle.json')
with mk_session(arch_path) as session:

    # a = session.input('water', volume=15.0, dimensions=(1,1))
    # a.move((2,2))
    # session._flush()
    a = session.create(location=(14, 5), volume=1.0, dimensions=(1, 1))
    for i in range(10):
        print('python loop ', i)

        if i > 0:
            a = session.heat(a, temp=98, seconds=200)
            print('post 98 volume: ', a.volume())

        if a.volume() < 1.0:
            r = session.input('water', volume=15, dimensions=(1, 1))
            session._flush()
            a = session.combine_into(a, r)

        a = session.heat(a, temp=62, seconds=30)
        print('post 62 volume: ', a.volume())
        a = session.heat(a, temp=72, seconds=20)
        print('post 72 volume: ', a.volume())

    # initial_volume = c.volume()

    # droplet.move((2, 6))
    # session._flush()
    # droplet.move((2, 1))
    # session._flush()

    # initial_volume = droplet.volume()

    # print("MY VOLUME", initial_volume)

    # temps_and_times = 1 * [
    #     (95, 20 * seconds),
    #     (68, 30 * seconds),
    # ]
    # for temp, time in temps_and_times:
    #     session.heat(droplet, temp, time)
    #     if initial_volume - droplet.volume > 3:
    #         diff = initial_volume - droplet.volume
    #         droplet = session.combine_into(droplet, input('water', volume = diff))
