#include "opencv2/core/core.hpp"
#include "opencv2/imgproc/imgproc.hpp"
#include "opencv2/videoio/videoio.hpp"
#include "opencv2/highgui/highgui.hpp"

#include <iostream>
#include <math.h>

#define UNUSED(x) (void)(x)

using namespace cv;
using namespace std;

int find_dist(int x1, int y1, int x2, int y2){
	return pow(x2 - x1, 2) + pow(y2 - y1, 2);
}

// img must be grayscale current frame, maxArea is max area of fiducial marker, numsides is sides of the fiducial marker
vector<Point> find_fiducial(Mat img, int maxArea, unsigned numSides) {
  Mat edges;
	vector< vector<Point> > fiducialContours;
	vector< vector<Point> > finalContours;

  Canny(img, edges, 70, 200);
  findContours(edges, fiducialContours, RETR_EXTERNAL, CHAIN_APPROX_SIMPLE);

	double minArea = img.rows * img.cols * 0.001;

	for (unsigned i = 0; i < fiducialContours.size(); i++) {
    auto contour = fiducialContours[i];
		if (contourArea(contour) > minArea) {
			finalContours.push_back(contour);
		}
	}

	for(unsigned i = 0; i < finalContours.size(); i++){
    auto contour = finalContours[i];

		if (contourArea(contour) > maxArea) {
			continue;
		}

		vector<Point> approxCurve;
    approxPolyDP(contour, approxCurve, contour.size() * 0.04, true);

		if (approxCurve.size() != numSides || !isContourConvex(approxCurve)) {
			continue;
		}

		//for(unsigned j = 0; j<approxCurve.size(); j++){
		//
		//}
    cout << "Found fiducial with " << numSides << " sides!" << endl;
		return approxCurve;
	}

  cerr << "Could not find fiducial with " << numSides << " sides..." << endl;

  vector<Point> empty;
  return empty;
}

Mat readGray(char* path) {
  Mat frame;
	cout << "Reading " << path << "... ";
	frame = imread(path, CV_LOAD_IMAGE_GRAYSCALE);
  // resize(frame, frame, Size(0, 0), 0.5, 0.5);
	if (frame.empty()){
		cerr << "Could not read " << path << endl;
		exit(1);
	}
	cout << "done!" << endl;
  return frame;
}

typedef struct {
  // a frame from vid is used if current is null
  VideoCapture *vid;
  Mat *current;

  // the base frame to difference between
  Mat *background;

  // detection parameters
  int erode1;
  int dilate1;
  int erode2sub;
} DetectionArgs;



void detect_droplets(int value, void* args_p) {
  // `value` is supposed to be the changed value from the slider, but we don't
  // care because we have the whole struct
  UNUSED(value);
  DetectionArgs *args = (DetectionArgs *)args_p;

  Mat diff;
  absdiff(*args->current, *args->background, diff);
  imshow("diff", diff);

  Mat diffThresh;
  threshold(diff, diffThresh, 30, 255, THRESH_BINARY);
  imshow("diffThresh", diffThresh);

	// Erode the image to get rid of noise
  Mat erodedImg;
	// dilate(diffThresh, erodedImg,
  //        getStructuringElement(MORPH_ELLIPSE, Size(args->k_size1, args->k_size1)));
	erode(diffThresh, erodedImg,
        getStructuringElement(MORPH_ELLIPSE, Size(args->erode1, args->erode1)));
  imshow("eroded1", erodedImg);
	dilate(erodedImg, erodedImg,
         getStructuringElement(MORPH_ELLIPSE, Size(args->dilate1, args->dilate1)));
  // imshow("eroded2", erodedImg);
  int dim = args->dilate1 + args->erode2sub - 20;
	erode(erodedImg, erodedImg,
        getStructuringElement(MORPH_ELLIPSE, Size(dim, dim)));
  imshow("eroded3", erodedImg);

	// Find all the contours in the image, and filter them
	vector< vector<Point> > contours;
  findContours(erodedImg, contours, RETR_EXTERNAL, CHAIN_APPROX_SIMPLE);
	vector< vector<Point> > filteredContours;
	for(unsigned i = 0; i<contours.size(); i++){
    RotatedRect r = minAreaRect(Mat(contours[i]));
		int h = r.size.height;
		int w = r.size.width;
    double area = contourArea(contours[i]);
		if (w != 0 && h != 0 &&
        w / h < 9 && h / w < 9 &&
        50 < area && area < 20000) {
			filteredContours.push_back(contours[i]);
			// cout << contours[i] << endl;
		}
	}

	int n_contours = filteredContours.size();
	cout << "Found " << n_contours << " countours" << endl;

  Mat colored;
  cvtColor(*args->current, colored, CV_GRAY2BGR);
  Scalar color(0,0,255);
  drawContours( colored, filteredContours, -1, color, 2);
  imshow("Colored", colored);
}

