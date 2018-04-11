import requests
import json
import time

from contextlib import contextmanager

from subprocess import Popen, check_output, PIPE, CalledProcessError
import shlex


class Droplet:

    def __init__(self, session, id, i_know_what_im_doing=False):
        if not i_know_what_im_doing:
            raise Exception("You shouldn't be calling this constructor directly")
        self.session = session
        self.valid = True
        self._id = id['id']
        self._process = id['process_id']

    def _new(self, *args, **kwargs):
        return type(self)(self.session, *args, i_know_what_im_doing=True, **kwargs)

    def _use(self):
        if not self.valid:
            raise DropletConsumed('{} already used!'.format(self))
        self.valid = False
        return {'id': self._id, 'process_id': self._process}

    def move(self, loc):
        result_id = self.session._rpc("move", self.session.pid, self._use(), to_location(loc))
        return self._new(result_id)

    def mix(self, other):
        assert isinstance(other, type(self))
        result_id = self.session._rpc("mix", self.session.pid, self._use(), other._use())
        return self._new(result_id)

    def split(self):
        id1, id2 = self.session._rpc("split", self.session.pid, self._use())
        return (self._new(id1), self._new(id2))


def to_location(loc):
    return {'y': loc[0], 'x': loc[1]}


class RPCError(Exception):
    pass


class RequestError(Exception):
    pass


class SessionError(Exception):
    pass


class DropletConsumed(Exception):
    pass


class Session:

    json_headers = {
        'content-type': 'application/json'
    }

    def __init__(self, endpoint, name):
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

        self.pid = self._rpc('new_process', name)

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
        dlist = self._rpc("droplet_info", self.pid)
        return {d['id']['id']: d for d in dlist}

    def _flush(self):
        self._rpc("flush", self.pid)

    def input(self, location, volume, **kwargs):
        droplet_class = kwargs.pop('droplet_class', Droplet)
        result_id = self._rpc("input", self.pid, to_location(location) if location else None, volume)
        return droplet_class(self, result_id, **kwargs, i_know_what_im_doing=True)

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
        split_error = None,
):

    # make sure there aren't any puddle servers running now
    try:
        call('killall puddle-server')
    except CalledProcessError:
        pass

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

    if split_error is not None:
        cmd += ' --split-error-stdev {}'.format(split_error)

    popen = Popen(args=shlex.split(cmd), stdout=PIPE)

    # wait for the server to print 'Listening' so we know it's ready
    line = ''
    while 'Listening' not in line:
        print(line)
        line = popen.stdout.readline() or line
        line = line.decode('utf8')
        time.sleep(0.1)

    session = Session('http://{}:{}'.format(host, port), 'test')
    yield session

    session._flush()
    popen.terminate()
    popen.wait()
