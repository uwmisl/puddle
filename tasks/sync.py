import invoke

from tasks import root


@invoke.task
def server(c):
    with c.cd(root + 'src'):
        profile = '--release' if c['release'] else ''
        c.time_run(f'cargo build {profile} ' +
                   f'--bin puddle-server --target {c.target}')

        profile = 'release' if c['release'] else 'debug'
        binary = f'target/{c.target}/{profile}/puddle-server'
        c.run(f'{c.rsync} {binary} {c.pi}:')


@invoke.task
def boards(c):
    with c.cd(root):
        c.run(f'{c.rsync} --relative tests/./arches/*.json {c.pi}:')


@invoke.task(default=True)
def all(c):
    server(c)
    boards(c)


namespace = invoke.Collection(server, boards, all)
namespace.configure({
    'pi': 'blueberry-pie.zt',
    'rsync': 'rsync -iP --compress-level=9',
    'release': True,
    'target': 'armv7-unknown-linux-musleabihf'
})
