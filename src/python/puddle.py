import requests
import json
import time

from contextlib import contextmanager

from subprocess import Popen, check_output, PIPE
import shlex


def to_location(y,x):
    return {'y': y, 'x': x}


class RPCError(Exception):
    pass


class RequestError(Exception):
    pass


class Session:

    json_headers = {
        'content-type': 'application/json'
    }

    def __init__(self, endpoint):
        self.endpoint = endpoint
        self.next_id = 0

        try:
            resp = requests.get(endpoint + '/status')
        except Exception as exn:
            print(exn)
            raise RPCError('could not connect to {}'.format(endpoint)) from exn

        if resp.status_code != 200:
            raise RPCError('Something is wrong with {}: got status code'
                           .format(endpoint, resp.status_code))

    def _rpc(self, method, *args, **kwargs):

        if args and kwargs:
            raise RPCError('Cannot have both args and kwargs')

        request_id = self.next_id
        self.next_id += 1

        data = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": args or kwargs,
        }

        try:
            response = requests.post(
                self.endpoint,
                headers = Session.json_headers,
                data = json.dumps(data),
            )
        except requests.RequestException as exn:
            raise RequestError("Error calling method {}".format(method)) from exn

        if response.status_code != requests.codes.ok:
            raise RequestError("Response {} from server was not OK".format(response.status_code))

        resp_json = response.json()
        assert resp_json['id'] == request_id

        if 'result' in resp_json:
            return resp_json['result']
        else:
            raise SessionError(resp_json['error'])

    def droplets(self):
        return self._rpc("droplets")

    def _flush(self):
        self._rpc("flush")

    def input(self, y, x):
        result_id = self._rpc("input", to_location(y,x))
        return Droplet(self, result_id, i_know_what_im_doing=True)

    def move(self, d, y, x):
        result_id = self._rpc("move", d._use(), to_location(y,x))
        return Droplet(self, result_id, i_know_what_im_doing=True)

    def mix(self, d1, d2):
        assert isinstance(d1, Droplet)
        assert isinstance(d2, Droplet)
        result_id = self._rpc("mix", d1._use(), d2._use())
        return Droplet(self, result_id, i_know_what_im_doing=True)

    def split(self, d):
        assert isinstance(d, Droplet)
        id1, id2 = self._rpc("split", d._use())
        return (Droplet(self, id1, i_know_what_im_doing=True),
                Droplet(self, id2, i_know_what_im_doing=True))


def call(cmd):
    args = shlex.split(cmd)
    output = check_output(args)
    return output.decode('utf8').strip()


@contextmanager
def mk_session(
        arch_file = None,
        host = 'localhost',
        port = '3000',
):

    # paths written in this file should be relative to the project root
    root = call('git rev-parse --show-toplevel')

    arch_file = arch_file or root + '/tests/arches/arch01.json'

    # build the server command and run it
    cmd = 'cargo run --manifest-path {cargo_toml} -- ' \
        '--static {static_dir} --host {host} --port {port} {arch_file}'.format(
            cargo_toml = root + '/src/core/Cargo.toml',
            arch_file = arch_file,
            static_dir = root + '/src/web',
            host = host,
            port = port,
    )
    popen = Popen(args=shlex.split(cmd), stdout=PIPE)

    # wait for the server to print 'Listening' so we know it's ready
    line = ''
    while 'Listening' not in line:
        print(line)
        line = popen.stdout.readline() or line
        line = line.decode('utf8')
        time.sleep(0.1)

    session = Session('http://{}:{}'.format(host, port))
    yield session

    session._flush()
    popen.terminate()
    popen.wait()


class Droplet:

    def __init__(self, session, id, i_know_what_im_doing=False):
        if not i_know_what_im_doing:
            raise Exception("You shouldn't be calling this constructor directly")
        self.session = session
        self.valid = True
        self._id = id

    def _use(self):
        assert self.valid
        self.valid = False
        return self._id

    # just call the session methods
    def move (self, *args, **kwargs): return self.session.move (self, *args, **kwargs)
    def mix  (self, *args, **kwargs): return self.session.mix  (self, *args, **kwargs)
    def split(self, *args, **kwargs): return self.session.split(self, *args, **kwargs)
