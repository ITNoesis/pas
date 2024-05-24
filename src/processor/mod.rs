use crate::ARGS;
use crate::DATA;

use anyhow::Result;
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Local};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{types::Oid, PgPoolOptions},
    query_as, Executor, FromRow, Pool,
};
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::RwLock,
    time::{self, MissedTickBehavior},
};

pub mod pg_stat_activity;
pub mod wait_events;

pub use pg_stat_activity::PgStatActivity;
pub use wait_events::PgWaitTypeActivity;
pub use wait_events::PgWaitTypeBufferPin;
pub use wait_events::PgWaitTypeClient;
pub use wait_events::PgWaitTypes;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PgStatDatabaseSum {
    pub xact_commit_ps: f64,
    pub xact_rollback_ps: f64,
    pub blks_read_ps: f64,
    pub blks_hit_ps: f64,
    pub tup_returned_ps: f64,
    pub tup_fetched_ps: f64,
    pub tup_inserted_ps: f64,
    pub tup_updated_ps: f64,
    pub tup_deleted_ps: f64,
    pub blk_read_time_ps: f64,
    pub blk_write_time_ps: f64,
}

impl PgStatDatabaseSum {
    pub async fn process_pg_stat_database(pg_stat_database: Vec<PgStatDatabase>) {
        let pg_stat_database_timestamp = pg_stat_database.last().map(|r| r.timestamp).unwrap();
        DeltaTable::add_or_update(
            "pg_stat_database.xact_commit",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.xact_commit)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.xact_rollback",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.xact_rollback)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.blks_read",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.blks_read)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.blk_read_time",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.blk_read_time)
                .fold(0_f64, |sum, b| sum + b),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.blks_hit",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.blks_hit)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.blk_write_time",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.blk_write_time)
                .fold(0_f64, |sum, b| sum + b),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_returned",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_returned)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_fetched",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_fetched)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_inserted",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_inserted)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_updated",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_updated)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_deleted",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_deleted)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        // only add to DATA if updated_value is true, which means that there have been two
        // additions, and thus a DELTA (difference) is calculated.
        if DELTATABLE
            .read()
            .await
            .get("pg_stat_database.xact_commit")
            .unwrap()
            .updated_value
        {
            DATA.pg_stat_database_sum.write().await.push_back((
                pg_stat_database_timestamp,
                PgStatDatabaseSum {
                    xact_commit_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.xact_commit")
                        .unwrap()
                        .per_second_value,
                    xact_rollback_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.xact_rollback")
                        .unwrap()
                        .per_second_value,
                    blks_read_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.blks_read")
                        .unwrap()
                        .per_second_value,
                    blks_hit_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.blks_hit")
                        .unwrap()
                        .per_second_value,
                    tup_returned_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_returned")
                        .unwrap()
                        .per_second_value,
                    tup_fetched_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_fetched")
                        .unwrap()
                        .per_second_value,
                    tup_inserted_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_inserted")
                        .unwrap()
                        .per_second_value,
                    tup_updated_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_updated")
                        .unwrap()
                        .per_second_value,
                    tup_deleted_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_deleted")
                        .unwrap()
                        .per_second_value,
                    blk_read_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.blk_read_time")
                        .unwrap()
                        .per_second_value,
                    blk_write_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.blk_write_time")
                        .unwrap()
                        .per_second_value,
                },
            ));
        }
    }
}

// this pg_stat_database is consistent with postgres version 15
#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct PgStatDatabase {
    pub timestamp: DateTime<Local>,
    pub datid: Option<Oid>,
    pub datname: Option<String>,
    pub numbackends: i32,
    pub xact_commit: i64,
    pub xact_rollback: i64,
    pub blks_read: i64,
    pub blks_hit: i64,
    pub tup_returned: i64,
    pub tup_fetched: i64,
    pub tup_inserted: i64,
    pub tup_updated: i64,
    pub tup_deleted: i64,
    pub conflicts: i64,
    pub temp_files: i64,
    pub temp_bytes: i64,
    pub deadlocks: i64,
    pub checksum_failures: Option<i64>,
    pub checksum_last_failure: Option<DateTime<Local>>,
    pub blk_read_time: f64,
    pub blk_write_time: f64,
    pub session_time: f64,
    pub active_time: f64,
    pub idle_in_transaction_time: f64,
    pub sessions: i64,
    pub sessions_abandoned: i64,
    pub sessions_fatal: i64,
    pub sessions_killed: i64,
    pub stats_reset: Option<DateTime<Local>>,
}

