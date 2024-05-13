use crate::ARGS;
use crate::DATA;
use anyhow::Result;
use std::{fs::read_to_string, path::Path};

use crate::DataTransit;

pub async fn reader_main() -> Result<()> {
    let file_names = ARGS.read.clone().unwrap().as_str().to_string();
    for file in file_names.split(',') {
        if Path::new(&file).exists() {
            let transition: DataTransit =
                serde_json::from_str(&read_to_string(file).unwrap()).unwrap();
            for record in transition.pg_stat_activity {
                DATA.pg_stat_activity
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_types {
                DATA.wait_event_types
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_activity {
                DATA.wait_event_activity
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_bufferpin {
                DATA.wait_event_bufferpin
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_client {
                DATA.wait_event_client
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_extension {
                DATA.wait_event_extension
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_io {
                DATA.wait_event_io
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_ipc {
                DATA.wait_event_ipc
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_lock {
                DATA.wait_event_lock
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_lwlock {
                DATA.wait_event_lwlock
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.wait_event_timeout {
                DATA.wait_event_timeout
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.pg_stat_database_sum {
                DATA.pg_stat_database_sum
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.pg_stat_bgwriter_sum {
                DATA.pg_stat_bgwriter_sum
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.pg_stat_wal_sum {
                DATA.pg_stat_wal_sum
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
            for record in transition.pg_database_xid_limits {
                DATA.pg_database_xid_limits
                    .write()
                    .await
                    .push_back(record.clone())
                    .unwrap_or_default();
            }
        }
    }
    Ok(())
}
