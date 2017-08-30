import puddle
from puddle.arch import Architecture


def test_api():
    arch = Architecture.from_file('tests/arches/01.arch')
    sess = puddle.Session(arch)

    d1 = sess.input_droplet((1,1))
    d2 = sess.input_droplet((3,1))

    d3 = sess.mix(d1, d2)

    d4, d5 = sess.split(d3)


if __name__ == '__main__':
    test_api()


'''
templates = sess.input([0,0])
primers   = sess.input([2,0])
mmix      = sess.input([4,0])

# mix reagents
pcr_mix = puddle.mix([templates, primers, mmix])

# activate enzyme
puddle.heat(pcr_mix, 95, 120)

flourescence = 0
threshold    = 1
for cycle in range(10):
    # denature
    puddle.heat(pcr_mix, 95, 30)

    # anneal
    puddle.heat(pcr_mix, 62, 30)

    # extend
    puddle.heat(pcr_mix, 72, 30)

    # image
    flourescence = puddle.flourescence(pcr_mix)
    if flourescence > threshold:
        break

'''
