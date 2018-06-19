#include "opencv2/opencv.hpp"
// #include "opencv2/highgui/highgui.hpp"
#include <math.h>

using namespace cv;
using namespace std;

// functions exported to C can be called from Rust
// they should have only C types, so no cpp objects
extern "C" {
  int hello_world();
}

int find_dist(int x1, int y1, int x2, int y2){
	return pow(x2 - x1, 2) + pow(y2 - y1, 2);
}

// img must be grayscale current frame, maxArea is max area of fiducial marker, numsides is sides of the fiducial marker
vector<Point> find_fiducial(Mat img, int maxArea, unsigned numSides) {
  Mat edges;
	vector<vector<Point>> fiducialContours;
	vector<vector<Point>> finalContours;

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

  abort();
}

int hello_world() {
	string home = getenv("HOME");
	string currentFramePath = home + "/6drop1.png";
	string backgroundImgPath = home + "/6dropbackground.png";

	// Read the current frame
  Mat currentFrame;
  Mat currentFrame1;
	cout<<"TEST"<<flush;
	currentFrame1 = imread(currentFramePath);
	cout<<currentFrame1.channels()<<flush;
	if(currentFrame1.empty()){
		//exit(0);
		cout<<"Could not read image"<<flush;
	}
	// Convert the frame to grayscale
	cvtColor(currentFrame1, currentFrame, CV_BGR2GRAY);

	cout<<"Current done"<< endl;

	//Read the background frame
  Mat backgroundImg1;
  Mat backgroundImg;
	backgroundImg1 = imread(backgroundImgPath);
	// Convert the background image to grayscale
  cvtColor(backgroundImg1, backgroundImg, CV_BGR2GRAY);
	if(backgroundImg1.empty()){
		cout<<"Could not open background"<<flush;
	}

	//Subtract the images and do a bit of smoothing
  Mat absDiffImg;
  absdiff(currentFrame, backgroundImg, absDiffImg);

	//Take the threshold to isolate significant differences
  Mat diffThresh;
  threshold (absDiffImg, diffThresh, 30, 255, THRESH_BINARY);

	//Erode the image to get rid of noise
  Mat erodedImg;
	erode(diffThresh, erodedImg, Mat(), Point(-1, -1), 1, 1, 1);
	vector<vector<Point>> contours;
  imshow("erodeImg", erodedImg);

	// Find alllthe contours in the image, and filter them
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

	vector<Point> squareFiducial = find_fiducial(currentFrame, 20000, 4);
	vector<Point> pentagonFiducial = find_fiducial(currentFrame, 20000, 5);

	cout << "Found " << squareFiducial.size() << " countours\n" << endl;
	cout << "Found " << pentagonFiducial.size() << " countours\n" << endl;

  waitKey(0);

  return 0;
}
