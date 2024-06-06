use anyhow::Result;
use axum::{extract::Path, response::Html, response::IntoResponse, routing::get, Router};
use image::{DynamicImage, ImageFormat};
use plotters::prelude::*;
use plotters::style::full_palette::{
    BLUE_600, BROWN, GREEN_800, GREY, LIGHTBLUE_300, PINK_A100, PURPLE, RED_900,
};
use std::io::Cursor;
use std::time::Duration;
use tokio::time::sleep;

use crate::{
    webserver::query::{show_queries_queryid_html, waits_by_query_id},
    ARGS, DATA,
};

mod io;
mod query;
mod wait_events;
mod wal;
mod xid_age;

pub use io::{io_bandwidth, io_times};
pub use query::{show_queries, show_queries_html};
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
        .route("/handler/:plot_1", get(handler_1_html))
        .route("/handler/:plot_1/:arg_1", get(handler_2_html))
        .route("/dual_handler/:plot_1/:out_1", get(dual_handler_html))
        .route(
            "/dual_handler/:plot_1/:out_1/:arg_1",
            get(dual_handler_html_queryid),
        )
        .route("/plotter/:plot_1/:queryid", get(handler_plotter))
        .route("/", get(root_handler));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", ARGS.webserver_port))
        .await
        .unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

pub async fn root_handler() -> Html<String> {
    loop {
        // wait until there is data inside DATA
        if DATA.pg_stat_database_sum.read().await.iter().count() > 0 {
            break;
        } else {
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
     <li><a href="/handler/ash_wait_type" target="right">ASH by wait type</a></li>
     <li><a href="/handler/ash_wait_event" target="right">ASH by wait event</a></li>
     <li><a href="/dual_handler/ash_wait_query/all_queries" target="right">ASH and Queries</a></li>
     <li><a href="/handler/wal_io_times" target="right">WAL latency</a></li>
     <li><a href="/handler/wal_size" target="right">WAL size</a></li>
     <li><a href="/handler/io_latency" target="right">IO latency</a></li>
     <li><a href="/handler/io_bandwidth" target="right">IO bandwidth</a></li>
     <li><a href="/handler/xid_age" target="right">XID Age</a></li>
     <li><a href="/handler/we_qid_q" target="right">waits QueryID-Q</a></li>
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
//     <li><a href="/handler/sh_qid_q" target="right">ASH-QueryID-Q</a></li>
//     <li><a href="/handler/sh_qid" target="right">ASH-QueryID time</a></li>
//     <li><a href="/dual_handler/sh_qid/all_queries" target="right">ASH-QueryID-Q-HTML</a></li>

pub async fn handler_1_html(Path(plot_1): Path<String>) -> Html<String> {
    format!(r#"<img src="/plotter/{}/x">"#, plot_1).into()
}
pub async fn handler_2_html(Path((plot_1, arg_1)): Path<(String, String)>) -> Html<String> {
    format!(r#"<img src="/plotter/{}/{}">"#, plot_1, arg_1).into()
}
pub async fn dual_handler_html(Path((plot_1, out_1)): Path<(String, String)>) -> Html<String> {
    let output: String = format!(r#"<img src="/plotter/{}/x">"#, plot_1);
    let html = match out_1.as_str() {
        "all_queries" => show_queries_html(),
        &_ => todo!(),
    };
    format!("{}{}", output, html).into()
}
pub async fn dual_handler_html_queryid(
    Path((plot_1, out_1, queryid)): Path<(String, String, String)>,
) -> Html<String> {
    let output: String = format!(r#"<img src="/plotter/{}/{}">"#, plot_1, queryid);
    let html = match out_1.as_str() {
        "all_queries" => show_queries_queryid_html(&queryid.parse::<i64>().unwrap()),
        &_ => todo!(),
    };
    format!("{}{}", output, html).into()
}

pub async fn handler_plotter(Path((plot_1, queryid)): Path<(String, String)>) -> impl IntoResponse {
    let mut buffer = vec![
        0;
        (ARGS.graph_width * ARGS.graph_height * 3)
            .try_into()
            .unwrap()
    ];
    match plot_1.as_str() {
        "ash_wait_type" => create_ash_wait_type_plot(&mut buffer),
        "ash_wait_event" => create_ash_wait_event_plot(&mut buffer),
        "ash_wait_query" => create_ash_wait_event_and_queryid_overview(&mut buffer),
        "wal_io_times" => create_wait_event_type_and_wal_io_plot(&mut buffer),
        "wal_size" => create_wait_event_type_and_wal_size_plot(&mut buffer),
        "io_latency" => create_wait_event_type_and_io_latency_plot(&mut buffer),
        "io_bandwidth" => create_wait_event_type_and_io_bandwidth_plot(&mut buffer),
        "xid_age" => create_xid_age_plot(&mut buffer),
        //"sh_qid" => create_wait_event_type_and_queryid_time(&mut buffer),
        //"sh_qid_q" => create_wait_event_type_and_queryid_and_query(&mut buffer),
        "we_qid_q" => create_wait_events_and_queryid_and_query(&mut buffer),
        //"sh_qid_html" => create_wait_event_and_queryid_and_query_html(&mut buffer),
        "ash_wait_query_by_queryid" => create_awp(&mut buffer, queryid),
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

pub fn create_awp(buffer: &mut [u8], queryid: String) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_plot(
        &mut multi_backend,
        0,
        &true,
        &queryid.parse::<i64>().unwrap(),
    );
    waits_by_query_id(
        &mut multi_backend,
        1,
        &true,
        &queryid.parse::<i64>().unwrap(),
    );
}
pub fn create_ash_wait_type_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    wait_event_type_plot(&mut multi_backend, 0);
}
pub fn create_ash_wait_event_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    wait_event_plot(&mut multi_backend, 0, &false, &0_i64);
}
pub fn create_ash_wait_event_and_queryid_overview(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_plot(&mut multi_backend, 0, &false, &0_i64);
    waits_by_query_id(&mut multi_backend, 1, &false, &0_i64);
}
pub fn create_xid_age_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    xid_age(&mut multi_backend, 0);
}
/*
pub fn create_wait_event_type_and_queryid_and_query(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((3, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    ash_by_query_id(&mut multi_backend, 1);
    show_queries(&mut multi_backend, 2);
}
*/
pub fn create_wait_events_and_queryid_and_query(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((3, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    waits_by_query_id(&mut multi_backend, 1, &false, &0_i64);
    show_queries(&mut multi_backend, 2);
}
/*
pub fn create_wait_event_and_queryid_and_query_html(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_plot(&mut multi_backend, 0, &false, &0_i64);
    ash_by_query_id(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_queryid_time(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    ash_by_query_id(&mut multi_backend, 1);
}
*/
pub fn create_wait_event_type_and_io_bandwidth_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    io_bandwidth(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_io_latency_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    io_times(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_wal_io_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wal_io_times(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_wal_size_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wal_size(&mut multi_backend, 1);
}
