import itertools
from os import environ
from contextlib import AbstractContextManager
from ast import literal_eval
from typing import Tuple


from puddle.arch import Architecture, Droplet, Mix, Split, Move
from puddle.execution import Execution

import logging
log = logging.getLogger(__name__)


class Session(AbstractContextManager):

    droplet_id_counter = itertools.count()

    def __init__(self, arch: Architecture, visualize=None)  -> None:
        # create graph, connect to hw
        self.arch = arch
        self.arch.session = self
        # self.queue = cmd.CommandQueue()
        self.execution = Execution(arch)

        self.rendered = None
        self.server_thread = None

        if visualize is None:
            visualize = bool(literal_eval(environ.get('PUDDLE_VIZ', '0')))

        if visualize:
            log.info('starting server...')
            from threading import Thread, Event
            import puddle.server.server as server

            server.session = self
            self.rendered = Event()

            def go():
                server.app.run()

            self.server_thread = Thread(target=go)
            self.server_thread.start()

            log.info('started server!')

    def close(self):
        log.info('Closing session.')
        if self.server_thread:
            from puddle.server.server import app
            from urllib.request import urlopen
            url = app.config['SERVER_NAME'] or'http://127.0.0.1:5000'
            urlopen(url+'/shutdown')

    def __exit__(self, exc_type, exc_value, traceback):
        # do not suppress exceptions
        self.close()
        return False

    def input_droplet(self, location, info=None) -> Droplet:
        """bind location to new droplet"""

        info = info or next(self.droplet_id_counter)

        # make sure no droplet at this location already

        assert self.arch.get_droplet(location) is None

        droplet = Droplet(info, location)
        self.arch.add_droplet(droplet)

        return droplet

    def mix(self, droplet1: Droplet, droplet2: Droplet) -> Droplet:

        mix_cmd = Mix(self.arch, droplet1, droplet2)
        return self.execution.go(mix_cmd)

    def split(self, droplet: Droplet) -> Tuple[Droplet, Droplet]:

        split_cmd = Split(self.arch, droplet)
        return self.execution.go(split_cmd)

    def move(self, droplet: Droplet, location: Tuple):

        move_cmd = Move(
            self.arch,
            droplets = [droplet],
            locations = [location]
        )
        return self.execution.go(move_cmd)

    def heat(self, droplet, temp, time):
        # route droplet to heater
        pass

    def flush(self) -> None:
        raise NotImplementedError('ahhhh')
        # for cmd in self.queue:
        #     self.execution.go(cmd)
