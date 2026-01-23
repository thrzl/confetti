use crate::{enums::RobotMode, motor::Motor, robot::Robot};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};
use wpihal::initialize_common as initialize_hal;

pub mod enums;
pub mod errors;
pub mod motor;
pub mod robot;

use motor::{MOTOR_REGISTRY, MotorGuard, SparkMAX};

const LOOP_PERIOD: Duration = Duration::from_millis(20);

pub fn run(mut robot: impl Robot) -> ! {
    initialize_hal();

    loop {
        let loop_time = Instant::now();
        let control_word = wpihal::driver_station::get_control_word()
            .expect("failed to get control word from driverstation");
        match RobotMode::from_control_word(&control_word) {
            RobotMode::Autonomous => robot.autonomous_periodic(),
            RobotMode::Teleoperated => robot.teleop_periodic(),
            RobotMode::Test => robot.test_periodic(),
            _ => robot.disabled_periodic(),
        }
        MOTOR_REGISTRY.lock().check_motors();
        let elapsed = loop_time.elapsed();
        if elapsed > LOOP_PERIOD {
            println!("loop overrun") // do something better with this later
        } else {
            sleep(LOOP_PERIOD.saturating_sub(elapsed))
        }
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

struct MyRobot {
    front_right: MotorGuard<SparkMAX>,
}

impl Robot for MyRobot {
    fn teleop_init(&mut self) {
        self.front_right = SparkMAX::new(8);
        self.front_right.lock().set(0.5);
    }
}

pub fn make_bot() {
    let bot = MyRobot {};
    run(bot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
