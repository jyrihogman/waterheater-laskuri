use chrono::{DateTime, Utc};

pub trait TimeProvider {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemTimeProvider;

impl TimeProvider for SystemTimeProvider {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
