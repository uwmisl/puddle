import requests
import json
import time
import os
import sys

from contextlib import contextmanager

from subprocess import Popen, check_output, CalledProcessError
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
        return self._mk_id()

    def _mk_id(self):
        return {'id': self._id, 'process_id': self._process}

    def _renew(self, new_id):
        assert not self.valid
        assert self.session.pid == new_id['process_id']
        self.valid = True
        self._id = new_id['id']

    def move(self, loc):
        result_id = self.session._rpc("move", self.session.pid, self._use(), to_location(loc))
        self._renew(result_id)

    def mix(self, other):
        assert isinstance(other, type(self))
        result_id = self.session._rpc("mix", self.session.pid, self._use(), other._use())
        return self._new(result_id)

    def combine_into(self, other):
        assert isinstance(other, type(self))
        result_id = self.session._rpc("combine_into", self.session.pid, self._use(), other._use())
        return self._new(result_id)

    def split(self):
        id1, id2 = self.session._rpc("split", self.session.pid, self._use())
        return (self._new(id1), self._new(id2))

    def output(self, substance):
        self.session._rpc("output", self.session.pid, substance, self._use())

    def volume(self):
        droplets = self.session.droplets()
        return droplets[self._id]['volume']


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

        max_attempts = 10
        for attempt in range(max_attempts):
            try:
                resp = requests.get(status_check)
                break
            except Exception as exn:
                msg = 'Attempt {}: could not connect to {}'.format(attempt + 1, status_check)
                if attempt == max_attempts - 1:
                    raise RPCError(msg) from exn
                print(msg)
                time.sleep(0.5)

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

    def prelude(self, starting_dict = None):
        if starting_dict is None:
            starting_dict = {}
        else:
            starting_dict = dict(starting_dict)

        starting_dict['mix'] = self.mix
        starting_dict['split'] = self.split
        starting_dict['create'] = self.create
        starting_dict['droplets'] = self.droplets
        starting_dict['_flush'] = self._flush

        return starting_dict

    def droplets(self):
        dlist = self._rpc("droplet_info", self.pid)
        return {d['id']['id']: d for d in dlist}

    def _flush(self):
        self._rpc("flush", self.pid)

    def close(self):
        self._rpc("close_process", self.pid)

    def create(self, location, volume=1.0, dimensions=(1,1), **kwargs):
        droplet_class = kwargs.pop('droplet_class', Droplet)
        result_id = self._rpc("create", self.pid, to_location(location) if location else None, volume, to_location(dimensions) if dimensions else None)
        return droplet_class(self, result_id, **kwargs, i_know_what_im_doing=True)

    def input(self, substance, volume, dimensions, **kwargs):
        result_id = self._rpc("input", self.pid, substance, volume, dimensions)
        return Droplet(self, result_id, **kwargs, i_know_what_im_doing=True)

    def heat(self, droplet, temp, seconds, **kwargs):
        result_id = self._rpc("heat", self.pid, droplet._use(), temp, seconds)
        return Droplet(self, result_id, **kwargs, i_know_what_im_doing=True)

    # just call the droplet methods
    def move (self, droplet, *args, **kwargs): return droplet.move (*args, **kwargs)

    def mix  (self, droplet, *args, **kwargs): return droplet.mix  (*args, **kwargs)

    def combine_into  (self, droplet, *args, **kwargs): return droplet.combine_into  (*args, **kwargs)

    def split(self, droplet, *args, **kwargs): return droplet.split(*args, **kwargs)

    def output (self, substance, droplet, *args, **kwargs): return droplet.output (substance, *args, **kwargs)


def call(cmd):
    args = shlex.split(cmd)
    output = check_output(args)
    return output.decode('utf8').strip()


def project_path(path):
    root = call('git rev-parse --show-toplevel')
    return root + '/' + path


@contextmanager
def mk_session(
        arch_file,
        host = 'localhost',
        port = '3000',
        profile = '--release',
):

    # make sure there aren't any puddle servers running now
    try:
        call('killall puddle-server')
    except CalledProcessError:
        pass

    # this won't build the server, so make sure it's there
    default_command = project_path('/src/core/target/debug/puddle-server')
    command = os.environ.get('PUDDLE_SERVER', default_command)

    # build the server command and run it
    flags = ' --static {static_dir} --host {host} --port {port} {arch_file}'
    cmd = (command + flags).format(
        cargo_toml = project_path('/src/core/Cargo.toml'),
        profile = profile,
        arch_file = arch_file,
        static_dir = project_path('/src/web'),
        host = host,
        port = port,
    )
    print(cmd)

    log_file = open('puddle.log', 'a')
    popen = Popen(args=shlex.split(cmd), stdout=log_file, stderr=sys.stderr)

    session = Session('http://{}:{}'.format(host, port), 'test')
    yield session

    # session._flush()
    session.close()
    popen.terminate()
    popen.wait()
