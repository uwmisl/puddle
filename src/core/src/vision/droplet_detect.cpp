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

// // helper function:
// // finds a cosine of angle between vectors
// // from pt0->pt1 and from pt0->pt2
// static double angle( Point pt1, Point pt2, Point pt0 )
// {
//   double dx1 = pt1.x - pt0.x;
//   double dy1 = pt1.y - pt0.y;
//   double dx2 = pt2.x - pt0.x;
//   double dy2 = pt2.y - pt0.y;
//   return (dx1*dx2 + dy1*dy2)/sqrt((dx1*dx1 + dy1*dy1)*(dx2*dx2 + dy2*dy2) + 1e-10);
// }

// img must be grayscale current frame, maxArea is max area of fiducial marker, numsides is sides of the fiducial marker
vector<Point> find_fiducial(Mat img, int maxArea, unsigned numSides) {
  Mat edges;
  vector< vector<Point> > fiducialContours;
  vector< vector<Point> > finalContours;

  Canny(img, edges, 80, 200);

  findContours(edges, fiducialContours, RETR_EXTERNAL, CHAIN_APPROX_SIMPLE);

  double minArea = img.rows * img.cols * 0.001;

  for (unsigned i = 0; i < fiducialContours.size(); i++) {
    auto contour = fiducialContours[i];
    if (contourArea(contour) > minArea) {
      finalContours.push_back(contour);
    }
  }

  vector<Point> bestContour;
  double bestContourScore = HUGE_VAL;
  for(unsigned i = 0; i < finalContours.size(); i++){

    auto contour = finalContours[i];

    if (contourArea(contour) > maxArea) {
      continue;
    }

    vector<Point> approxCurve;
    approxPolyDP(contour, approxCurve, arcLength(contour, true) * 0.05, true);

    if (approxCurve.size() != numSides || !isContourConvex(approxCurve)) {
      continue;
    }

    vector<double> sideLengths;
		for (unsigned j = 0; j < approxCurve.size() - 1; j++) {
			Point p0 = approxCurve[j];
			Point p1 = approxCurve[j + 1];
			double dy = p0.y - p1.y;
			double dx = p0.x - p1.x;
			double len = sqrt(pow(dy, 2) + pow(dx, 2));
			sideLengths.push_back(len);
		}

		vector<double> mean;
		vector<double> stdDev;
		meanStdDev(sideLengths, mean, stdDev);

		double score = stdDev[0] / 12 - contourArea(approxCurve) / 2500;

		if (score < bestContourScore) {
			bestContourScore = score;
			bestContour = approxCurve;
		}
	}

  return bestContour;
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
  VideoCapture* cap;
  unsigned iteration;
  vector< vector<Point> > droplets;
};

extern "C"
DetectionState *makeDetectionState() {
  int history = 500;
  double varThreshold = 80;
  bool detectShadows = false;

  DetectionState *state = new DetectionState;
  state->bgSub = createBackgroundSubtractorMOG2(history, varThreshold, detectShadows);

  state->cap = new VideoCapture(0);
  state->iteration = 0;
  state->droplets = vector< vector<Point> >();

  cout << "VideoCapture opened: " << state->cap->isOpened() << endl;

  state->cap->set(CV_CAP_PROP_FRAME_WIDTH, 320);
  state->cap->set(CV_CAP_PROP_FRAME_HEIGHT, 240);

  Mat currentFrame;
  state->cap->read(currentFrame);
  return state;
}

