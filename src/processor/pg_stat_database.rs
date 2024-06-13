use crate::processor::DeltaTable;
use crate::processor::DELTATABLE;
use crate::DATA;

use anyhow::Result;
use chrono::{DateTime, Local};
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, FromRow, Pool};

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
    pub datid: Option<i32>,
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
        match PgStatDatabase::query(pool).await {
            Ok(pg_stat_database) => {
                trace!("pg_stat_database: {:#?}", pg_stat_database);
                PgStatDatabaseSum::process_pg_stat_database(pg_stat_database).await
            }
            Err(error) => {
                warn!("Pool connection failed: {:?}", error);
            }
        }
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Result<Vec<PgStatDatabase>> {
        let stat_database: Vec<PgStatDatabase> = query_as(
            "
            select clock_timestamp() as timestamp,
                   datid::text::int, 
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
        .await?;

        Ok(stat_database)
    }
}
