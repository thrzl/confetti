use crate::{enums::RobotMode, robot::Robot};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};
use wpihal::initialize_common as initialize_hal;

pub mod can;
pub mod drive;
pub mod enums;
pub mod errors;
pub mod motor;
pub mod robot;
pub mod prelude {
    pub use super::{
        drive::{DifferentialDrive, DifferentialDriveBuilder, MecanumDrive, MecanumDriveBuilder},
        errors::HALError,
        motor::{MotorGroup, SparkMAX},
        robot::Robot,
        run,
    };
}

use motor::MOTOR_REGISTRY;

const LOOP_PERIOD: Duration = Duration::from_millis(20);

/// begin the robot loop.
///
/// the loop runs every 20 ms, and handles periodic functions as well as
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
        MOTOR_REGISTRY.lock().motor_periodic();
        let elapsed = loop_time.elapsed();
        if elapsed > LOOP_PERIOD {
            println!("loop overrun") // do something better with this later
        } else {
            sleep(LOOP_PERIOD.saturating_sub(elapsed))
        }
    }
}
