
import puddle
import unittest


class TestPuddleStuff(unittest.TestCase):

    def setUp(self):
        arch_path = puddle.project_path('tests/arches/arch01.json')

        session = puddle.mk_session(arch_path)
        self.session = session.__enter__()
        self.addCleanup(session.__exit__, None, None, None)

    def test_easy(self):
        a = self.session.create((1, 1), 1.0, (1, 1))
        b = self.session.create(None, 1.0, (1, 1))
        c = a.mix(b)

        droplets = self.session.droplets()

        # TODO droplet ids should be strings at some point
        self.assertSetEqual(set(droplets.keys()), {c._id})

    def test_consumed(self):
        a = self.session.create(None, 1.0, (1, 1))
        b = self.session.create(None, 1.0, (1, 1))
        c = a.mix(b)

        self.assertIsNotNone(c)

        with self.assertRaises(puddle.DropletConsumed):
            a.mix(b)

    def test_volume(self):
        a = self.session.create(None, 1.0, (1, 1))
        b = self.session.create(None, 2.0, (1, 1))
        ab = self.session.mix(a, b)

        (a_split, b_split) = self.session.split(ab)

        droplets = self.session.droplets()
        self.assertEqual(droplets[a_split._id]['volume'], 1.5)
        self.assertEqual(droplets[b_split._id]['volume'], 1.5)
