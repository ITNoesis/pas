use std::{env::current_dir, fs::write};

use crate::{
    processor::{
        PgCurrentWaitTypes, PgDatabaseXidLimits, PgStatBgWriterSum, PgStatDatabaseSum,
        PgStatWalSum, PgWaitTypeBufferPin, PgWaitTypeIO, PgWaitTypeIPC, PgWaitTypeLWLock,
        PgWaitTypeLock, PgWaitTypeTimeout,
    },
    ARGS,
};
use crate::{
    processor::{PgStatActivity, PgWaitTypeClient},
    DataTransit,
};
use crate::{
    processor::{PgWaitTypeActivity, PgWaitTypeExtension},
    DATA,
};

use anyhow::{Context, Result};
use chrono::{DateTime, DurationRound, Local};
use tokio::time::{interval, Duration, MissedTickBehavior};

pub async fn archiver_main() -> Result<()> {
    let mut interval = interval(Duration::from_secs(60));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut high_time = Local::now()
        .duration_trunc(chrono::Duration::minutes(ARGS.archiver_interval))?
        + chrono::Duration::minutes(ARGS.archiver_interval);

    loop {
        interval.tick().await;
        if Local::now() > high_time {
            match save_to_disk(high_time).await {
                Ok(_) => {}
                Err(error) => return Err(error),
            }
            high_time += chrono::Duration::minutes(ARGS.archiver_interval)
        };
    }
}

pub async fn save_to_disk(high_time: DateTime<Local>) -> Result<()> {
    let mut transition = DataTransit::default();
    let low_time = high_time - chrono::Duration::minutes(ARGS.archiver_interval);

    transition.pg_stat_activity = DATA
        .pg_stat_activity
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, Vec<PgStatActivity>)>>();
    transition.wait_event_types = DATA
        .wait_event_types
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgCurrentWaitTypes)>>();
    transition.wait_event_activity = DATA
        .wait_event_activity
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeActivity)>>();
    transition.wait_event_bufferpin = DATA
        .wait_event_bufferpin
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeBufferPin)>>();
    transition.wait_event_client = DATA
        .wait_event_client
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeClient)>>();
    transition.wait_event_extension = DATA
        .wait_event_extension
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeExtension)>>();
    transition.wait_event_io = DATA
        .wait_event_io
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeIO)>>();
    transition.wait_event_ipc = DATA
        .wait_event_ipc
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeIPC)>>();
    transition.wait_event_lock = DATA
        .wait_event_lock
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeLock)>>();
    transition.wait_event_lwlock = DATA
        .wait_event_lwlock
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeLWLock)>>();
    transition.wait_event_timeout = DATA
        .wait_event_timeout
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgWaitTypeTimeout)>>();
    transition.pg_stat_database_sum = DATA
        .pg_stat_database_sum
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgStatDatabaseSum)>>();
    transition.pg_stat_bgwriter_sum = DATA
        .pg_stat_bgwriter_sum
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgStatBgWriterSum)>>();
    transition.pg_stat_wal_sum = DATA
        .pg_stat_wal_sum
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgStatWalSum)>>();
    transition.pg_database_xid_limits = DATA
        .pg_database_xid_limits
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, PgDatabaseXidLimits)>>();

    let current_directory = current_dir()?;
    let filename = current_directory.join(format!(
        "pas_{}-{}-{}T{}-{}.json",
        low_time.format("%Y"),
        low_time.format("%m"),
        low_time.format("%d"),
        low_time.format("%H"),
        low_time.format("%M"),
    ));
    write(filename.clone(), serde_json::to_string(&transition)?).with_context(|| {
        format!(
            "Error writing {} to {}",
            filename.to_string_lossy(),
            current_directory.to_string_lossy()
        )
    })?;

    Ok(())
}
