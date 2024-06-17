use crate::ARGS;
use crate::DATA;
use anyhow::{Context, Result};
use std::{fs::read_to_string, path::Path};

use crate::DataTransit;

pub async fn reader_main() -> Result<()> {
    for file in ARGS.read.clone().unwrap() {
        if Path::new(&file).exists() {
            let transition: DataTransit = serde_json::from_str(
                &read_to_string(file.clone())
                    .with_context(|| format!("Error reading file: {}", file))?,
            )
            .with_context(|| format!("Error reading JSON from: {}", file))?;

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
                pg_stat_database_sum,
                pg_stat_bgwriter_sum,
                pg_stat_wal_sum,
                pg_database_xid_limits
            );

            println!("✔ {}", &file);
        } else {
            println!("✘ {}", &file);
        }
    }
    println!("All files loaded.");

    Ok(())
}
