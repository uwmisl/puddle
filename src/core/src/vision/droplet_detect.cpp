#include "opencv2/core/core.hpp"
#include "opencv2/imgproc/imgproc.hpp"
#include "opencv2/videoio/videoio.hpp"
#include "opencv2/video/background_segm.hpp"
#include "opencv2/highgui/highgui.hpp"

#include <iostream>
#include <math.h>

#define UNUSED(x) (void)(x)

using namespace cv;
using namespace std;

int find_dist(int x1, int y1, int x2, int y2) {
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

	for (unsigned i = 0; i < finalContours.size(); i++) {
    auto contour = finalContours[i];

		if (contourArea(contour) > maxArea) {
			continue;
		}

		vector<Point> approxCurve;
    approxPolyDP(contour, approxCurve, contour.size() * 0.04, true);

		if (approxCurve.size() != numSides || !isContourConvex(approxCurve)) {
			continue;
		}

		//for(unsigned j = 0; j<approxCurve.size(); j++) {
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

struct MyPoint {
  unsigned y;
  unsigned x;
};

struct Contour {
  size_t numPoints;
  struct MyPoint *points;
};

struct DetectionResponse {
  size_t numContours;
  struct Contour *contours;
  struct MyPoint pentaCenter;
  struct MyPoint squareCenter;
};

struct DetectionState {
  Ptr<BackgroundSubtractor> bgSub;
  VideoCapture cap;
};

extern "C"
DetectionState *makeDetectionState() {
  int history = 500;
  double varThreshold = 16;
  bool detectShadows = false;

  DetectionState *state = (DetectionState*) malloc(sizeof(DetectionState));
  state->bgSub = createBackgroundSubtractorMOG2(history, varThreshold, detectShadows);
  state->cap = VideoCapture(0);

  return state;
}

// returns true if we should quit
extern "C"
bool detect_from_camera(DetectionState *det, DetectionResponse* resp, bool shouldDraw) {
  Mat currentFrame;
  Mat currentFrameGray;

  det->cap.read(currentFrame);
  cvtColor(currentFrame, currentFrameGray, CV_RGB2GRAY);

  vector<Point> squareFiducial = find_fiducial(currentFrame, 20000, 4);
  vector<Point> pentagonFiducial = find_fiducial(currentFrame, 20000, 5);

  blur(currentFrameGray, currentFrameGray, Size(3,3));

  Mat fg;
  det->bgSub->apply(currentFrameGray, fg);

  if (shouldDraw) {
    imshow("current", currentFrame);
    imshow("foreground", fg);

    Mat bg;
    det->bgSub->getBackgroundImage(bg);
    imshow("background", bg);
  }

  // Find all the contours in the image, and filter them
  vector< vector<Point> > contours;
  findContours(fg, contours, RETR_EXTERNAL, CHAIN_APPROX_SIMPLE);
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

  size_t numContours = filteredContours.size();
  cout << "Found " << numContours << " countours" << endl;

  // free the contents of the old response
  if (!resp->contours) {
    for (size_t i = 0; i < resp->numContours; i++) {
      free(resp->contours[i].points);
    }
    free(resp->contours);
  }

  // fill the response with the filteredContours
  resp->contours = (Contour*) malloc(numContours * sizeof(Contour));
  for (size_t i = 0; i < numContours; i++) {
    size_t numPoints = filteredContours[i].size();
    Contour *c = &resp->contours[i];
    c->numPoints = numPoints;
    c->points = (MyPoint*) malloc(numPoints * sizeof(MyPoint));
    for (size_t j = 0; j < numPoints; j++) {
      MyPoint *p = &c->points[j];
      p->y = filteredContours[i][j].y;
      p->x = filteredContours[i][j].x;
    }
  }

  // draw the contours
  if (shouldDraw) {
    Mat colored;
    cvtColor(currentFrameGray, colored, CV_GRAY2BGR);
    Scalar color(0,0,255);
    drawContours( colored, filteredContours, -1, color, 2);
    imshow("Colored", colored);

    switch (waitKey(10)) {
    case 'q': return true;
    case 'p': while (waitKey(10) != 'p');
    }
  }

  return false;
}

extern "C"
void detect_from_camera2() {

  namedWindow("window");

  VideoCapture cap(0);

  cap.open(0);
  cout << "Video capture is open: " << cap.isOpened() << endl;

  // cap.set(CAP_PROP_FRAME_COUNT, 1);
  // cap.set(CAP_PROP_MODE, CAP_MODE_GRAY);
  // cap.set(CAP_PROP_BUFFERSIZE, 1);
  // cap.set(CAP_PROP_POS_FRAMES, 5);

  Mat currentFrame;
  Mat currentFrameGray;

  // int history = 500;
  // double dist2Threshold = 400.0;
  // bool detectShadows = false;
  // Ptr<BackgroundSubtractorKNN> bg = createBackgroundSubtractorKNN(history, dist2Threshold, detectShadows);

  int history = 500;
  double varThreshold = 16;
  bool detectShadows = false;
  Ptr<BackgroundSubtractor> bgSub = createBackgroundSubtractorMOG2(history, varThreshold, detectShadows);

  int i = 0;
  char key = 0;
  while ((key = waitKey(10)) != 'q') {
    if (key == 'p') {
      while (waitKey(10) != 'p');
    }
    cout << "Loop " << i++ << endl;

    cap.read(currentFrame);
    imshow("current", currentFrame);
    cvtColor(currentFrame, currentFrameGray, CV_RGB2GRAY);

    vector<Point> squareFiducial = find_fiducial(currentFrame, 20000, 4);
    vector<Point> pentagonFiducial = find_fiducial(currentFrame, 20000, 5);

    blur(currentFrameGray, currentFrameGray, Size(3,3));

    // get the fg and bg
    Mat fg;
    bgSub->apply(currentFrameGray, fg);
    imshow("foreground", fg);

    Mat bg;
    bgSub->getBackgroundImage(bg);
    imshow("background", bg);

    // Find all the contours in the image, and filter them
    vector< vector<Point> > contours;
    findContours(fg, contours, RETR_EXTERNAL, CHAIN_APPROX_SIMPLE);
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

    // draw the contours
    Mat colored;
    cvtColor(currentFrameGray, colored, CV_GRAY2BGR);
    Scalar color(0,0,255);
    drawContours( colored, filteredContours, -1, color, 2);
    imshow("Colored", colored);
  }
}
