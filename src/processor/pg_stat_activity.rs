use crate::DATA;
use anyhow::Result;
use chrono::{DateTime, Local};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, FromRow, Pool};

// this pg_stat_activity is consistent with postgres version 15
#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct PgStatActivity {
    pub timestamp: DateTime<Local>,
    pub datid: Option<i32>,
    pub datname: Option<String>,
    pub pid: i32,
    pub leader_pid: Option<i32>,
    pub usesysid: Option<i32>,
    pub usename: Option<String>,
    pub application_name: Option<String>,
    pub client_addr: Option<String>,
    pub client_hostname: Option<String>,
    pub client_port: Option<i32>,
    pub backend_time: Option<i64>,
    pub xact_time: Option<i64>,
    pub query_time: Option<i64>,
    pub state_time: Option<i64>,
    pub wait_event_type: Option<String>,
    pub wait_event: Option<String>,
    pub state: Option<String>,
    pub backend_xid: Option<i32>,
    pub backend_xmin: Option<i32>,
    pub query_id: Option<i64>,
    pub query: Option<String>,
    pub backend_type: Option<String>,
}

impl PgStatActivity {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        match PgStatActivity::query(pool).await {
            Ok(pg_stat_activity) => {
                trace!("pg_stat_activity: {:#?}", pg_stat_activity);
                let current_timestamp = Local::now();
                DATA.pg_stat_activity
                    .write()
                    .await
                    .push_back((current_timestamp, pg_stat_activity.clone()));
            }
            Err(_) => {
                debug!("Pool connection failed.");
            }
        }
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Result<Vec<PgStatActivity>> {
        let mut sql_rows: Vec<PgStatActivity> = query_as(
            "
            select clock_timestamp() as timestamp,
                   datid::text::int, 
                   datname, 
                   pid,
                   leader_pid,
                   usesysid::text::int,
                   usename, 
                   application_name, 
                   client_addr::text,
                   client_hostname,
                   client_port,
                   cast(extract(epoch from (clock_timestamp()-backend_start)) as bigint) as backend_time,
                   cast(extract(epoch from (clock_timestamp()-xact_start)) as bigint) as xact_time,
                   cast(extract(epoch from (clock_timestamp()-query_start)) as bigint) as query_time,
                   cast(extract(epoch from (clock_timestamp()-state_change)) as bigint) as state_time,
                   lower(wait_event_type) as wait_event_type,
                   lower(wait_event) as wait_event,
                   state, 
                   backend_xid::text::int,
                   backend_xmin::text::int,
                   query_id, 
                   query, 
                   backend_type 
             from  pg_stat_activity 
             where pid != pg_backend_pid() 
        ",
        )
        .fetch_all(pool)
        .await?;
        sql_rows.sort_by_key(|a| *a.query_time.as_ref().unwrap_or(&0_i64));
        sql_rows.reverse();

        Ok(sql_rows)
    }
}
