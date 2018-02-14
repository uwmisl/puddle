import requests
import json
import time

from contextlib import contextmanager

from subprocess import Popen, check_output, PIPE
import shlex


class Droplet:

    def __init__(self, session, id, i_know_what_im_doing=False):
        if not i_know_what_im_doing:
            raise Exception("You shouldn't be calling this constructor directly")
        self.session = session
        self.valid = True
        self._id = id

    def _new(self, *args, **kwargs):
        return type(self)(self.session, *args, i_know_what_im_doing=True, **kwargs)

    def _use(self):
        assert self.valid
        self.valid = False
        return self._id

    def move(self, y, x):
        result_id = self.session._rpc("move", self._use(), to_location(y,x))
        return self.new(result_id)

    def mix(self, other):
        assert isinstance(other, type(self))
        result_id = self.session._rpc("mix", self._use(), other._use())
        return self._new(result_id)

    def split(self):
        id1, id2 = self.session._rpc("split", self._use())
        return (self._new(id1), self._new(id2))


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

        status_check = endpoint + '/status'
        try:
            resp = requests.get(status_check)
        except Exception as exn:
            print(exn)
            raise RPCError('could not connect to {}'.format(status_check)) from exn

        if resp.status_code != 200:
            raise RPCError('Something is wrong with {}: got status code {}'
                           .format(status_check, resp.status_code))

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
                self.endpoint + '/rpc',
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

    def input(self, y, x, droplet_class=Droplet):
        result_id = self._rpc("input", to_location(y,x))
        return droplet_class(self, result_id, i_know_what_im_doing=True)

    # just call the droplet methods
    def move (self, droplet, *args, **kwargs): return droplet.move (*args, **kwargs)
    def mix  (self, droplet, *args, **kwargs): return droplet.mix  (*args, **kwargs)
    def split(self, droplet, *args, **kwargs): return droplet.split(*args, **kwargs)


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
