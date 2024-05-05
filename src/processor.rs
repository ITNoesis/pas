use crate::ARGS;
use crate::DATA;

use anyhow::Result;
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Local};
use once_cell::sync::Lazy;
use sqlx::{
    postgres::{types::Oid, types::PgInterval, PgPoolOptions},
    query_as,
    types::BigDecimal,
    FromRow, Pool,
};
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::RwLock,
    time::{self, MissedTickBehavior},
};

// this pg_stat_activity is consistent with postgres version 15
#[derive(Debug, FromRow, Clone)]
pub struct PgStatActivity {
    pub timestamp: DateTime<Local>,
    pub datid: Option<Oid>,
    pub datname: Option<String>,
    pub pid: i32,
    pub leader_pid: Option<i32>,
    pub usesysid: Option<Oid>,
    pub usename: Option<String>,
    pub application_name: Option<String>,
    pub client_addr: Option<String>,
    pub client_hostname: Option<String>,
    pub client_port: Option<i32>,
    pub backend_time: Option<PgInterval>,
    pub xact_time: Option<PgInterval>,
    pub query_time: Option<PgInterval>,
    pub state_time: Option<PgInterval>,
    pub wait_event_type: Option<String>,
    pub wait_event: Option<String>,
    pub state: Option<String>,
    pub backend_xid: Option<i64>,
    pub backend_xmin: Option<i64>,
    pub query_id: Option<i64>,
    pub query: Option<String>,
    pub backend_type: Option<String>,
}

impl PgStatActivity {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_stat_activity = PgStatActivity::query(pool).await;
        let current_timestamp = Local::now();
        DATA.pg_stat_activity
            .write()
            .await
            .push_back((current_timestamp, pg_stat_activity.clone()));
        DATA.wait_event_types.write().await.push_back((
            current_timestamp,
            PgCurrentWaitTypes::process_pg_stat_activity(pg_stat_activity).await,
        ));
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgStatActivity> {
        let mut sql_rows: Vec<PgStatActivity> = query_as(
            "
            select clock_timestamp() as timestamp,
                   datid, 
                   datname, 
                   pid,
                   leader_pid,
                   usesysid,
                   usename, 
                   application_name, 
                   client_addr,
                   client_hostname,
                   client_port,
                   clock_timestamp()-backend_start as backend_time, 
                   clock_timestamp()-xact_start as xact_time, 
                   clock_timestamp()-query_start as query_time, 
                   clock_timestamp()-state_change as state_time, 
                   wait_event_type,
                   wait_event,
                   state, 
                   backend_xid,
                   backend_xmin,
                   query_id, 
                   query, 
                   backend_type 
             from  pg_stat_activity 
             where pid != pg_backend_pid() 
        ",
        )
        .fetch_all(pool)
        .await
        .expect("error executing query");
        sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        sql_rows.reverse();
        sql_rows
    }
}

#[derive(Debug)]
pub struct PgCurrentWaitTypes {
    pub on_cpu: usize,
    pub activity: usize,
    pub buffer_pin: usize,
    pub client: usize,
    pub extension: usize,
    pub io: usize,
    pub ipc: usize,
    pub lock: usize,
    pub lwlock: usize,
    pub timeout: usize,
}

impl PgCurrentWaitTypes {
    pub async fn process_pg_stat_activity(
        pg_stat_activity: Vec<PgStatActivity>,
    ) -> PgCurrentWaitTypes {
        let on_cpu = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.is_none())
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let activity = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Activity")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let buffer_pin = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "BufferPin")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let client = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Client")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let extension = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Extension")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let io = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Io")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let ipc = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "IPC")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let lock = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Lock")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let lwlock = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "LWLock")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let timeout = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Timeout")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        PgCurrentWaitTypes {
            on_cpu,
            activity,
            buffer_pin,
            client,
            extension,
            io,
            ipc,
            lock,
            lwlock,
            timeout,
        }
    }
}

//
#[derive(Debug)]
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
#[derive(Debug, FromRow, Clone)]
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

#[derive(Debug)]
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
#[derive(Debug, FromRow, Clone)]
pub struct PgStatWal {
    pub timestamp: DateTime<Local>,
    pub wal_records: i64,
    pub wal_fpi: i64,
    pub wal_bytes: BigDecimal,
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
                   wal_bytes,
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

#[derive(Debug)]
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
#[derive(Debug, FromRow, Clone)]
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

#[derive(Debug)]
pub struct PgDatabaseXidLimits {
    pub age_datfronzenxid: f64,
    pub age_datminmxid: f64,
}
impl PgDatabaseXidLimits {}

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
    pub async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgDatabase> {
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
        //sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        //sql_rows.reverse();
        stat_database
    }
}

#[derive(Debug, Default)]
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
        .connect("postgres://frits.hoogland@frits.hoogland?host=/tmp/")
        .await
        .expect("Error creating connection pool");

    let mut interval = time::interval(Duration::from_secs(ARGS.interval));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        println!("tick");
        PgStatActivity::fetch_and_add_to_data(&pool).await;
        PgStatDatabase::fetch_and_add_to_data(&pool).await;
        PgStatBgWriter::fetch_and_add_to_data(&pool).await;
        PgStatWal::fetch_and_add_to_data(&pool).await;
        let pg_database = PgDatabase::query(&pool).await;
        DeltaTable::add_or_update(
            "pg_database.age_datfrozenxid",
            pg_database.first().map(|r| r.timestamp).unwrap(),
            pg_database
                .iter()
                .map(|r| r.age_datfrozenxid)
                .max()
                .unwrap() as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_database.age_datminmxid",
            pg_database.first().map(|r| r.timestamp).unwrap(),
            pg_database.iter().map(|r| r.age_datminmxid).max().unwrap() as f64,
        )
        .await;
        //println!(
        //    "{:#?}",
        //    pg_database.iter().map(|r| r.age_datfrozenxid).max()
        //);
        //println!("{:#?}", DELTATABLE.read().await);
        println!("{:?}", DATA.pg_stat_database_sum.read().await);
    }
}
