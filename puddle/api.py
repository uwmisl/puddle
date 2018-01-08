import itertools
from os import environ
from contextlib import AbstractContextManager
from ast import literal_eval
from typing import Tuple


from puddle.arch import Architecture, Droplet, Mix, Split, Move
from puddle.execution import Execution
from puddle.engine import Engine

import logging
log = logging.getLogger(__name__)


class Session(AbstractContextManager):

    droplet_id_counter = itertools.count()

    def __init__(self, arch: Architecture, visualize=None)  -> None:
        # create graph, connect to hw
        self.arch = arch
        self.arch.session = self
        self.execution = Execution(arch)
        self.engine = Engine(self.execution)

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

    def input_droplet(self, **kwargs) -> Droplet:
        """bind location to new droplet"""

        self.engine.flush()

        d = Droplet(**kwargs)
        self.arch.add_droplet(d)
        return d

    def mix(self, droplet1: Droplet, droplet2: Droplet) -> Droplet:

        mix_cmd = Mix(self.arch, droplet1, droplet2)
        droplet, = self.engine.virtualize(mix_cmd)
        return droplet

    def split(self, droplet: Droplet) -> Tuple[Droplet, Droplet]:

        split_cmd = Split(self.arch, droplet)
        droplet1, droplet2 = self.engine.virtualize(split_cmd)
        return droplet1, droplet2

    def move(self, droplet: Droplet, location: Tuple):

        move_cmd = Move(
            self.arch,
            droplets = [droplet],
            locations = [location]
        )

        self.engine.flush()
        self.engine.realize(move_cmd)

    def heat(self, droplet, temp, time):
        # route droplet to heater
        pass

    def flush(self, droplet=None):
        self.engine.flush(droplet=droplet)
