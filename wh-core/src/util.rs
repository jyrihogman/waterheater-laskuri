use chrono::{Duration, NaiveDate, Timelike, Utc};
use chrono_tz::Europe::Helsinki;

pub fn get_storage_date() -> NaiveDate {
    let utc_now = Utc::now();

    let helsinki_time = utc_now.with_timezone(&Helsinki);

    if helsinki_time.hour() < 14 {
        return helsinki_time.date_naive() - Duration::days(1);
    }

    helsinki_time.date_naive()
}
