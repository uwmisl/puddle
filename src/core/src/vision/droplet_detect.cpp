#include "opencv2/core/core.hpp"
#include "opencv2/imgproc/imgproc.hpp"
#include "opencv2/videoio/videoio.hpp"
#include "opencv2/video/background_segm.hpp"
#include "opencv2/highgui/highgui.hpp"

#include <iostream>
#include <thread>
#include <mutex>
#include <math.h>

#define UNUSED(x) (void)(x)

using namespace cv;
using namespace std;

int find_dist(int x1, int y1, int x2, int y2) {
	return pow(x2 - x1, 2) + pow(y2 - y1, 2);
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
};

struct DetectionState {
  VideoCapture* cap;
  std::mutex lock;
  std::thread* grabber;
  unsigned iteration = 0;

  int lo_h = 60, lo_s = 83, lo_v = 20;
  int hi_h = 80, hi_s = 255, hi_v = 255;

  int blur_size = 3;

  int close_size = 15;
  int open_size = 3;
  int bonus = 0;
};

void grab_frames(DetectionState* det) {
  auto delay = std::chrono::milliseconds(10);
  while (true) {
    det->lock.lock();
    det->cap->grab();
    // cout << "grabbed" << endl;
    det->lock.unlock();
    std::this_thread::sleep_for(delay);
  }
}

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

  det->grabber = new std::thread(grab_frames, det);
  return det;
}

// returns true if we should quit
extern "C"
bool detect_from_camera(DetectionState *det, DetectionResponse* resp, bool shouldDraw) {
  Mat raw;
  Mat hsv;

  // cout << "VideoCapture opened: " << det->cap->isOpened() << endl;

  det->lock.lock();
  det->cap->retrieve(raw);
  // cout << "retrieved!!!!" << endl;
  det->lock.unlock();

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
  // cout << "Found " << numContours << " countours" << endl;

  // free the contents of the old response
  if (!resp->contours) {
    for (size_t i = 0; i < resp->numContours; i++) {
      free(resp->contours[i].points);
    }
    free(resp->contours);
  }

  // fill the response with the filteredContours
  resp->contours = (Contour*) malloc(numContours * sizeof(Contour));
  resp->numContours = numContours;
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

    imshow("Colored", raw);

    switch (waitKey(10)) {
    case 'q': return true;
    case 'p':
      cout << "Pausing..." << endl;
      while (waitKey(10) != 'p');
      cout << "Resuming..." << endl;
    }
  }

  return false;
}
