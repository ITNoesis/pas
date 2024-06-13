use crate::processor::DeltaTable;
use crate::processor::DELTATABLE;
use crate::DATA;

use anyhow::Result;
use chrono::{DateTime, Local};
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, FromRow, Pool};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PgStatBgWriterSum {
    pub checkpoints_timed: f64,
    pub checkpoints_req: f64,
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
            "pg_stat_bgwriter.checkpoints_timed",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.checkpoints_timed as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.checkpoints_req",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.checkpoints_req as f64,
        )
        .await;
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
            .get("pg_stat_bgwriter.checkpoints_req")
            .unwrap()
            .updated_value
        {
            DATA.pg_stat_bgwriter_sum.write().await.push_back((
                pg_stat_bgwriter.timestamp,
                PgStatBgWriterSum {
                    checkpoints_timed: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.checkpoints_timed")
                        .unwrap()
                        .delta_value,
                    checkpoints_req: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.checkpoints_req")
                        .unwrap()
                        .delta_value,
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
        //let pg_stat_bgwriter = PgStatBgWriter::query(pool).await;
        match PgStatBgWriter::query(pool).await {
            Ok(pg_stat_bgwriter) => {
                trace!("pg_stat_bgwriter: {:#?}", pg_stat_bgwriter);
                PgStatBgWriterSum::process_pg_bgwriter(pg_stat_bgwriter).await;
            }
            Err(error) => {
                warn!("Pool connection failed: {:?}", error);
            }
        }
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Result<PgStatBgWriter> {
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
        .await?;
        Ok(stat_bgwriter)
    }
}
