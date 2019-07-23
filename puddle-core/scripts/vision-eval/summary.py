import argparse
import json
import statistics
import os.path

def summary(files):

    for f in files:
        assert os.path.isfile(f)

    output = {}
    jsons = [json.load(open(f)) for f in files]

    output['mean precision'] = statistics.mean(j['precision'] for j in jsons)
    output['mean recall'] = statistics.mean(j['recall'] for j in jsons)

    return output

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Summarize the diff reports')
    parser.add_argument('files', help='json data files', nargs='+')
    parser.add_argument('-o','--output')
    parser.add_argument('-v','--verbose', action='store_true')
    args = vars(parser.parse_args())
    output = summary(args['files'])

    out_path = args.get('output')
    if out_path:
        json.dump(output, open(out_path, 'w'), indent=2)
        if args['verbose']:
            print(json.dumps(output, indent=2))
    else:
        print(json.dumps(output, indent=2))