impl PgStatDatabase {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_stat_database = PgStatDatabase::query(pool).await;
        PgStatDatabaseSum::process_pg_stat_database(pg_stat_database).await;
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgStatDatabase> {
        let stat_database: Vec<PgStatDatabase> = query_as(
            "
            select clock_timestamp() as timestamp,
                   datid, 
                   datname, 
                   numbackends,
                   xact_commit,
                   xact_rollback,
                   blks_read, 
                   blks_hit, 
                   tup_returned,
                   tup_fetched,
                   tup_inserted,
                   tup_updated, 
                   tup_deleted, 
                   conflicts, 
                   temp_files, 
                   temp_bytes,
                   deadlocks,
                   checksum_failures, 
                   checksum_last_failure,
                   blk_read_time,
                   blk_write_time, 
                   session_time, 
                   active_time,
                   idle_in_transaction_time,
                   sessions,
                   sessions_abandoned,
                   sessions_fatal,
                   sessions_killed,
                   stats_reset
             from  pg_stat_database 
        ",
        )
        .fetch_all(pool)
        .await
        .expect("error executing query");
        //sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        //sql_rows.reverse();
        stat_database
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PgStatWalSum {
    pub wal_records_ps: f64,
    pub wal_fpi_ps: f64,
    pub wal_bytes_ps: f64,
    pub wal_buffers_full_ps: f64,
    pub wal_write_ps: f64,
    pub wal_sync_ps: f64,
    pub wal_write_time_ps: f64,
    pub wal_sync_time_ps: f64,
}

impl PgStatWalSum {
    pub async fn process_pg_stat_wal(pg_stat_wal: PgStatWal) {
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_records",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_records as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_fpi",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_fpi as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_bytes",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_bytes.to_f64().unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_buffers_full",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_buffers_full as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_write",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_write as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_sync",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_sync as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_write_time",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_write_time,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_sync_time",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_sync_time,
        )
        .await;
        if DELTATABLE
            .read()
            .await
            .get("pg_stat_wal.wal_records")
            .unwrap()
            .updated_value
        {
            DATA.pg_stat_wal_sum.write().await.push_back((
                pg_stat_wal.timestamp,
                PgStatWalSum {
                    wal_records_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_records")
                        .unwrap()
                        .per_second_value,
                    wal_fpi_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_fpi")
                        .unwrap()
                        .per_second_value,
                    wal_bytes_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_bytes")
                        .unwrap()
                        .per_second_value,
                    wal_buffers_full_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_buffers_full")
                        .unwrap()
                        .per_second_value,
                    wal_write_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_write")
                        .unwrap()
                        .per_second_value,
                    wal_sync_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_sync")
                        .unwrap()
                        .per_second_value,
                    wal_write_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_write_time")
                        .unwrap()
                        .per_second_value,
                    wal_sync_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_sync_time")
                        .unwrap()
                        .per_second_value,
                },
            ));
        }
    }
}
// this pg_stat_wal is consistent with postgres version 15
#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct PgStatWal {
    pub timestamp: DateTime<Local>,
    pub wal_records: i64,
    pub wal_fpi: i64,
    pub wal_bytes: f64,
    pub wal_buffers_full: i64,
    pub wal_write: i64,
    pub wal_sync: i64,
    pub wal_write_time: f64,
    pub wal_sync_time: f64,
    pub stats_reset: Option<DateTime<Local>>,
}

