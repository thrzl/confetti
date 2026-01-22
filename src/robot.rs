use wpihal::{driver_station, initialize_common as initialize_hal};

/// the main robot type. you should start by implementing this trait for your own class, like this:
/// ```
/// struct MyRobot {}
///
/// impl Robot for MyRobot {
///     fn autonomous_periodic(&self) {
///         println!("started autonomous")
///     }
/// }
///
/// ```
pub trait Robot {
    fn teleop_init(&self) {}
    fn teleop_periodic(&self) {}
    fn teleop_exit(&self) {}
    fn disabled_periodic(&self) {}
    fn autonomous_periodic(&self) {}
    fn test_periodic(&self) {}
}

struct MyRobot {}

impl Robot for MyRobot {
    fn autonomous_periodic(&self) {
        println!("started autonomous")
    }
}
