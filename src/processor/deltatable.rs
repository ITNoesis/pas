use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::processor::DELTATABLE;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatisticsDelta {
    pub last_timestamp: DateTime<Local>,
    pub last_value: f64,
    pub delta_value: f64,
    pub per_second_value: f64,
    pub updated_value: bool,
}

pub struct DeltaTable {}

impl DeltaTable {
    pub async fn add_or_update(name: &str, last_timestamp: DateTime<Local>, last_value: f64) {
        DELTATABLE
            .write()
            .await
            .entry(name.to_string())
            .and_modify(|r| {
                // if fetched timestamp doesn't make sense alias the fetch was invalid:
                if last_timestamp == r.last_timestamp {
                    r.updated_value = false;
                } else {
                    // if the statistics are reset
                    if r.last_value > last_value {
                        r.last_timestamp = last_timestamp;
                        r.last_value = last_value;
                        r.delta_value = 0_f64;
                        r.per_second_value = 0_f64;
                        r.updated_value = false;
                    } else {
                        // this is the normal situation after the insert, where we can calculate
                        // the delta, as well as the amount per second
                        r.delta_value = last_value - r.last_value;
                        // the per secon value is caluclated by dividing it by the number of
                        // milliseconds (not seconds), and then dividing it by 1000 to make it per
                        // second.
                        r.per_second_value = r.delta_value
                            / (last_timestamp
                                .signed_duration_since(r.last_timestamp)
                                .num_milliseconds() as f64
                                / 1000_f64);
                        r.last_value = last_value;
                        r.last_timestamp = last_timestamp;
                        r.updated_value = true;
                        if r.per_second_value.is_nan() {
                            r.per_second_value = 0_f64
                        }
                    }
                };
            })
            .or_insert(StatisticsDelta {
                last_timestamp,
                last_value,
                delta_value: 0_f64,
                per_second_value: 0_f64,
                updated_value: false,
            });
    }
}