impl PgStatWal {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_stat_wal = PgStatWal::query(pool).await;
        PgStatWalSum::process_pg_stat_wal(pg_stat_wal).await;
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> PgStatWal {
        let stat_wal: PgStatWal = query_as(
            "
            select clock_timestamp() as timestamp,
                   wal_records, 
                   wal_fpi, 
                   wal_bytes::double precision,
                   wal_buffers_full,
                   wal_write,
                   wal_sync, 
                   wal_write_time, 
                   wal_sync_time,
                   stats_reset
             from  pg_stat_wal 
        ",
        )
        .fetch_one(pool)
        .await
        .expect("error executing query");
        //sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        //sql_rows.reverse();
        stat_wal
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PgStatBgWriterSum {
    pub checkpoint_write_time_ps: f64,
    pub checkpoint_sync_time_ps: f64,
    pub buffers_checkpoint_ps: f64,
    pub buffers_clean_ps: f64,
    pub buffers_backend_ps: f64,
    pub buffers_backend_fsync_ps: f64,
    pub buffers_alloc_ps: f64,
}

impl PgStatBgWriterSum {
    pub async fn process_pg_bgwriter(pg_stat_bgwriter: PgStatBgWriter) {
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.checkpoint_write_time",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.checkpoint_write_time,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.checkpoint_sync_time",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.checkpoint_sync_time,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_checkpoint",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_checkpoint as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_clean",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_clean as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_backend",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_backend as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_backend_fsync",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_backend_fsync as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_alloc",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_alloc as f64,
        )
        .await;
        if DELTATABLE
            .read()
            .await
            .get("pg_stat_bgwriter.checkpoint_write_time")
            .unwrap()
            .updated_value
        {
            DATA.pg_stat_bgwriter_sum.write().await.push_back((
                pg_stat_bgwriter.timestamp,
                PgStatBgWriterSum {
                    checkpoint_write_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.checkpoint_write_time")
                        .unwrap()
                        .per_second_value,
                    checkpoint_sync_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.checkpoint_sync_time")
                        .unwrap()
                        .per_second_value,
                    buffers_checkpoint_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_checkpoint")
                        .unwrap()
                        .per_second_value,
                    buffers_clean_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_clean")
                        .unwrap()
                        .per_second_value,
                    buffers_backend_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_backend")
                        .unwrap()
                        .per_second_value,
                    buffers_backend_fsync_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_backend_fsync")
                        .unwrap()
                        .per_second_value,
                    buffers_alloc_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_alloc")
                        .unwrap()
                        .per_second_value,
                },
            ));
        }
    }
}

// this pg_stat_bgwriter is consistent with postgres version 15
#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct PgStatBgWriter {
    pub timestamp: DateTime<Local>,
    pub checkpoints_timed: i64,
    pub checkpoints_req: i64,
    pub checkpoint_write_time: f64,
    pub checkpoint_sync_time: f64,
    pub buffers_checkpoint: i64,
    pub buffers_clean: i64,
    pub maxwritten_clean: i64,
    pub buffers_backend: i64,
    pub buffers_backend_fsync: i64,
    pub buffers_alloc: i64,
    pub stats_reset: Option<DateTime<Local>>,
}