// returns true if we should quit
extern "C"
bool detect_from_camera(DetectionState *det, DetectionResponse* resp, bool shouldDraw) {
  Mat currentFrame;
  Mat currentFrameGray;

  cout << det << endl;
  cout << "VideoCapture opened: " << det->cap->isOpened() << endl;

  det->cap->read(currentFrame);
  cvtColor(currentFrame, currentFrameGray, CV_BGR2GRAY);

  // find the centers of the fiducial markers
  vector<Point> squareFiducial = find_fiducial(currentFrameGray, 20000, 4);
  vector<Point> pentaFiducial = find_fiducial(currentFrameGray, 20000, 5);
  // fill the response if we found one
  if (squareFiducial.size() > 0) {
    Moments squareMoments =  moments(squareFiducial, false);
    Point2f squareCenter = Point2f(squareMoments.m10/squareMoments.m00, squareMoments.m01/squareMoments.m00);
    resp->squareCenter.y = squareCenter.y;
    resp->squareCenter.x = squareCenter.x;
  } else {
    cout << "Could not find square fiducial!" << endl;
  }
  if (pentaFiducial.size() > 0) {
    Moments pentaMoments =  moments(pentaFiducial, false);
    Point2f pentaCenter = Point2f(pentaMoments.m10/pentaMoments.m00, pentaMoments.m01/pentaMoments.m00);
    resp->pentaCenter.y = pentaCenter.y;
    resp->pentaCenter.x = pentaCenter.x;
  } else {
    cout << "Could not find penta fiducial!" << endl;
  }

  // blur(currentFrameGray, currentFrameGray, Size(3,3));
  Mat dropletMask(currentFrameGray.size(), CV_8UC1);
  dropletMask = 0;
  if(det->droplets.size() > 0){
    drawContours(dropletMask, det->droplets, -1, Scalar(255), -1);
  }

  Mat bg;
  det->bgSub->getBackgroundImage(bg);
  Mat currentFrameMod = currentFrameGray.clone();
  // "Hide" the droplets in currentFrameMod by copying over the background to those locations
  // Background is empty initially, so wait until after the first iteration of the loop is done
  if (det->iteration > 0) {
    bg.copyTo(currentFrameMod, dropletMask);
  }

  Mat fg;
  det->bgSub->apply(currentFrameMod, fg);
  // Don't update the background (weight of 0), but extract the foreground with the droplets
  det->bgSub->apply(currentFrameGray, fg, 0.0);

  if (shouldDraw) {
    imshow("current", currentFrame);
    imshow("foreground", fg);
    cout << currentFrame.size() << endl;
    cout << fg.size() << endl;
    cout << bg.size() << endl;
    if (det->iteration > 0) {
      // background won't exist yet
      imshow("background", bg);
    }
  }

  dilate(fg, fg, Mat(), Point(-1, -1), 2, 1, 1);
  erode(fg, fg, Mat(), Point(-1, -1), 2, 1, 1);
  dilate(fg, fg, Mat(), Point(-1, -1), 2, 1, 1);
  erode(fg, fg, Mat(), Point(-1, -1), 1, 1, 1);

  // Find all the contours in the image, and filter them
  vector< vector<Point> > contours;
  findContours(fg, contours, RETR_EXTERNAL, CHAIN_APPROX_SIMPLE);
  vector< vector<Point> > filteredContours;

  // Find all the contours in the image, and filter them
  for(unsigned i = 0; i<contours.size(); i++){
    RotatedRect r = minAreaRect(Mat(contours[i]));
    int h = r.size.height;
    int w = r.size.width;
    double area = contourArea(contours[i]);
    // Remove irregular droplets
    if (w != 0 && h != 0 &&
        w / h < 9 && h / w < 9 &&
        50 < area && area < 20000) {
      filteredContours.push_back(contours[i]);
      // cout << contours[i] << endl;
      // Wait until the background has been intilialized before storing droplets
      if(det->iteration > 2){
        det->droplets.push_back(contours[i]);
      }
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

  det->iteration += 1;

  // draw the contours
  if (shouldDraw) {
    Mat colored;
    cvtColor(currentFrameGray, colored, CV_GRAY2BGR);
    Scalar color(0,0,255);
    drawContours(colored, filteredContours, -1, color, 2);

    vector< vector<Point> > contour_holder;
    if (pentaFiducial.size() > 0) {
      contour_holder.clear();
      contour_holder.push_back(pentaFiducial);
      drawContours(colored, contour_holder, -1, Scalar(255,255,255), 2);
    }
    if (squareFiducial.size() > 0) {
      contour_holder.clear();
      contour_holder.push_back(squareFiducial);
      drawContours(colored, contour_holder, -1, Scalar(255,255,255), 2);
    }
		// cout << "Polygon with " << approxCurve.size() << " sides." << endl;

    imshow("Colored", colored);

    switch (waitKey(10)) {
    case 'q': return true;
    case 'p': while (waitKey(10) != 'p');
    }
  }

  return false;
}
