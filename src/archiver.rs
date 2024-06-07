use crate::processor::{
    PgDatabaseXidLimits, PgStatActivity, PgStatBgWriterSum, PgStatDatabaseSum, PgStatWalSum,
};
use crate::{DataTransit, ARGS, DATA};

use anyhow::{Context, Result};
use chrono::{DateTime, DurationRound, Local};
use std::{env::current_dir, fs::write};
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
    let low_time = high_time.duration_trunc(chrono::Duration::minutes(ARGS.archiver_interval))?;

    //println!("archiver: low_time: {}, high_time: {}", low_time, high_time);

    macro_rules! generate_transition_collections {
        ($([$category:ident, $struct:ident]),*) => {
            $(
            transition.$category = DATA
                .$category
                .read()
                .await
                .iter()
                .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
                .cloned()
                .collect::<Vec<(DateTime<Local>, $struct)>>();
            )*
        };
    }
    // pg_stat_activity contains a vector
    transition.pg_stat_activity = DATA
        .pg_stat_activity
        .read()
        .await
        .iter()
        .filter(|(ts, _)| *ts > low_time && *ts <= high_time)
        .cloned()
        .collect::<Vec<(DateTime<Local>, Vec<PgStatActivity>)>>();

    generate_transition_collections!(
        [pg_stat_database_sum, PgStatDatabaseSum],
        [pg_stat_bgwriter_sum, PgStatBgWriterSum],
        [pg_stat_wal_sum, PgStatWalSum],
        [pg_database_xid_limits, PgDatabaseXidLimits]
    );

    let current_directory = current_dir()?;
    let filename = current_directory.join(format!(
        "pas_{}-{}-{}T{}-{}.json",
        high_time.format("%Y"),
        high_time.format("%m"),
        high_time.format("%d"),
        high_time.format("%H"),
        high_time.format("%M"),
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
