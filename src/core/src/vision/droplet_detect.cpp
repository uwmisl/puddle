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
  VideoCapture* cap;
  unsigned iteration = 0;

  int lo_h = 45, lo_s = 70, lo_v = 20;
  int hi_h = 75, hi_s = 255, hi_v = 255;

  int blur_size = 3;

  int close_size = 5;
  int open_size = 3;
  int bonus = 5;
};

extern "C"
DetectionState *makeDetectionState(bool trackbars) {
  DetectionState *det = new DetectionState;

  det->cap = new VideoCapture(0);

  cout << "VideoCapture opened: " << det->cap->isOpened() << endl;

  det->cap->set(CV_CAP_PROP_FRAME_WIDTH, 320);
  det->cap->set(CV_CAP_PROP_FRAME_HEIGHT, 240);

  if (trackbars) {
    namedWindow("settings");

    createTrackbar("lo h", "settings", &det->lo_h, 180, NULL, NULL);
    createTrackbar("hi h", "settings", &det->hi_h, 180, NULL, NULL);
    createTrackbar("lo s", "settings", &det->lo_s, 255, NULL, NULL);
    createTrackbar("hi s", "settings", &det->hi_s, 255, NULL, NULL);
    createTrackbar("lo v", "settings", &det->lo_v, 255, NULL, NULL);
    createTrackbar("hi v", "settings", &det->hi_v, 255, NULL, NULL);

    createTrackbar("blur", "settings", &det->blur_size, 15, NULL, NULL);
    createTrackbar("close", "settings", &det->close_size, 35, NULL, NULL);
    createTrackbar("open", "settings", &det->open_size, 35, NULL, NULL);
    createTrackbar("bonus", "settings", &det->bonus, 15, NULL, NULL);
  }

  Mat currentFrame;
  det->cap->read(currentFrame);
  return det;
}

// returns true if we should quit
extern "C"
bool detect_from_camera(DetectionState *det, DetectionResponse* resp, bool shouldDraw) {
  Mat raw;
  Mat hsv;

  cout << det << endl;
  cout << "VideoCapture opened: " << det->cap->isOpened() << endl;

  det->cap->read(raw);
  Mat blurred;
  int blur_size = max(det->blur_size, 1);
  blur(raw, blurred, Size(blur_size, blur_size));

  cvtColor(blurred, hsv, CV_BGR2HSV);

  Scalar lowerb(det->lo_h, det->lo_s, det->lo_v);
  Scalar upperb(det->hi_h, det->hi_s, det->hi_v);
  Mat isColor;
  inRange(hsv, lowerb, upperb, isColor);

  Mat closed, opened;

  int close_size = max(det->close_size, 1);
  Mat close_morph = getStructuringElement(MORPH_ELLIPSE, Size(close_size, close_size));
  dilate(isColor, closed, close_morph);
  erode(closed, closed, close_morph);

  int open_size = max(det->open_size, 1);
  Mat open_morph = getStructuringElement(MORPH_ELLIPSE, Size(open_size, open_size));
  erode(closed, opened, open_morph);
  Mat open_bonus_morph = getStructuringElement(MORPH_ELLIPSE, Size(open_size + det->bonus, open_size + det->bonus));
  dilate(opened, opened, open_bonus_morph);

  // find the centers of the fiducial markers
  vector<Point> squareFiducial = find_fiducial(raw, 20000, 4);
  vector<Point> pentaFiducial = find_fiducial(raw, 20000, 5);
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

  // Find all the contours in the image, and filter them
  vector< vector<Point> > contours;
  findContours(opened, contours, RETR_EXTERNAL, CHAIN_APPROX_SIMPLE);
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
    imshow("closed", closed);
    imshow("blurred", blurred);
    imshow("in range", isColor);
    imshow("opened", opened);

    Scalar color(0,0,255);
    drawContours(raw, filteredContours, -1, color, 2);

    vector< vector<Point> > contour_holder;
    if (pentaFiducial.size() > 0) {
      contour_holder.clear();
      contour_holder.push_back(pentaFiducial);
      drawContours(raw, contour_holder, -1, Scalar(255,255,255), 2);
    }
    if (squareFiducial.size() > 0) {
      contour_holder.clear();
      contour_holder.push_back(squareFiducial);
      drawContours(raw, contour_holder, -1, Scalar(255,255,255), 2);
    }
		// cout << "Polygon with " << approxCurve.size() << " sides." << endl;

    imshow("Colored", raw);

    switch (waitKey(10)) {
    case 'q': return true;
    case 'p': while (waitKey(10) != 'p');
    }
  }

  return false;
}
