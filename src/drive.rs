use crate::motor::{Motor, MotorGuard};
use derive_builder::Builder;

#[derive(Builder)]
pub struct DifferentialDrive {
    left_motor: MotorGuard<dyn Motor>,
    right_motor: MotorGuard<dyn Motor>,
}

impl DifferentialDrive {
    pub fn tank_drive(&self, left_side: f64, right_side: f64) {
        self.left_motor.lock().set_percent(left_side);
        self.right_motor.lock().set_percent(right_side);
    }

    pub fn arcade_drive(&self, forward: f64, rotation: f64) {
        let forward = forward.clamp(-1.0, 1.0);
        let rotation = rotation.clamp(-1.0, 1.0);
        let left_motion = forward + rotation;
        let right_motion = forward - rotation;

        let max = left_motion.abs().max(right_motion.abs()).min(1.0); // either the max or 1.

        self.left_motor.lock().set_percent(left_motion / max);
        self.right_motor.lock().set_percent(right_motion / max);
    }
}
