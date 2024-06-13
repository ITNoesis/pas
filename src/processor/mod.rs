use crate::{
    processor::{
        pg_database::PgDatabase, pg_settings::PgSettings, pg_stat_bgwriter::PgStatBgWriter,
        pg_stat_database::PgStatDatabase, pg_stat_wal::PgStatWal,
    },
    ARGS,
};

use anyhow::Result;
use log::{debug, info, warn};
use once_cell::sync::Lazy;
use sqlx::{postgres::PgPoolOptions, Executor};
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::RwLock,
    time::{self, MissedTickBehavior},
};

pub mod deltatable;
pub mod pg_database;
pub mod pg_settings;
pub mod pg_stat_activity;
pub mod pg_stat_bgwriter;
pub mod pg_stat_database;
pub mod pg_stat_wal;

pub use deltatable::{DeltaTable, StatisticsDelta};
pub use pg_database::PgDatabaseXidLimits;
pub use pg_stat_activity::PgStatActivity;
pub use pg_stat_bgwriter::PgStatBgWriterSum;
pub use pg_stat_database::PgStatDatabaseSum;
pub use pg_stat_wal::PgStatWalSum;

type DeltaHashTable = RwLock<HashMap<String, StatisticsDelta>>;
static DELTATABLE: Lazy<DeltaHashTable> = Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn processor_main() -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(ARGS.interval));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    info!("Setup database connectionpool.");
    // loop until connection pool becomes available
    let pool = loop {
        match PgPoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .after_connect(|connection, _| {
                Box::pin(async move {
                    connection.execute("set application_name = 'PAS';").await?;
                    Ok(())
                })
            })
            .connect(&ARGS.connection_string)
            .await
        {
            Ok(pool) => {
                info!("Database connectionpool created.");
                break pool;
            }
            Err(error) => {
                warn!(
                    "Database connectionpool creation failed, error: {:?}, retrying",
                    error
                );
                interval.tick().await;
            }
        };
    };

    loop {
        interval.tick().await;
        debug!("tick!");

        PgStatActivity::fetch_and_add_to_data(&pool).await;
        PgStatDatabase::fetch_and_add_to_data(&pool).await;
        PgStatBgWriter::fetch_and_add_to_data(&pool).await;
        PgStatWal::fetch_and_add_to_data(&pool).await;
        PgSettings::fetch_and_add_to_data(&pool).await;
        PgDatabase::fetch_and_add_to_data(&pool).await;
    }
}
