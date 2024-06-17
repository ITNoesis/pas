use anyhow::Result;
use axum::{extract::Path, response::Html, response::IntoResponse, routing::get, Router};
use image::{DynamicImage, ImageFormat};
use io::iops;
use log::debug;
use plotters::prelude::*;
use plotters::style::full_palette::{
    BLUE_600, BROWN, GREEN_800, GREY, LIGHTBLUE_300, PINK_A100, PURPLE, RED_900,
};
use std::io::Cursor;
use std::time::Duration;
use tokio::time::sleep;

use crate::{
    webserver::query::{
        show_queries_query_html, show_queries_queryid_html, waits_by_query_id, waits_by_query_text,
    },
    ARGS, DATA,
};

mod io;
mod query;
mod transactions;
mod tuples;
mod wait_events;
mod wal;
mod xid_age;

pub use io::{io_bandwidth, io_times};
pub use query::{show_queries, show_queries_html};
pub use transactions::transactions;
pub use tuples::tuples_processed;
pub use wait_events::{wait_event_plot, wait_event_type_plot};
pub use wal::{wal_io_times, wal_size};
pub use xid_age::xid_age;

pub fn wait_type_color(wait_event_type: &str) -> RGBColor {
    match wait_event_type {
        "activity" => PURPLE,
        "buffer_pin" => LIGHTBLUE_300,
        "client" => GREY,
        "extension" => GREEN_800,
        "timeout" => BROWN,
        "ipc" => PINK_A100,
        "lwlock" => RED_900,
        "lock" => RED,
        "io" => BLUE_600,
        "on_cpu" | "~on_cpu" => GREEN,
        other => {
            println!("unknown wait event type: {:?}", other);
            todo!()
        }
    }
}

