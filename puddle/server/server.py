import logging
from pathlib import Path

from flask import Flask, jsonify, request, send_from_directory

app = Flask(__name__)

# suppress debug printing from the web server
# logging.getLogger('werkzeug').setLevel(logging.DEBUG)

# relative where the server is being run from, hopefully the project root
TEST_DIR = Path('tests').resolve(strict=True)
app.config['tests'] = TEST_DIR

session = None


@app.route('/')
def index():
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
        {
            'id': d._id,
            'location': d._location,
            'volume': d._volume,
            'info': d._info,
            "shape": list(d.shape),
        }
        for d in session.arch.droplets
    ]

    session.rendered.set()

    return jsonify(droplets)


@app.route('/shutdown')
def shutdown():
    # from http://flask.pocoo.org/snippets/67/
    func = request.environ.get('werkzeug.server.shutdown')
    if func is None:
        raise RuntimeError('Not running with the Werkzeug Server')
    func()

    return 'Shutdown...'
