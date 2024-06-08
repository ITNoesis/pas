use anyhow::Result;
use chrono::Local;
use futures::executor::block_on;
use pas::ARGS;
use std::time::Duration;

/*
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
*/

use pas::archiver::{archiver_main, save_to_disk};
use pas::processor::processor_main;
use pas::reader::reader_main;
use pas::webserver::webserver_main;
use std::process;
use tokio::time;

#[tokio::main]
async fn main() -> Result<()> {
    println!("PAS starting.");
    /*
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    ////
    let mut connection = PgConnection::connect("postgres://fritshoogland@fritshoogland?host=/tmp/").await.expect("Error connecting to database");
    ////
    loop {
        let mut sql_rows: Vec<PgStatActivity> = sqlx::query_as(
            "select pid,
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
             from pg_stat_activity
             where pid != pg_backend_pid()
             and state != 'idle'"
        ).fetch_all(&mut connection).await.expect("Error executiong query");
        sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        sql_rows.reverse();

        let header_style = Style::new().black().on_white();
        let header = ["pid", "datname", "usename", "application name", "query time", "state time", "state", "wait state", "query id", "query"]
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
                    0 => Style::new().fg(Color::White).bg(Color::Black),
                    _ => Style::new().fg(Color::White).bg(Color::Rgb(85,85,85)),
                };
                let output = | microseconds: i64 | -> String {
                    let unaligned = match microseconds {
                        microseconds if microseconds < 1000 => format!("{} u", microseconds),
                        microseconds if microseconds < 1000_000 => { let microseconds_float = microseconds as f64; format!("{:.3} m", microseconds_float/1000_f64)},
                        microseconds => { let microseconds_float = microseconds as f64; format!("{:.3} s", microseconds_float/1000_000_f64)},
                    };
                    // poor mans right aligning
                    format!("{}{}", " ".repeat(10-unaligned.len()), unaligned)
                };
                Row::new({
                    let wait_event = r.wait_event.as_ref().unwrap_or(&"".to_string()).to_string();
                    vec![
                        r.pid.to_string().clone(),
                        r.datname.as_ref().unwrap_or(&"".to_string()).to_string(),
                        r.usename.as_ref().unwrap_or(&"".to_string()).to_string(),
                        r.application_name.as_ref().unwrap_or(&"".to_string()).to_string(),
                        output(r.query_time.as_ref().map_or(0_i64, |r| r.microseconds)),
                        output(r.state_time.as_ref().map_or(0_i64, |r| r.microseconds)),
                        r.state.as_ref().unwrap_or(&"".to_string()).to_string(),
                        r.wait_event_type.as_ref().map_or("ON CPU".to_string(), |r| format!("{}:{}", r, wait_event)),
                        r.query_id.as_ref().unwrap_or(&"".to_string()).to_string(),
                        r.query.as_ref().unwrap_or(&"".to_string()).to_string(),
                    ]})
                    .style(color)
                })
            .collect::<Vec<_>>();

        let widths = [
            Constraint::Length(6),    // pid
            Constraint::Length(14),   // datname
            Constraint::Length(14),   // usename
            Constraint::Length(16),   // application_name
            Constraint::Length(12),   // query_time
            Constraint::Length(12),   // state_time
            Constraint::Length(10),   // state
            Constraint::Length(25),   // wait_state
            Constraint::Length(12),   // query_id
            Constraint::Length(40),   // query
        ];
        let table = Table::new(rows.clone(), widths)
           .header(header);

        terminal.draw(|frame| {
            let area = frame.size();
            frame.render_widget( table, area );
        })?;

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
    */
    ctrlc::set_handler(move || {
        println!("SIGINT received, terminating.");
        let mut return_value = 0;
        if ARGS.archiver {
            match block_on(save_to_disk(Local::now())) {
                Ok(_) => {}
                Err(error) => {
                    return_value = 1;
                    eprintln!("{:?}", error);
                }
            }
        }
        process::exit(return_value);
    })
    .unwrap();

    if ARGS.read.is_none() {
        tokio::spawn(async move {
            match processor_main().await {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{:?}", error);
                    process::exit(1);
                }
            }
        });
    };

    if ARGS.webserver {
        println!("PAS webserver started.");
        tokio::spawn(async move {
            match webserver_main().await {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{:?}", error);
                    process::exit(1);
                }
            }
        });
    };

    if ARGS.archiver {
        println!("PAS archiver started.");
        tokio::spawn(async move {
            match archiver_main().await {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{:?}", error);
                    process::exit(1);
                }
            }
        });
    };

    if ARGS.read.is_some() {
        tokio::spawn(async move {
            match reader_main().await {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{:?}", error);
                    process::exit(1);
                }
            }
        });
    };

    println!("PAS running.");
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
    }

    //Ok(())
}
