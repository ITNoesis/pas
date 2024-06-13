use crate::processor::DeltaTable;

use anyhow::Result;
use chrono::{DateTime, Local};
use log::{trace, warn};
use sqlx::{query_as, FromRow, Pool};

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
        match PgSettings::query(pool).await {
            Ok(pg_settings) => {
                trace!("pg_settings: {:#?}", pg_settings);
                PgSettings::add_to_deltatable(pg_settings).await;
            }
            Err(error) => {
                warn!("Pool connection failed: {:?}", error);
            }
        }
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Result<Vec<PgSettings>> {
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
        .await?;

        Ok(pg_settings)
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
