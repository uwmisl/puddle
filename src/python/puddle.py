import requests
import json


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
        result_id = self._rpc("split", d._use())
        return Droplet(self, result_id, i_know_what_im_doing=True)


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
