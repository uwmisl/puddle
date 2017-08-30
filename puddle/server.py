import os
from threading import Thread, Event

from flask import Flask, jsonify, send_from_directory
from puddle import Architecture, Session

app = Flask(__name__)

TEST_DIR = os.path.realpath(os.path.join(__file__, '..', '..', 'tests'))
app.config['tests'] = TEST_DIR

session = None


@app.route('/')
def index():

    global session

    def go():

        global session
        session = Session(
            arch = Architecture.from_file(
                'tests/arches/01.arch',
                rendered = Event()
            )
        )

        print(f'Session: {session}')
        a = session.input_droplet((1,1))
        b = session.input_droplet((3,1))
        c = session.input_droplet((4,4))

        ab = session.mix(a, b)
        ab1, ab2 = session.split(ab)
        abc = session.mix(ab1, c)
        ababc = session.mix(abc, ab2)
        print('thread done!', ababc)

    puddle_thread = Thread(target=go)
    puddle_thread.start()

    print('here')

    return send_from_directory('static', 'index.html')


@app.route('/static/<url>')
def static_stuff(url):
    return send_from_directory('static', url)


@app.route('/state')
def state():
    global session

    if not session:
        return jsonify([])

    droplets = [
        cell.droplet.to_dict()
        for _, cell in session.arch.graph.nodes(data=True)
        if cell.droplet
    ]

    session.arch.rendered.set()

    return jsonify(droplets)


@app.route('/arch/<arch_name>')
def arch(arch_name):
    return 'hello'
    # send_from_directory(app.config['tests'], f'arches/{arch_name}')
    # return str(sess.arch).replace('\n', '<br>')
