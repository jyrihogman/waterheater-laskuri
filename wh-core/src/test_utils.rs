use chrono::{DateTime, Utc};

use crate::time_provider::TimeProvider;

pub struct MockTimeProvider {
    mock_time: DateTime<Utc>,
}

impl MockTimeProvider {
    pub fn new(mock_time: DateTime<Utc>) -> Self {
        MockTimeProvider { mock_time }
    }
}

impl TimeProvider for MockTimeProvider {
    fn now(&self) -> DateTime<Utc> {
        self.mock_time
    }
}
