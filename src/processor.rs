use crate::ARGS;
use crate::DATA;

use std::time::Duration;
use tokio::time::{self, MissedTickBehavior};
use anyhow::Result;
use sqlx::{query_as, Pool, FromRow, postgres::{types::PgInterval, PgPoolOptions}};
use chrono::Local;

#[derive(Debug, FromRow, Clone)]
pub struct PgStatActivity {
    pub pid: i32,
    pub datname: Option<String>,
    pub usename: Option<String>,
    pub application_name: Option<String>,
    pub query_time: Option<PgInterval>, 
    pub state_time: Option<PgInterval>, 
    pub state: Option<String>,
    pub wait_event_type: Option<String>,
    pub wait_event: Option<String>,
    pub backend_type: Option<String>,
    pub query_id: Option<i64>,
    pub query: Option<String>,
}

impl PgStatActivity {
    pub async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgStatActivity> {
        let mut sql_rows: Vec<PgStatActivity> = query_as("
            select pid, 
                   datname, 
                   usename, 
                   application_name, 
                   clock_timestamp()-query_start as query_time, 
                   clock_timestamp()-state_change as state_time, 
                   state, 
                   wait_event_type,
                   wait_event,
                   backend_type, 
                   query_id, 
                   query 
             from  pg_stat_activity 
             where pid != pg_backend_pid() 
        ").fetch_all(pool).await.expect("error executing query");
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
    pub async fn process_pg_stat_activity(pg_stat_activity: Vec<PgStatActivity>) -> PgCurrentWaitTypes {
        let on_cpu = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.is_none())
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let activity = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Activity")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let buffer_pin = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "BufferPin")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let client = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Client")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let extension = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Extension")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let io = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Io")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let ipc = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "IPC")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let lock = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Lock")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let lwlock = pg_stat_activity.iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "LWLock")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let timeout = pg_stat_activity.iter()
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

pub async fn processor_main() -> Result<()> {
    
    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .connect("postgres://fritshoogland@fritshoogland?host=/tmp/")
        .await
        .expect("Error creating connection pool");

    let mut interval = time::interval(Duration::from_secs(ARGS.interval));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        println!("tick");
        let current_timestamp = Local::now();
        let pg_stat_activity = PgStatActivity::query(&pool).await;
        DATA.pg_stat_activity.write().await.push_back((current_timestamp, pg_stat_activity.clone()));
        DATA.wait_event_types.write().await.push_back((current_timestamp, PgCurrentWaitTypes::process_pg_stat_activity(pg_stat_activity).await));
    }
}
