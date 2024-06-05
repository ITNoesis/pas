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

            macro_rules! transition_loader {
                ($($category:ident),*) => {
                    $(
                    for record in transition.$category {
                        DATA.$category
                            .write()
                            .await
                            .push_back(record.clone())
                            .unwrap_or_default();
                    };
                    )*
                };
            }

            transition_loader!(
                pg_stat_activity,
                //wait_event_types,
                //wait_event_activity,
                //wait_event_bufferpin,
                //wait_event_client,
                //wait_event_extension,
                //wait_event_io,
                //wait_event_ipc,
                //wait_event_lock,
                //wait_event_lwlock,
                //wait_event_timeout,
                pg_stat_database_sum,
                pg_stat_bgwriter_sum,
                pg_stat_wal_sum,
                pg_database_xid_limits
            );
        }
    }
    Ok(())
}
