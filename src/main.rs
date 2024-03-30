use anyhow::Result;
use sqlx::{FromRow, postgres::PgConnection, Connection};
use chrono::{DateTime, Utc};

#[derive(Debug, FromRow)]
struct PgStatActivity {
    pid: i32,
    datname: Option<String>,
    usename: Option<String>,
    application_name: Option<String>,
    query_start: Option<DateTime<Utc>>, 
    state_change: Option<DateTime<Utc>>, 
    state: Option<String>,
    wait_event_type: Option<String>,
    wait_event: Option<String>,
    backend_type: Option<String>,
    query_id: Option<String>,
    query: Option<String>,

}

#[tokio::main]
async fn main() -> Result<()> {
   let mut connection = PgConnection::connect("postgres://fritshoogland@fritshoogland?host=/tmp/").await.expect("Error connecting to database");
    let rows: Vec<PgStatActivity> = sqlx::query_as("select pid, datname, usename, application_name, query_start, state_change, state, wait_event_type, wait_event, backend_type, query_id, query from pg_stat_activity").fetch_all(&mut connection).await.expect("Error executiong query");

    for row in rows {
        println!("{} {} {}:{}", row.pid, row.state.unwrap_or("unkn".to_string()), row.wait_event_type.unwrap_or("CPU".to_string()), row.wait_event.unwrap_or("".to_string()));
    }

    Ok(())
}
