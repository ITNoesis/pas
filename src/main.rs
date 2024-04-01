use anyhow::Result;
use sqlx::{FromRow, postgres::{types::PgInterval, PgConnection}, Connection};
//use chrono::{DateTime, Duration, Utc};
use std::time::Duration;

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use ratatui::{
    prelude::{CrosstermBackend, Stylize, Terminal, Text, Constraint}, style::{Color, Style}, widgets::{Paragraph, Cell, Row, Table}
};
use std::io::stdout;
use tokio::time::{self, MissedTickBehavior};


#[derive(Debug, FromRow)]
struct PgStatActivity {
    pid: i32,
    datname: Option<String>,
    usename: Option<String>,
    application_name: Option<String>,
    query_time: Option<PgInterval>, 
    state_time: Option<PgInterval>, 
    state: Option<String>,
    wait_state: Option<String>,
    backend_type: Option<String>,
    query_id: Option<String>,
    query: Option<String>,

}

#[tokio::main]
async fn main() -> Result<()> {

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    ////
    let mut connection = PgConnection::connect("postgres://fritshoogland@fritshoogland?host=/tmp/").await.expect("Error connecting to database");
    //let rows: Vec<PgStatActivity> = sqlx::query_as("select pid, datname, usename, application_name, query_start, state_change, state, wait_event_type, wait_event, backend_type, query_id, query from pg_stat_activity").fetch_all(&mut connection).await.expect("Error executiong query");

    //for row in rows {
    //    //println!("{} {} {}:{}", row.pid, row.state.unwrap_or("unkn".to_string()), row.wait_event_type.unwrap_or("CPU".to_string()), row.wait_event.unwrap_or("".to_string()));
    //    //row.into_iter.map(|r| r)
    //    let t = [ &row.pid.to_string(), &row.state.unwrap_or("unkn".to_string())];
    //    println!("{:?}", t);
    //}
    ////
    loop {
        let mut sql_rows: Vec<PgStatActivity> = sqlx::query_as("select pid, datname, usename, application_name, clock_timestamp()-query_start as query_time, clock_timestamp()-state_change as state_time, state, wait_event_type||':'||wait_event as wait_state, backend_type, query_id, query from pg_stat_activity where pid != pg_backend_pid() and state != 'idle'").fetch_all(&mut connection).await.expect("Error executiong query");
        sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        sql_rows.reverse();

        let header_style = Style::new().white().on_black();
        let header = ["pid", "datname", "usename", "application", "query time", "state time", "state", "wait state", "query id", "query"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = sql_rows
            .into_iter()
            .enumerate()
            .map(|(i, r)|{
                let color = match i % 2 {
                    0 => Style::new().fg(Color::Black).bg(Color::Rgb(170,170,170)),
                    _ => Style::new().fg(Color::Black).bg(Color::Rgb(192,192,192)),
                };
                let output = | microseconds: i64 | -> String {
                    match microseconds {
                        microseconds if microseconds < 1000 => format!("{} u", microseconds),
                        microseconds if microseconds < 1000_000 => { let microseconds_float = microseconds as f64; format!("{:.3} m", microseconds_float/1000_f64)},
                        microseconds => { let microseconds_float = microseconds as f64; format!("{:.3} s", microseconds_float/1000_000_f64)},
                    }
                };
                Row::new(
                    vec![
                        r.pid.to_string().clone(), 
                        r.datname.as_ref().unwrap_or(&"".to_string()).to_string(),
                        r.usename.as_ref().unwrap_or(&"".to_string()).to_string(),
                        r.application_name.as_ref().unwrap_or(&"".to_string()).to_string(),
                        //format!("{:?}", Duration::from_micros(r.query_time.as_ref().map_or(0_i64, |r| r.microseconds).try_into().unwrap())),
                        output(r.query_time.as_ref().map_or(0_i64, |r| r.microseconds)),
                        //format!("{:?}", Duration::from_micros(r.state_time.as_ref().map_or(0_i64, |r| r.microseconds).try_into().unwrap())),
                        output(r.state_time.as_ref().map_or(0_i64, |r| r.microseconds)),
                        r.state.as_ref().unwrap_or(&"".to_string()).to_string(),
                        r.wait_state.as_ref().map_or("ON CPU", |r| r).to_string(),
                        r.query_id.as_ref().unwrap_or(&"".to_string()).to_string(),
                        r.query.as_ref().unwrap_or(&"".to_string()).to_string(),
                    ])
                    .style(color)
                }) 
            .collect::<Vec<_>>();
            //.height(1);
                
        let widths = [Constraint::Length(6), Constraint::Length(14), Constraint::Length(14), Constraint::Length(10), Constraint::Length(10), Constraint::Length(10), Constraint::Length(10), Constraint::Length(25), Constraint::Length(12), Constraint::Length(20)];
        let table = Table::new(rows.clone(), widths)
           .header(header);
        //println!("{:#?}", header);
        //println!("{:#?}", table);
        //interval.tick().await;
        //break;
        terminal.draw(|frame| {
            let area = frame.size();
            frame.render_widget( table, area );
                //Paragraph::new("Hello Ratatui! (press 'q' to quit)")
                //    .white()
                //    .on_black(),
                //area,
        //);
        })?;

        


        ////
        if event::poll(std::time::Duration::from_secs(1))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && key.code == KeyCode::Char('q')
                {
                    break;
                }
            }
        }
    }
    ////
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
