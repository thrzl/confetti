use parking_lot::Mutex;
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, Instant};

pub trait Motor: Send {
    fn check_timeout(&mut self);
    fn set(&mut self, value: f32);
    fn stop(&mut self);
}

pub type MotorGuard<M> = Arc<Mutex<M>>;

pub struct MotorWatchdog {
    motors: Vec<Weak<Mutex<dyn Motor + Send>>>,
}

impl MotorWatchdog {
    pub fn new() -> Self {
        return MotorWatchdog { motors: Vec::new() };
    }
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

    fn add_motor(&mut self, motor: Weak<Mutex<dyn Motor + Send>>) {
        self.motors.push(motor)
    }
}

pub static MOTOR_REGISTRY: LazyLock<Mutex<MotorWatchdog>> =
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

    fn set(&mut self, value: f32) {
        let _ = value;
        todo!()
    }

    fn stop(&mut self) {
        self.set(0.0)
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