extern "C"
int detect_from_files(char* currentPath, char* backgroundPath) {

  Mat currentFrame = readGray(currentPath);
  Mat backgroundFrame = readGray(backgroundPath);

  namedWindow("window");

  DetectionArgs args;
  args.background = &backgroundFrame;
  args.current = &currentFrame;
  args.erode1 = 3;
  args.dilate1 = 50;
  args.erode2sub = 20;

  createTrackbar("Erode Size 1", "window", &args.erode1, 10, &detect_droplets, (void*)&args);
  createTrackbar("Dilate Size 1", "window", &args.dilate1, 200, &detect_droplets, (void*)&args);
  createTrackbar("Erode Sub 2 - 20", "window", &args.erode2sub, 40, &detect_droplets, (void*)&args);

  // make an initial callback
  detect_droplets(0, (void*)&args);

  // don't worry about markers for now

	vector<Point> squareFiducial = find_fiducial(currentFrame, 20000, 4);
	vector<Point> pentagonFiducial = find_fiducial(currentFrame, 20000, 5);

  waitKey(0);

  return 0;
}

extern "C"
void detect_from_camera() {

  namedWindow("window");

  VideoCapture cap(0);

  cap.open(0);
  cout << "Video capture is open: " << cap.isOpened() << endl;

  // cap.set(CAP_PROP_FRAME_COUNT, 1);
  // cap.set(CAP_PROP_MODE, CAP_MODE_GRAY);
  // cap.set(CAP_PROP_BUFFERSIZE, 1);
  // cap.set(CAP_PROP_POS_FRAMES, 5);

  Mat backgroundFrame;
  cap.read(backgroundFrame);
  Mat backgroundFrameGray;
  cvtColor(backgroundFrame, backgroundFrameGray, CV_RGB2GRAY);

  Mat currentFrame;
  cap.read(currentFrame);
  Mat currentFrameGray;
  cvtColor(currentFrame, currentFrameGray, CV_RGB2GRAY);

  DetectionArgs args;
  args.background = &backgroundFrameGray;
  args.current = &currentFrameGray;
  args.erode1 = 3;
  args.dilate1 = 20;
  args.erode2sub = 5;

  createTrackbar("Erode Size 1", "window", &args.erode1, 10, &detect_droplets, (void*)&args);
  createTrackbar("Dilate Size 1", "window", &args.dilate1, 200, &detect_droplets, (void*)&args);
  createTrackbar("Erode Sub 2 - 20", "window", &args.erode2sub, 40, &detect_droplets, (void*)&args);

  detect_droplets(0, (void*)&args);

  for (unsigned i = 0; ; i++) {
    cout << "Loop " << i << endl;
    switch ((char)waitKey(0)) {
    case 'q': goto done;
    case 'c':
      cap.read(backgroundFrame);
      cvtColor(backgroundFrame, backgroundFrameGray, CV_RGB2GRAY);
      break;
    }

    cap.read(currentFrame);
    cvtColor(currentFrame, currentFrameGray, CV_RGB2GRAY);

    vector<Point> squareFiducial = find_fiducial(currentFrame, 20000, 4);
    vector<Point> pentagonFiducial = find_fiducial(currentFrame, 20000, 5);

    // re-run detection
    detect_droplets(0, (void*)&args);
  }

 done:;
}
