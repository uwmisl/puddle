import argparse
import cv2
import json
import os.path


def diff(labeled_path, guessed_path):
    assert os.path.isfile(labeled_path)
    assert os.path.isfile(guessed_path)

    groundTruth = cv2.imread(labeled_path, 0)
    detectedObject = cv2.imread(guessed_path, 0)

    output = {}

    _, groundTruthBinary = cv2.threshold(groundTruth, 127, 255, cv2.THRESH_BINARY_INV)
    _, detectedObjectBinary = cv2.threshold(detectedObject, 127, 255, cv2.THRESH_BINARY_INV)

    intersectionImg = cv2.bitwise_and(groundTruthBinary, detectedObjectBinary)

    # Area of droplets in ground truth
    totalTruthArea = cv2.countNonZero(groundTruthBinary)
    # output['totalTruthArea'] = totalTruthArea

    # Area of detected droplets
    totalDetectedArea = cv2.countNonZero(detectedObjectBinary)
    output['totalDetectedArea'] = totalDetectedArea

    # Area of intersection between detected droplets and ground truth
    totalIntersectionArea = cv2.countNonZero(intersectionImg)
    output['totalIntersectionArea'] = totalIntersectionArea

    truePositive = totalIntersectionArea
    output['truePositive'] = truePositive
    falsePositive = totalDetectedArea - totalIntersectionArea
    output['falsePositive'] = falsePositive
    trueNegative = 320*240 - (totalDetectedArea + totalTruthArea - totalIntersectionArea)
    output['trueNegative'] = trueNegative
    falseNegative = totalTruthArea - totalIntersectionArea
    output['falseNegative'] = falseNegative
    falseNegativeRate = falseNegative / (truePositive + falseNegative)
    trueNegativeRate = trueNegative / (truePositive + falseNegative)

    # Metric 1 and 2
    precision = truePositive / (truePositive + falsePositive)
    output['precision'] = precision
    recall = truePositive / (truePositive + falseNegative)
    output['recall'] = recall

    # Metric 3 (also called true positive rate)
    diceCoefficient = totalIntersectionArea/totalTruthArea
    output['diceCoefficient'] = diceCoefficient

    # Metric 4
    falsePositiveRate = falsePositive / (falsePositive+trueNegative)
    output['falsePositiveRate'] = falsePositiveRate

    # Metric 5
    f1Score = 2*(recall * precision) / (recall + precision)
    output['f1Score'] = f1Score

    # Metric 6 - Intersection over Union
    IoU = truePositive / (truePositive + falsePositive + falseNegative)
    output['IoU'] = IoU

    return output

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Diff a labeled and guessed image')
    parser.add_argument('-l','--labeled', help='Labeled image path', required=True)
    parser.add_argument('-g','--guessed', help='Guessed image path', required=True)
    parser.add_argument('-o','--output')
    args = vars(parser.parse_args())
    output = diff(args['labeled'], args['guessed'])

    out_path = args.get('output')
    if out_path:
        json.dump(output, open(out_path, 'w'), indent=2)
    else:
        print(json.dumps(output, indent=2))