impl PgStatBgWriter {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_stat_bgwriter = PgStatBgWriter::query(pool).await;
        PgStatBgWriterSum::process_pg_bgwriter(pg_stat_bgwriter).await;
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> PgStatBgWriter {
        let stat_bgwriter: PgStatBgWriter = query_as(
            "
            select clock_timestamp() as timestamp,
                   checkpoints_timed, 
                   checkpoints_req, 
                   checkpoint_write_time,
                   checkpoint_sync_time,
                   buffers_checkpoint,
                   buffers_clean, 
                   maxwritten_clean, 
                   buffers_backend,
                   buffers_backend_fsync,
                   buffers_alloc,
                   stats_reset
             from  pg_stat_bgwriter 
        ",
        )
        .fetch_one(pool)
        .await
        .expect("error executing query");
        //sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        //sql_rows.reverse();
        stat_bgwriter
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PgDatabaseXidLimits {
    pub age_datfrozenxid: f64,
    pub age_datminmxid: f64,
    pub vacuum_failsafe_age: f64,
    pub autovacuum_freeze_max_age: f64,
    pub vacuum_freeze_table_age: f64,
    pub vacuum_freeze_min_age: f64,
    pub vacuum_multixact_failsafe_age: f64,
    pub autovacuum_multixact_freeze_max_age: f64,
    pub vacuum_multixact_freeze_table_age: f64,
    pub vacuum_multixact_freeze_min_age: f64,
}
impl PgDatabaseXidLimits {
    pub async fn process_pg_database(pg_database: Vec<PgDatabase>) {
        let pg_database_timestamp = pg_database.last().map(|r| r.timestamp).unwrap();

        DeltaTable::add_or_update(
            "pg_database.age_datfrozenxid",
            pg_database_timestamp,
            pg_database
                .iter()
                .map(|r| r.age_datfrozenxid)
                .max()
                .unwrap() as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_database.age_datminmxid",
            pg_database_timestamp,
            pg_database.iter().map(|r| r.age_datminmxid).max().unwrap() as f64,
        )
        .await;
        if DELTATABLE
            .read()
            .await
            .get("pg_database.age_datfrozenxid")
            .unwrap()
            .updated_value
        {
            DATA.pg_database_xid_limits.write().await.push_back((
                pg_database_timestamp,
                PgDatabaseXidLimits {
                    age_datfrozenxid: DELTATABLE
                        .read()
                        .await
                        .get("pg_database.age_datfrozenxid")
                        .unwrap()
                        .last_value,
                    age_datminmxid: DELTATABLE
                        .read()
                        .await
                        .get("pg_database.age_datminmxid")
                        .unwrap()
                        .last_value,
                    vacuum_failsafe_age: DELTATABLE
                        .read()
                        .await
                        .get("pg_settings.vacuum_failsafe_age")
                        .unwrap()
                        .last_value,
                    autovacuum_freeze_max_age: DELTATABLE
                        .read()
                        .await
                        .get("pg_settings.autovacuum_freeze_max_age")
                        .unwrap()
                        .last_value,
                    vacuum_freeze_table_age: DELTATABLE
                        .read()
                        .await
                        .get("pg_settings.vacuum_freeze_table_age")
                        .unwrap()
                        .last_value,
                    vacuum_freeze_min_age: DELTATABLE
                        .read()
                        .await
                        .get("pg_settings.vacuum_freeze_min_age")
                        .unwrap()
                        .last_value,
                    vacuum_multixact_failsafe_age: DELTATABLE
                        .read()
                        .await
                        .get("pg_settings.vacuum_multixact_failsafe_age")
                        .unwrap()
                        .last_value,
                    autovacuum_multixact_freeze_max_age: DELTATABLE
                        .read()
                        .await
                        .get("pg_settings.autovacuum_multixact_freeze_max_age")
                        .unwrap()
                        .last_value,
                    vacuum_multixact_freeze_table_age: DELTATABLE
                        .read()
                        .await
                        .get("pg_settings.vacuum_multixact_freeze_table_age")
                        .unwrap()
                        .last_value,
                    vacuum_multixact_freeze_min_age: DELTATABLE
                        .read()
                        .await
                        .get("pg_settings.vacuum_multixact_freeze_min_age")
                        .unwrap()
                        .last_value,
                },
            ));
        }
    }
}

// this pg_database is consistent with postgres version 15
#[derive(Debug, FromRow, Clone)]
pub struct PgDatabase {
    pub timestamp: DateTime<Local>,
    pub oid: Oid,
    pub datname: String,
    pub datdba: Oid,
    pub encoding: i32,
    pub datlocprovider: String,
    pub datistemplate: bool,
    pub datallowconn: bool,
    pub datconnlimit: i32,
    pub age_datfrozenxid: i32,
    pub age_datminmxid: i32,
    pub dattablespace: Oid,
    pub datcollate: String,
    pub datctype: String,
    pub daticulocale: Option<String>,
    pub datcollversion: Option<String>,
}

impl PgDatabase {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_database = PgDatabase::query(pool).await;
        PgDatabaseXidLimits::process_pg_database(pg_database).await;
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgDatabase> {
        let stat_database: Vec<PgDatabase> = query_as(
            "
            select clock_timestamp() as timestamp,
                   oid, 
                   datname, 
                   datdba,
                   encoding,
                   datlocprovider::text,
                   datistemplate, 
                   datallowconn, 
                   datconnlimit,
                   age(datfrozenxid) as age_datfrozenxid,
                   mxid_age(datminmxid) as age_datminmxid,
                   dattablespace, 
                   datcollate, 
                   datctype, 
                   daticulocale, 
                   datcollversion
             from  pg_database 
        ",
        )
        .fetch_all(pool)
        .await
        .expect("error executing query");
        stat_database
    }
}

// this pg_database is consistent with postgres version 15
#[derive(Debug, FromRow, Clone)]
pub struct PgSettings {
    pub timestamp: DateTime<Local>,
    pub name: String,
    pub setting: String,
    pub unit: Option<String>,
    pub category: String,
    pub boot_val: String,
    pub reset_val: String,
    pub sourcefile: Option<String>,
    pub sourceline: Option<i32>,
    pub pending_restart: bool,
}

impl PgSettings {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_settings = PgSettings::query(pool).await;
        PgSettings::add_to_deltatable(pg_settings).await;
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgSettings> {
        let pg_settings: Vec<PgSettings> = query_as(
            "
            select clock_timestamp() as timestamp,
                   name, 
                   setting, 
                   unit,
                   category,
                   boot_val,
                   reset_val, 
                   sourcefile, 
                   sourceline,
                   pending_restart
            from   pg_settings
            where  name in (
                   'autovacuum_freeze_max_age',
                   'autovacuum_multixact_freeze_max_age',
                   'vacuum_freeze_min_age',
                   'vacuum_freeze_table_age',
                   'vacuum_failsafe_age',
                   'vacuum_multixact_freeze_min_age',
                   'vacuum_multixact_freeze_table_age',
                   'vacuum_multixact_failsafe_age'
                   )
        ",
        )
        .fetch_all(pool)
        .await
        .expect("error executing query");
        pg_settings
    }
    async fn add_to_deltatable(pg_settings: Vec<PgSettings>) {
        let pg_settings_timestamp = pg_settings.last().map(|r| r.timestamp).unwrap();

        DeltaTable::add_or_update(
            "pg_settings.autovacuum_freeze_max_age",
            pg_settings_timestamp,
            *pg_settings
                .iter()
                .filter(|r| r.name == "autovacuum_freeze_max_age")
                .filter_map(|r| r.setting.parse::<f64>().ok())
                .collect::<Vec<f64>>()
                .first()
                .unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_settings.vacuum_freeze_min_age",
            pg_settings_timestamp,
            *pg_settings
                .iter()
                .filter(|r| r.name == "vacuum_freeze_min_age")
                .filter_map(|r| r.setting.parse::<f64>().ok())
                .collect::<Vec<f64>>()
                .first()
                .unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_settings.vacuum_freeze_table_age",
            pg_settings_timestamp,
            *pg_settings
                .iter()
                .filter(|r| r.name == "vacuum_freeze_table_age")
                .filter_map(|r| r.setting.parse::<f64>().ok())
                .collect::<Vec<f64>>()
                .first()
                .unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_settings.vacuum_failsafe_age",
            pg_settings_timestamp,
            *pg_settings
                .iter()
                .filter(|r| r.name == "vacuum_failsafe_age")
                .filter_map(|r| r.setting.parse::<f64>().ok())
                .collect::<Vec<f64>>()
                .first()
                .unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_settings.autovacuum_multixact_freeze_max_age",
            pg_settings_timestamp,
            *pg_settings
                .iter()
                .filter(|r| r.name == "autovacuum_multixact_freeze_max_age")
                .filter_map(|r| r.setting.parse::<f64>().ok())
                .collect::<Vec<f64>>()
                .first()
                .unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_settings.vacuum_multixact_freeze_min_age",
            pg_settings_timestamp,
            *pg_settings
                .iter()
                .filter(|r| r.name == "vacuum_multixact_freeze_min_age")
                .filter_map(|r| r.setting.parse::<f64>().ok())
                .collect::<Vec<f64>>()
                .first()
                .unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_settings.vacuum_multixact_freeze_table_age",
            pg_settings_timestamp,
            *pg_settings
                .iter()
                .filter(|r| r.name == "vacuum_multixact_freeze_table_age")
                .filter_map(|r| r.setting.parse::<f64>().ok())
                .collect::<Vec<f64>>()
                .first()
                .unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_settings.vacuum_multixact_failsafe_age",
            pg_settings_timestamp,
            *pg_settings
                .iter()
                .filter(|r| r.name == "vacuum_multixact_failsafe_age")
                .filter_map(|r| r.setting.parse::<f64>().ok())
                .collect::<Vec<f64>>()
                .first()
                .unwrap(),
        )
        .await;
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatisticsDelta {
    pub last_timestamp: DateTime<Local>,
    pub last_value: f64,
    pub delta_value: f64,
    pub per_second_value: f64,
    pub updated_value: bool,
}

type DeltaHashTable = RwLock<HashMap<String, StatisticsDelta>>;
static DELTATABLE: Lazy<DeltaHashTable> = Lazy::new(|| RwLock::new(HashMap::new()));

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

pub async fn processor_main() -> Result<()> {
    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                connection.execute("set application_name = 'PAS';").await?;
                Ok(())
            })
        })
        .connect(&ARGS.connection_string)
        .await
        .expect("Error creating connection pool");

    let mut interval = time::interval(Duration::from_secs(ARGS.interval));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        PgStatActivity::fetch_and_add_to_data(&pool).await;
        PgStatDatabase::fetch_and_add_to_data(&pool).await;
        PgStatBgWriter::fetch_and_add_to_data(&pool).await;
        PgStatWal::fetch_and_add_to_data(&pool).await;
        PgSettings::fetch_and_add_to_data(&pool).await;
        PgDatabase::fetch_and_add_to_data(&pool).await;
    }
}