pub async fn webserver_main() -> Result<()> {
    let app = Router::new()
        .route("/handler/:plot_1/:show_clientread", get(handler_1_html))
        .route(
            "/handler/:plot_1/:arg_1/:show_clientread",
            get(handler_2_html),
        )
        .route(
            "/dual_handler/:plot_1/:out_1/:show_clientread",
            get(dual_handler_html),
        )
        .route(
            "/dual_handler/:plot_1/:out_1/:arg_1/:show_clientread",
            get(dual_handler_html_queryid),
        )
        .route(
            "/plotter/:plot_1/:queryid/:show_clientread",
            get(handler_plotter),
        )
        .route("/", get(root_handler));
    let listener =
        tokio::net::TcpListener::bind(format!("0.0.0.0:{}", ARGS.webserver_port)).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

pub async fn root_handler() -> Html<String> {
    loop {
        // wait until there is data inside DATA
        if DATA.pg_stat_database_sum.read().await.iter().count() > 0 {
            debug!("Records found in DATA.pg_stat_database_sum, continue.");
            break;
        } else {
            debug!("No records found in DATA.pg_stat_database_sum, sleeping and retry...");
            sleep(Duration::from_secs(1)).await;
        }
    }

    r##"<!doctype html>
 <html>
   <head>
   <style>
    .column_left{ 
        width: 10%; 
        float:left; 
    }
    .column_right{ 
        width: 90%; 
        height: 3000px; 
        float:right; 
    }
   </style>
  </head>
  <body>
  <div class = "container">
   <div class = "column_left">
    <nav>
     <li><a href="/" target="right">Home</a></li>
     <li><a href="/handler/ash_wait_type/Y" target="right">ASH by wait type</a></li>
     <li><a href="/handler/ash_wait_event/Y" target="right">ASH by wait event</a></li>
     <li><a href="/dual_handler/ash_wait_query/all_queries/Y" target="right">ASH and Queries</a></li>
     <li><a href="/handler/wal_io_times/x" target="right">WAL latency</a></li>
     <li><a href="/handler/wal_size/x" target="right">WAL size</a></li>
     <li><a href="/handler/io_latency/x" target="right">IO latency</a></li>
     <li><a href="/handler/io_bandwidth/x" target="right">IO bandwidth</a></li>
     <li><a href="/handler/iops/x" target="right">IOPS</a></li>
     <li><a href="/handler/xid_age/x" target="right">XID Age</a></li>
     <li><a href="/handler/transactions/Y" target="right">Transactions</a></li>
     <li><a href="/handler/tuples/Y" target="right">Tuples</a></li>
     <li><a href="/handler/ash_wait_type/N" target="right">ASH by wait type (no clientread)</a></li>
     <li><a href="/handler/ash_wait_event/N" target="right">ASH by wait event (no clientread)</a></li>
     <li><a href="/dual_handler/ash_wait_query/all_queries/N" target="right">ASH and Queries (no clientread)</a></li>
     <li><a href="/handler/transactions/N" target="right">Transactions (no clientread)</a></li>
     <li><a href="/handler/tuples/N" target="right">Tuples (no clientread)</a></li>
    </nav>
   </div>
   <div class = "column_right">
    <iframe name="right" id="right" width="100%" height="100%">
   </div>
  </div>
  </body>
 </html>
 "##
    .to_string()
    .into()
}

pub async fn handler_1_html(
    Path((plot_1, show_clientread)): Path<(String, String)>,
) -> Html<String> {
    format!(r#"<img src="/plotter/{}/x/{}">"#, plot_1, show_clientread).into()
}
pub async fn handler_2_html(
    Path((plot_1, arg_1, show_clientread)): Path<(String, String, String)>,
) -> Html<String> {
    format!(
        r#"<img src="/plotter/{}/{}/{}">"#,
        plot_1, arg_1, show_clientread
    )
    .into()
}
pub async fn dual_handler_html(
    Path((plot_1, out_1, show_clientread)): Path<(String, String, String)>,
) -> Html<String> {
    let output: String = format!(r#"<img src="/plotter/{}/x/{}">"#, plot_1, show_clientread);
    let html = match out_1.as_str() {
        "all_queries" => show_queries_html(show_clientread),
        &_ => todo!(),
    };
    format!("{}{}", output, html).into()
}
pub async fn dual_handler_html_queryid(
    Path((plot_1, out_1, queryid, show_clientread)): Path<(String, String, String, String)>,
) -> Html<String> {
    debug!(
        "dual_handler: plot_1: {}, out_1: {}, queryid: {}, show_clientread: {}",
        plot_1, out_1, queryid, show_clientread
    );
    let output: String = format!(
        r#"<img src="/plotter/{}/{}/{}">"#,
        plot_1, queryid, show_clientread
    );
    let html = match out_1.as_str() {
        "all_queries" => {
            show_queries_queryid_html(&queryid.parse::<i64>().unwrap(), show_clientread)
        }
        "selected_queries" => show_queries_query_html(&queryid, show_clientread),
        &_ => todo!(),
    };
    format!("{}{}", output, html).into()
}

pub async fn handler_plotter(
    Path((plot_1, queryid, show_clientread)): Path<(String, String, String)>,
) -> impl IntoResponse {
    debug!(
        "handler_plotter: plot_1: {}, queryid: {}, show_clientread: {}",
        plot_1, queryid, show_clientread
    );
    let mut buffer = vec![
        0;
        (ARGS.graph_width * ARGS.graph_height * 3)
            .try_into()
            .unwrap()
    ];
    let remove_clientread = if show_clientread.as_str() == "Y" {
        false
    } else {
        true
    };
    match plot_1.as_str() {
        "ash_wait_type" => create_ash_wait_type_plot(&mut buffer, remove_clientread),
        "ash_wait_event" => create_ash_wait_event_plot(&mut buffer, remove_clientread),
        "ash_wait_query" => {
            create_ash_wait_event_and_queryid_overview(&mut buffer, remove_clientread)
        }
        "wal_io_times" => create_wait_event_type_and_wal_io_plot(&mut buffer),
        "wal_size" => create_wait_event_type_and_wal_size_plot(&mut buffer),
        "io_latency" => create_wait_event_type_and_io_latency_plot(&mut buffer),
        "io_bandwidth" => create_wait_event_type_and_io_bandwidth_plot(&mut buffer),
        "iops" => create_iops_plot(&mut buffer),
        "xid_age" => create_xid_age_plot(&mut buffer),
        "transactions" => create_wait_event_and_transactions_plot(&mut buffer, remove_clientread),
        "tuples" => create_wait_event_and_tuples_plot(&mut buffer, remove_clientread),
        "we_qid_q" => create_wait_events_and_queryid_and_query(&mut buffer, remove_clientread),
        "ash_wait_query_by_queryid" => {
            create_ash_wait_query_by_queryid(&mut buffer, queryid, remove_clientread)
        }
        "ash_wait_query_by_query" => {
            create_ash_wait_query_by_query(&mut buffer, queryid, remove_clientread)
        }
        unknown => {
            println!("handler plotter: unknown request: {}", unknown);
            todo!()
        }
    }
    let rgb_image = DynamicImage::ImageRgb8(
        image::RgbImage::from_raw(ARGS.graph_width, ARGS.graph_height, buffer).unwrap(),
    );
    let mut cursor = Cursor::new(Vec::new());
    rgb_image.write_to(&mut cursor, ImageFormat::Png).unwrap();
    cursor.into_inner()
}

pub fn create_ash_wait_query_by_queryid(
    buffer: &mut [u8],
    queryid: String,
    remove_clientread: bool,
) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_plot(
        &mut multi_backend,
        0,
        &true,
        &queryid.parse::<i64>().unwrap(),
        &false,
        "",
        remove_clientread,
    );
    waits_by_query_text(
        &mut multi_backend,
        1,
        &true,
        &queryid.parse::<i64>().unwrap(),
        remove_clientread,
    );
}
pub fn create_ash_wait_query_by_query(buffer: &mut [u8], query: String, remove_clientread: bool) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    wait_event_plot(
        &mut multi_backend,
        0,
        &false,
        &0_i64,
        &true,
        &query,
        remove_clientread,
    );
}
pub fn create_ash_wait_type_plot(buffer: &mut [u8], remove_clientread: bool) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    wait_event_type_plot(&mut multi_backend, 0, remove_clientread);
}
pub fn create_ash_wait_event_plot(buffer: &mut [u8], remove_clientread: bool) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    wait_event_plot(
        &mut multi_backend,
        0,
        &false,
        &0_i64,
        &false,
        "",
        remove_clientread,
    );
}
pub fn create_ash_wait_event_and_queryid_overview(buffer: &mut [u8], remove_clientread: bool) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_plot(
        &mut multi_backend,
        0,
        &false,
        &0_i64,
        &false,
        "",
        remove_clientread,
    );
    waits_by_query_id(&mut multi_backend, 1, &false, &0_i64, remove_clientread);
}
pub fn create_wait_event_and_transactions_plot(buffer: &mut [u8], remove_clientread: bool) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_plot(
        &mut multi_backend,
        0,
        &false,
        &0_i64,
        &false,
        "",
        remove_clientread,
    );
    transactions(&mut multi_backend, 1);
}
pub fn create_wait_event_and_tuples_plot(buffer: &mut [u8], remove_clientread: bool) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_plot(
        &mut multi_backend,
        0,
        &false,
        &0_i64,
        &false,
        "",
        remove_clientread,
    );
    tuples_processed(&mut multi_backend, 1);
}
pub fn create_xid_age_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    xid_age(&mut multi_backend, 0);
}
pub fn create_wait_events_and_queryid_and_query(buffer: &mut [u8], remove_clientread: bool) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((3, 1));
    wait_event_type_plot(&mut multi_backend, 0, remove_clientread);
    waits_by_query_id(&mut multi_backend, 1, &false, &0_i64, remove_clientread);
    show_queries(&mut multi_backend, 2, remove_clientread);
}
pub fn create_wait_event_type_and_io_bandwidth_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_plot(&mut multi_backend, 0, &false, &0_i64, &false, "", true);
    io_bandwidth(&mut multi_backend, 1);
}
pub fn create_iops_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    //wait_event_plot(&mut multi_backend, 0, &false, &0_i64, &false, "");
    iops(&mut multi_backend, 0);
}
pub fn create_wait_event_type_and_io_latency_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0, true);
    io_times(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_wal_io_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0, true);
    wal_io_times(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_wal_size_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0, true);
    wal_size(&mut multi_backend, 1);
}
