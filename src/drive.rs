use crate::motor::{Motor, MotorGuard};
use derive_builder::Builder;

#[derive(Builder)]
pub struct DifferentialDrive {
    left_motor: MotorGuard<dyn Motor>,
    right_motor: MotorGuard<dyn Motor>,
}

impl DifferentialDrive {
    pub fn tank_drive(&self, left_side: f32, right_side: f32) {
        self.left_motor.lock().set_percent(left_side);
        self.right_motor.lock().set_percent(right_side);
    }

    pub fn arcade_drive(&self, forward: f32, rotation: f32) {
        let forward = forward.clamp(-1.0, 1.0);
        let rotation = rotation.clamp(-1.0, 1.0);
        let left_motion = forward + rotation;
        let right_motion = forward - rotation;

        let max = left_motion.abs().max(right_motion.abs()).min(1.0); // either the max or 1.

        self.left_motor.lock().set_percent(left_motion / max);
        self.right_motor.lock().set_percent(right_motion / max);
    }
}

#[derive(Builder)]
pub struct MecanumDrive {
    front_left_motor: MotorGuard<dyn Motor>,
    front_right_motor: MotorGuard<dyn Motor>,
    rear_left_motor: MotorGuard<dyn Motor>,
    rear_right_motor: MotorGuard<dyn Motor>,
}

impl MecanumDrive {
    pub fn cartesian_drive(&self, x_speed: f32, y_speed: f32, rotation: f32) {
        // thanks https://seamonsters-2605.github.io/archive/mecanum/

        // first, we need to ensure all values are between -1 and 1
        let x_speed = x_speed.clamp(-1.0, 1.0);
        let y_speed = y_speed.clamp(-1.0, 1.0);
        let rotation = rotation.clamp(-1.0, 1.0);

        let front_left = y_speed + x_speed + rotation;
        let front_right = y_speed - x_speed - rotation;
        let rear_left = y_speed - x_speed + rotation;
        let rear_right = y_speed + x_speed - rotation;

        // get the highest value so we can scale everything down to match it
        let limit = front_left
            .abs()
            .max(front_right.abs())
            .max(rear_left.abs())
            .max(rear_right.abs())
            .min(1.0);

        self.front_left_motor.lock().set_percent(front_left / limit);
        self.front_right_motor
            .lock()
            .set_percent(front_right / limit);
        self.rear_left_motor.lock().set_percent(rear_left / limit);
        self.rear_right_motor.lock().set_percent(rear_right / limit);
    }

    pub fn field_cartesian_drive(
        &mut self,
        x_speed: f32,
        y_speed: f32,
        rotation: f32,
        heading: f32,
    ) {
        let heading_cos = heading.cos();
        let heading_sin = heading.sin();
        let field_x_speed = x_speed * heading_cos + y_speed * heading_sin;
        let field_y_speed = -x_speed * heading_sin + y_speed * heading_cos;
        self.cartesian_drive(field_x_speed, field_y_speed, rotation);
    }
}
