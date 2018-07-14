use std::f64::{INFINITY, NAN};
use std::time::Duration;

pub struct PidController {
    pub p_gain: f64,
    pub i_gain: f64,
    pub d_gain: f64,

    pub i_min: f64,
    pub i_max: f64,

    pub out_min: f64,
    pub out_max: f64,

    pub target: f64,

    total_error: f64,
    prev_error: f64,
}

impl PidController {
    pub fn new(p_gain: f64, i_gain: f64, d_gain: f64) -> Self {
        PidController {
            p_gain,
            i_gain,
            d_gain,
            ..PidController::default()
        }
    }

    pub fn set_target(&mut self, target: f64) {
        self.target = target;
    }

    pub fn update(&mut self, measured: f64, dt: &Duration) -> f64 {
        let dt_seconds = duration_seconds(dt);

        let error = self.target - measured;

        let p = self.p_gain * error;

        let new_i = self.i_gain * error * dt_seconds;
        self.total_error = (self.total_error + new_i).min(self.i_max).max(self.i_min);
        let i = self.total_error;

        let d = if self.prev_error.is_nan() {
            0.0
        } else {
            self.d_gain * (error - self.prev_error) / dt_seconds
        };

        self.prev_error = error;

        // return the sum of the PID components, clamping the output
        (p + i + d).max(self.out_min).min(self.out_max)
    }
}

impl Default for PidController {
    fn default() -> Self {
        PidController {
            p_gain: 0.0,
            i_gain: 0.0,
            d_gain: 0.0,

            i_min: 0.0,
            i_max: INFINITY,

            out_min: 0.0,
            out_max: INFINITY,

            target: 0.0,
            total_error: 0.0,
            prev_error: NAN,
        }
    }
}

fn duration_seconds(duration: &Duration) -> f64 {
    (duration.as_secs() as f64) + (duration.subsec_nanos() as f64) * 1e9
}
