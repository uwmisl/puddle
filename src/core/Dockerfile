# use buster so we can get opencv 3.2 that's on the pi
FROM debian:buster-slim

RUN \
  dpkg --add-architecture armhf && \
  apt-get update && \
  apt-get install -y pkg-config && \
  apt-get install -y curl && \
  apt-get install -y crossbuild-essential-armhf  && \
  apt-get install -y libopencv-core-dev:armhf    && \
  apt-get install -y libopencv-imgproc-dev:armhf && \
  apt-get install -y libopencv-calib3d-dev:armhf &&\
  apt-get install -y libopencv-videoio-dev:armhf &&\
  apt-get install -y libopencv-highgui-dev:armhf --no-install-recommends

RUN echo "deb http://archive.raspberrypi.org/debian stretch main" >> /etc/apt/sources.list
RUN curl http://archive.raspberrypi.org/debian/raspberrypi.gpg.key | apt-key add -
RUN apt-get update && apt-get install pigpio:armhf
RUN apt-get install -y qemu-user
RUN apt-get install -y git

ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
    CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_RUNNER=qemu-arm \
    CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc \
    CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++ \
    RUNNING_IN_CROSS=1