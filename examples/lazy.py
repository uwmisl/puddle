from puddle import Session, Architecture

arch = Architecture.from_file('tests/arches/arch01.yaml')

with Session(arch) as session:

    a = session.input_droplet(info = 'a', location = (1,1))
    b = session.input_droplet(info = 'b', location = (3,1))
    c = session.input_droplet(info = 'c', location = (4,3))

    ab = session.mix(a, b)
    ab1, ab2 = session.split(ab)
    abc = session.mix(ab1, c)
    ababc = session.mix(abc, ab2)

    # Simple REPL setup for seeing laziness... in action?
    #
    # Run through the animation until the commands above finish
    # rendering and you see the ">>>" prompt. Then, use the
    # functions below to further manipulate the droplets.
    #
    # Best viewed with the auto-render box checked.
    #
    # NOTE: CTRL-D to exit the REPL


    #
    # These commands force evaluation of...
    #

    # ...all queued commands.
    def input(a, b):
        return session.input((a, b))

    # ...commands that a and b depend on.
    def force_mix(a, b):
        return session.force_mix(a,b)

    # ...commands that a depends on.
    def force_split(a):
        return session.force_split(a)

    #
    # These commands do not force evaluation.
    #

    def mix(a, b):
        return session.mix(a,b)

    def split(a):
        return session.split(a)


    import code
    code.InteractiveConsole(locals=globals()).interact()