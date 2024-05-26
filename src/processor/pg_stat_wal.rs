use crate::processor::DeltaTable;
use crate::processor::DELTATABLE;
use crate::DATA;

use bigdecimal::ToPrimitive;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, FromRow, Pool};

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
