#include "opencv2/opencv.hpp"
#include "opencv2/highgui/highgui.hpp"
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
		return approxCurve;
	}

  cerr << "Could not find fiducial with " << numSides << " sides" << endl;

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

struct Args {
  Mat *current;
  Mat *diff;
  int erode1;
  int dilate1;
  int erode2sub;
};

void do_something(int value, void* args_p) {

  UNUSED(value);

  struct Args *args = (struct Args *)args_p;
  Mat diffThresh;
  threshold(*args->diff, diffThresh, 30, 255, THRESH_BINARY);
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

	// Find alllthe contours in the image, and filter them
	vector<vector<Point>> contours;
  findContours(erodedImg, contours, RETR_EXTERNAL, CHAIN_APPROX_SIMPLE);
	vector<vector<Point>> filteredContours;
	for(unsigned i = 0; i<contours.size(); i++){
    RotatedRect r = minAreaRect(Mat(contours[i]));
		int h = r.size.height;
		int w = r.size.width;
		if(w!=0 && h!=0 and w/h < 9 and h/w < 9 and contourArea(contours[i])>50 and contourArea(contours[i])<20000){
			filteredContours.push_back(contours[i]);
			cout << contours[i] << endl;
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
int detect_droplets(char* framePath, char* backgroundPath) {

  Mat currentFrame = readGray(framePath);
  Mat backgroundImg = readGray(backgroundPath);

	//Subtract the images and do a bit of smoothing
  Mat absDiffImg;
  absdiff(currentFrame, backgroundImg, absDiffImg);
  // imshow("absDiffImg", absDiffImg);

  namedWindow("window");

  struct Args args;
  args.diff = &absDiffImg;
  args.current = &currentFrame;
  args.erode1 = 3;
  args.dilate1 = 50;
  args.erode2sub = 20;

  createTrackbar("Erode Size 1", "window", &args.erode1, 10, &do_something, (void*)&args);
  createTrackbar("Dilate Size 1", "window", &args.dilate1, 200, &do_something, (void*)&args);
  createTrackbar("Erode Sub 2 - 20", "window", &args.erode2sub, 40, &do_something, (void*)&args);

  // make an initial callback
  do_something(0, (void*)&args);

  // don't worry about markers for now

	// vector<Point> squareFiducial = find_fiducial(currentFrame, 20000, 4);
	// vector<Point> pentagonFiducial = find_fiducial(currentFrame, 20000, 5);

	// cout << "Found " << squareFiducial.size() << " countours\n" << endl;
	// cout << "Found " << pentagonFiducial.size() << " countours\n" << endl;

  waitKey(0);

  return 0;
}
