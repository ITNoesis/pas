use crate::processor::DeltaTable;
use crate::processor::DELTATABLE;
use crate::DATA;

use anyhow::Result;
use chrono::{DateTime, Local};
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, FromRow, Pool};

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
    pub oid: i32,
    pub datname: String,
    pub datdba: i32,
    pub encoding: i32,
    pub datlocprovider: String,
    pub datistemplate: bool,
    pub datallowconn: bool,
    pub datconnlimit: i32,
    pub age_datfrozenxid: i32,
    pub age_datminmxid: i32,
    pub dattablespace: i32,
    pub datcollate: String,
    pub datctype: String,
    pub daticulocale: Option<String>,
    pub datcollversion: Option<String>,
}

impl PgDatabase {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        match PgDatabase::query(pool).await {
            Ok(pg_database) => {
                trace!("pg_database: {:#?}", pg_database);
                PgDatabaseXidLimits::process_pg_database(pg_database).await;
            }
            Err(error) => {
                warn!("Pool connection failed: {:?}", error);
            }
        }
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Result<Vec<PgDatabase>> {
        let pg_database: Vec<PgDatabase> = query_as(
            "
            select clock_timestamp() as timestamp,
                   oid::text::int, 
                   datname, 
                   datdba::text::int,
                   encoding,
                   datlocprovider::text,
                   datistemplate, 
                   datallowconn, 
                   datconnlimit,
                   age(datfrozenxid) as age_datfrozenxid,
                   mxid_age(datminmxid) as age_datminmxid,
                   dattablespace::text::int, 
                   datcollate, 
                   datctype, 
                   daticulocale, 
                   datcollversion
             from  pg_database 
        ",
        )
        .fetch_all(pool)
        .await?;

        Ok(pg_database)
    }
}
