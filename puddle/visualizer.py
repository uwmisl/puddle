import zmq
import pickle

import numpy as np

# we need to use a virtualenv-friendly backend
import matplotlib
matplotlib.use('TkAgg')

from . import arch as architecture

from multiprocessing import Process
from threading import Thread, Event

cell_color    = [1.0, 1.0, 1.0]
heater_color  = [0.8, 0.5, 0.2]
droplet_color = [0.6, 0.6, 0.6]
command_color = [0.8, 0.2, 0.2]


class Visualizer:

    def __init__(self, arch):
        self.arch = arch
        ctxt = zmq.Context()
        self.sock = ctxt.socket(zmq.REP)
        port = self.sock.bind_to_random_port("tcp://*")

        self.stopped = Event()
        self.thr = Thread(target = self.send_arch)
        self.proc = Process(target=Visualizer.grapher, args=(port,))

    def start(self):
        self.thr.start()
        self.proc.start()

    def stop(self):
        self.stopped.set()
        self.thr.join()

    def send_arch(self):
        while True:
            self.sock.recv()
            if self.stopped.is_set():
                self.sock.send_string("stopped")
                break
            else:
                self.sock.send(pickle.dumps(self.arch))

    @staticmethod
    def grapher(port):
        import matplotlib.pyplot as plt
        ctxt = zmq.Context()
        sock = ctxt.socket(zmq.REQ)
        sock.connect(f"tcp://localhost:{port}")

        fig = plt.figure()
        ax = fig.gca()
        timer = fig.canvas.new_timer(interval = 50)

        def render():

            # get graph
            sock.send_string("request")
            reply = sock.recv()
            if reply == b"stopped":
                timer.stop()
                timer.remove_callback(render)
                return

            arch = pickle.loads(reply)
            graph = arch.graph
            cmds = arch.active_commands

            # parse graph properties
            rs, cs = zip(*graph.nodes())
            h, w = max(rs) + 1, max(cs) + 1

            # set up axes
            ax.cla()
            ax.vlines(np.arange(-0.5, w+0.5), -0.5, h+0.5, colors='gray')
            ax.hlines(np.arange(-0.5, h+0.5), -0.5, w+0.5, colors='gray')
            ax.set_xticks(())
            ax.set_yticks(())

            # turn graph into color grid
            grid = np.zeros((h, w, 3))
            for (r, c), cell in graph.nodes(data=True):
                assert isinstance(cell, architecture.Cell)
                if isinstance(cell, architecture.Heater):
                    colors = heater_color
                else:
                    colors = cell_color

                if cell.droplet:
                    colors = np.mean([colors, droplet_color], 0)

                grid[r,c,:] = colors

            # plot functional units
            for cmd in cmds:
                for (r,c) in cmd.placement.values():
                    grid[r,c,:] += command_color
                    grid[r,c,:] /= 2
            # plot color grid on axes
            ax.imshow(grid)

            # annotate grid with droplet ids
            for (r, c), cell in graph.nodes(data=True):
                if cell.droplet:
                    info = cell.droplet.info
                    offset = 0.1 * len(info)
                    ax.text(c-offset, r+0.1, info, fontsize=16)

            fig.canvas.draw_idle()

        timer.add_callback(render)
        timer.start()
        plt.show()
