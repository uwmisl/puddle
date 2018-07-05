
board_width = 98;

cam_x = 52;
cam_y = 31;
cam_z = 61;

arm_zl = 4;
mount_yl = 6;

// make the base
cube([20, board_width, 3]);

translate ([0, cam_y, 0]) {
    // vertical arm coming from base
    cube([10, cam_yl, cam_z]);

    translate([0,0, cam_z - arm_zl]) {
        // horizontal arm from vertical arm
        cube([cam_x, cam_yl, arm_zl]);

        translate([cam_x, 0, 0]) {
            mount();
        }
    }
}

// from https://www.raspberrypi-spy.co.uk/2013/05/pi-camera-module-mechanical-dimensions/
cam_zl = 1.07; // thickness
cam_yl = 25;   // across the port
cam_xl = 24;   // portside to far edge

module cam_holes(hole_thickness = 2) {
    translate([9.5,2]) {
        circle(diameter=hole_thickness, $fn = 50);
        translate([   0, 21]) circle(diameter=hole_thickness, $fn = 50);
        translate([12.5, 21]) circle(diameter=hole_thickness, $fn = 50);
        translate([12.5,  0]) circle(diameter=hole_thickness, $fn = 50);
    }
}

module mount() {
    difference() {
        union() {
            cube([cam_xl, mount_yl, arm_zl]);
            translate([0, cam_yl - mount_yl, 0])
                cube([cam_xl, mount_yl, arm_zl]);
        }
        translate([0,0,-0.5])
            linear_extrude(height=arm_zl+1)
            cam_holes();
    }
}
