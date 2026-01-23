use parking_lot::Mutex;
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, Instant};

pub trait Motor: Send {
    /// check if the watchdog timeout has been exceeded. if it has,
    /// this function should stop the motor immediately.
    fn check_timeout(&mut self);

    /// run a motor at a percentage of its capacity.
    fn set_percent(&mut self, percentage: f64);

    /// run a motor at a set voltage.
    fn set_voltage(&mut self, volts: f64);

    /// stop the motor.
    fn stop(&mut self);
}

/// carries a motor that can be obtained via `.lock()`.
///
/// really an `Arc<parking_lot::Mutex<Motor>>` under the hood.
pub type MotorGuard<M> = Arc<Mutex<M>>;

/// stores motors and essentially "ticks" them every loop.
///
/// ticked motors that have not received updates in too long should immediately stop.
pub(crate) struct MotorWatchdog {
    motors: Vec<Weak<Mutex<dyn Motor + Send>>>,
}

impl MotorWatchdog {
    /// initialize a new, empty motor watchdog.
    pub fn new() -> Self {
        return MotorWatchdog { motors: Vec::new() };
    }

    /// check all stored motors. automatically cleans up dropped motors.
    pub fn check_motors(&mut self) {
        self.motors.retain(|item: &Weak<Mutex<dyn Motor + Send>>| {
            if let Some(motor) = item.upgrade() {
                motor.lock().check_timeout();
                true
            } else {
                false
            }
        });
    }

    /// add a new motor from a weak reference. automatically run when a motor is initialized.
    fn add_motor(&mut self, motor: Weak<Mutex<dyn Motor + Send>>) {
        self.motors.push(motor)
    }
}

/// the global motor watchdog. ensures that motors receive updates and automatically stop them if they do not.
pub(crate) static MOTOR_REGISTRY: LazyLock<Mutex<MotorWatchdog>> =
    LazyLock::new(|| Mutex::new(MotorWatchdog::new()));
const WATCHDOG_TIMEOUT: Duration = Duration::from_millis(100);

pub struct SparkMAX {
    watchdog_time: Instant,
    port: i32,
}

impl Motor for SparkMAX {
    fn check_timeout(&mut self) {
        if self.watchdog_time.elapsed() > WATCHDOG_TIMEOUT {
            self.stop();
        }
    }

    fn set_percent(&mut self, percentage: f64) {
        let _ = percentage;
        todo!()
    }

    fn set_voltage(&mut self, volts: f64) {
        let _ = volts;
        todo!()
    }

    fn stop(&mut self) {
        self.set_percent(0.0)
    }
}

impl SparkMAX {
    pub fn new(port: i32) -> MotorGuard<Self> {
        let motor = MotorGuard::new(Mutex::new(Self {
            port,
            watchdog_time: Instant::now(),
        }));
        let trait_motor: Arc<Mutex<dyn Motor + Send>> = motor.clone();
        MOTOR_REGISTRY
            .lock()
            .add_motor(Arc::downgrade(&trait_motor));
        motor
    }
}
