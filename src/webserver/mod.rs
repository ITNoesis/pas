use anyhow::Result;
use axum::{extract::Path, response::Html, response::IntoResponse, routing::get, Router};
use futures::executor;
use human_bytes::human_bytes;
use image::{DynamicImage, ImageFormat};
use plotters::prelude::*;
use plotters::style::full_palette::{
    BLUE_200, BLUE_600, BROWN, GREEN_200, GREEN_800, GREY, LIGHTBLUE_300, ORANGE, ORANGE_200,
    PINK_A100, PURPLE, PURPLE_200, RED_200, RED_900,
};
use plotters::style::Palette99;
use std::collections::HashMap;
use std::io::Cursor;
use std::time::Duration;
use tokio::time::sleep;

use plotters::backend::RGBPixel;
use plotters::chart::SeriesLabelPosition::{LowerRight, UpperLeft};
use plotters::coord::Shift;

use crate::{
    CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE,
    LABEL_AREA_SIZE_BOTTOM, LABEL_AREA_SIZE_LEFT, LABEL_AREA_SIZE_RIGHT, MESH_STYLE_FONT,
    MESH_STYLE_FONT_SIZE,
};

use crate::{ARGS, DATA};

pub async fn webserver_main() -> Result<()> {
    let app = Router::new()
        .route("/handler/:plot_1", get(handler_html))
        .route("/plotter/:plot_1", get(handler_plotter))
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
        if DATA.wait_event_types.read().await.iter().count() > 0 {
            break;
        } else {
            sleep(Duration::from_secs(1)).await;
        }
    }

    //    .container {{ }}

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
     <li><a href="/handler/sh" target="right">ASH</a></li>
     <li><a href="/handler/sh_activity" target="right">ASH-activity</a></li>
     <li><a href="/handler/sh_bufferpin" target="right">ASH-bufferpin</a></li>
     <li><a href="/handler/sh_client" target="right">ASH-client</a></li>
     <li><a href="/handler/sh_extension" target="right">ASH-extension</a></li>
     <li><a href="/handler/sh_io" target="right">ASH-io</a></li>
     <li><a href="/handler/sh_ipc" target="right">ASH-ipc</a></li>
     <li><a href="/handler/sh_lock" target="right">ASH-lock</a></li>
     <li><a href="/handler/sh_lwlock" target="right">ASH-lwlock</a></li>
     <li><a href="/handler/sh_timeout" target="right">ASH-timeout</a></li>
     <li><a href="/handler/wal_io_times" target="right">WAL latency</a></li>
     <li><a href="/handler/wal_size" target="right">WAL size</a></li>
     <li><a href="/handler/io_latency" target="right">IO latency</a></li>
     <li><a href="/handler/io_bandwidth" target="right">IO bandwidth</a></li>
     <li><a href="/handler/sh_qid" target="right">ASH-QueryID time</a></li>
     <li><a href="/handler/sh_qid_q" target="right">ASH-QueryID-Q</a></li>
     <li><a href="/handler/xid_age" target="right">XID Age</a></li>
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

pub async fn handler_html(Path(plot_1): Path<String>) -> Html<String> {
    format!(r#"<img src="/plotter/{}">"#, plot_1).into()
}

pub async fn handler_plotter(Path(plot_1): Path<String>) -> impl IntoResponse {
    let mut buffer = vec![
        0;
        (ARGS.graph_width * ARGS.graph_height * 3)
            .try_into()
            .unwrap()
    ];
    match plot_1.as_str() {
        "sh" => create_wait_event_type_plot(&mut buffer),
        "sh_activity" => create_wait_event_type_and_activity_plot(&mut buffer),
        "sh_bufferpin" => create_wait_event_type_and_bufferpin_plot(&mut buffer),
        "sh_client" => create_wait_event_type_and_client_plot(&mut buffer),
        "sh_extension" => create_wait_event_type_and_extension_plot(&mut buffer),
        "sh_io" => create_wait_event_type_and_io_plot(&mut buffer),
        "sh_ipc" => create_wait_event_type_and_ipc_plot(&mut buffer),
        "sh_lock" => create_wait_event_type_and_lock_plot(&mut buffer),
        "sh_lwlock" => create_wait_event_type_and_lwlock_plot(&mut buffer),
        "sh_timeout" => create_wait_event_type_and_timeout_plot(&mut buffer),
        "wal_io_times" => create_wait_event_type_and_wal_io_plot(&mut buffer),
        "wal_size" => create_wait_event_type_and_wal_size_plot(&mut buffer),
        "io_latency" => create_wait_event_type_and_io_latency_plot(&mut buffer),
        "io_bandwidth" => create_wait_event_type_and_io_bandwidth_plot(&mut buffer),
        "sh_qid" => create_wait_event_type_and_queryid_time(&mut buffer),
        "sh_qid_q" => create_wait_event_type_and_queryid_and_query(&mut buffer),
        "xid_age" => create_xid_age_plot(&mut buffer),
        &_ => todo!(),
    }
    let rgb_image = DynamicImage::ImageRgb8(
        image::RgbImage::from_raw(ARGS.graph_width, ARGS.graph_height, buffer).unwrap(),
    );
    let mut cursor = Cursor::new(Vec::new());
    rgb_image.write_to(&mut cursor, ImageFormat::Png).unwrap();
    cursor.into_inner()
}

pub fn create_wait_event_type_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    wait_event_type_plot(&mut multi_backend, 0);
}
pub fn create_xid_age_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    xid_age(&mut multi_backend, 0);
}
pub fn create_wait_event_type_and_queryid_and_query(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((3, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    ash_by_query_id(&mut multi_backend, 1);
    show_queries(&mut multi_backend, 2);
}
pub fn create_wait_event_type_and_queryid_time(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    ash_by_query_id(&mut multi_backend, 1);
}
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
pub fn create_wait_event_type_and_activity_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_activity(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_bufferpin_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_bufferpin(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_client_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_client(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_extension_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_extension(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_io_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_io(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_ipc_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_ipc(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_lock_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_lock(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_lwlock_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_lwlock(&mut multi_backend, 1);
}
pub fn create_wait_event_type_and_timeout_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height))
        .into_drawing_area();
    let mut multi_backend = backend.split_evenly((2, 1));
    wait_event_type_plot(&mut multi_backend, 0);
    wait_type_timeout(&mut multi_backend, 1);
}

pub fn wait_event_type_plot(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let wait_event_type = executor::block_on(DATA.wait_event_types.read());
    let start_time = wait_event_type
        .iter()
        .map(|(timestamp, _)| timestamp)
        .min()
        .unwrap();
    let end_time = wait_event_type
        .iter()
        .map(|(timestamp, _)| timestamp)
        .max()
        .unwrap();
    let low_value_f64 = 0_f64;
    let high_value = wait_event_type
        .iter()
        .map(|(_, w)| {
            w.on_cpu
                + w.activity
                + w.buffer_pin
                + w.client
                + w.extension
                + w.io
                + w.ipc
                + w.lock
                + w.lwlock
                + w.timeout
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = wait_event_type
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Active sessions",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            wait_event_type
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.on_cpu as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let min_activity = wait_event_type
        .iter()
        .map(|(_, w)| w.activity)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_activity = wait_event_type
        .iter()
        .map(|(_, w)| w.activity)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_activity: usize = wait_event_type.iter().map(|(_, w)| w.activity).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    (w.on_cpu
                        + w.io
                        + w.lock
                        + w.lwlock
                        + w.ipc
                        + w.timeout
                        + w.extension
                        + w.client
                        + w.buffer_pin
                        + w.activity) as f64,
                )
            }),
            0.0,
            PURPLE,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "Activity",
            min_activity,
            max_activity,
            wait_event_type.back().map_or(0, |(_, r)| r.activity),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_activity as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE.filled()));
    //
    let min_buffer_pin = wait_event_type
        .iter()
        .map(|(_, w)| w.buffer_pin)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_buffer_pin = wait_event_type
        .iter()
        .map(|(_, w)| w.buffer_pin)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_buffer_pin: usize = wait_event_type.iter().map(|(_, w)| w.buffer_pin).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    (w.on_cpu
                        + w.io
                        + w.lock
                        + w.lwlock
                        + w.ipc
                        + w.timeout
                        + w.extension
                        + w.client
                        + w.buffer_pin) as f64,
                )
            }),
            0.0,
            LIGHTBLUE_300,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "BufferPin",
            min_buffer_pin,
            max_buffer_pin,
            wait_event_type.back().map_or(0, |(_, r)| r.buffer_pin),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_buffer_pin as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| {
            Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], LIGHTBLUE_300.filled())
        });
    //
    let min_client = wait_event_type
        .iter()
        .map(|(_, w)| w.client)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_client = wait_event_type
        .iter()
        .map(|(_, w)| w.client)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_client: usize = wait_event_type.iter().map(|(_, w)| w.client).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    (w.on_cpu
                        + w.io
                        + w.lock
                        + w.lwlock
                        + w.ipc
                        + w.timeout
                        + w.extension
                        + w.client) as f64,
                )
            }),
            0.0,
            GREY,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "Client",
            min_client,
            max_client,
            wait_event_type.back().map_or(0, |(_, r)| r.client),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_client as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREY.filled()));
    //
    let min_extension = wait_event_type
        .iter()
        .map(|(_, w)| w.extension)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_extension = wait_event_type
        .iter()
        .map(|(_, w)| w.extension)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_extension: usize = wait_event_type.iter().map(|(_, w)| w.extension).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc + w.timeout + w.extension) as f64,
                )
            }),
            0.0,
            GREEN_800,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "Extension",
            min_extension,
            max_extension,
            wait_event_type.back().map_or(0, |(_, r)| r.extension),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_extension as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN_800.filled()));
    //
    let min_timeout = wait_event_type
        .iter()
        .map(|(_, w)| w.timeout)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_timeout = wait_event_type
        .iter()
        .map(|(_, w)| w.timeout)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_timeout: usize = wait_event_type.iter().map(|(_, w)| w.timeout).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc + w.timeout) as f64,
                )
            }),
            0.0,
            BROWN,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "Timeout",
            min_timeout,
            max_timeout,
            wait_event_type.back().map_or(0, |(_, r)| r.timeout),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_timeout as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BROWN.filled()));
    //
    let min_ipc = wait_event_type
        .iter()
        .map(|(_, w)| w.ipc)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_ipc = wait_event_type
        .iter()
        .map(|(_, w)| w.ipc)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_ipc: usize = wait_event_type.iter().map(|(_, w)| w.ipc).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc) as f64,
                )
            }),
            0.0,
            PINK_A100,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "IPC",
            min_ipc,
            max_ipc,
            wait_event_type.back().map_or(0, |(_, r)| r.ipc),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_ipc as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PINK_A100.filled()));
    //
    let min_lwlock = wait_event_type
        .iter()
        .map(|(_, w)| w.lwlock)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_lwlock = wait_event_type
        .iter()
        .map(|(_, w)| w.lwlock)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_lwlock: usize = wait_event_type.iter().map(|(_, w)| w.lwlock).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type
                .iter()
                .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock + w.lwlock) as f64)),
            0.0,
            RED_900,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "LWLock",
            min_lwlock,
            max_lwlock,
            wait_event_type.back().map_or(0, |(_, r)| r.lwlock),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_lwlock as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED_900.filled()));
    //
    let min_lock = wait_event_type
        .iter()
        .map(|(_, w)| w.lock)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_lock = wait_event_type
        .iter()
        .map(|(_, w)| w.lock)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_lock: usize = wait_event_type.iter().map(|(_, w)| w.lock).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type
                .iter()
                .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock) as f64)),
            0.0,
            RED,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "Lock",
            min_lock,
            max_lock,
            wait_event_type.back().map_or(0, |(_, r)| r.lock),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_lock as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED.filled()));
    //
    let min_io = wait_event_type
        .iter()
        .map(|(_, w)| w.io)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_io = wait_event_type
        .iter()
        .map(|(_, w)| w.io)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_io: usize = wait_event_type.iter().map(|(_, w)| w.io).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type
                .iter()
                .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io) as f64)),
            0.0,
            BLUE_600,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "IO",
            min_io,
            max_io,
            wait_event_type.back().map_or(0, |(_, r)| r.io),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_io as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE_600.filled()));
    //
    let min_on_cpu = wait_event_type
        .iter()
        .map(|(_, w)| w.on_cpu)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_on_cpu = wait_event_type
        .iter()
        .map(|(_, w)| w.on_cpu)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let sum_on_cpu: usize = wait_event_type.iter().map(|(_, w)| w.on_cpu).sum();
    contextarea
        .draw_series(AreaSeries::new(
            wait_event_type
                .iter()
                .map(|(timestamp, w)| (*timestamp, w.on_cpu as f64)),
            0.0,
            GREEN,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10} {:10} {:10} {:10.2}",
            "On CPU",
            min_on_cpu,
            max_on_cpu,
            wait_event_type.back().map_or(0, |(_, r)| r.on_cpu),
            if sum_all_activity == 0 {
                0_f64
            } else {
                (sum_on_cpu as f64 / sum_all_activity as f64) * 100_f64
            },
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_activity(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_activity.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| {
            w.archivermain
                + w.autovacuummain
                + w.bgwriterhibernate
                + w.bgwritermain
                + w.checkpointermain
                + w.logicalapplymain
                + w.logicallaunchermain
                + w.logicalparallelapplymain
                + w.recoverywalstream
                + w.sysloggermain
                + w.walreceivermain
                + w.walsendermain
                + w.other
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type Activity",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.archivermain as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%",
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(
        archivermain,
        bgwriterhibernate,
        bgwritermain,
        checkpointermain,
        logicalapplymain,
        logicallaunchermain,
        logicalparallelapplymain,
        recoverywalstream,
        sysloggermain,
        walreceivermain,
        walsendermain,
        other,
    );

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_bufferpin(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_bufferpin.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| w.bufferpin + w.other)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type BufferPin",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.bufferpin as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(bufferpin, other);

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_client(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_client.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| {
            w.clientread
                + w.clientwrite
                + w.gssopenserver
                + w.libpqwalreceiverconnect
                + w.libpqwalreceiverreceive
                + w.sslopenserver
                + w.walsenderwaitforwal
                + w.walsenderwritedata
                + w.other
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type Client",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.clientread as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(
        clientread,
        clientwrite,
        gssopenserver,
        libpqwalreceiverconnect,
        libpqwalreceiverreceive,
        sslopenserver,
        walsenderwaitforwal,
        walsenderwritedata,
        other,
    );

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_extension(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_extension.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| w.extension + w.other)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type Extension",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.extension as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(extension, other);

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_io(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_io.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| {
            w.basebackupread
                + w.basebackupsync
                + w.basebackupwrite
                + w.buffileread
                + w.buffiletruncate
                + w.buffilewrite
                + w.controlfileread
                + w.controlfilesync
                + w.controlfilesyncupdate
                + w.controlfilewrite
                + w.controlfilewriteupdate
                + w.copyfileread
                + w.copyfilewrite
                + w.dsmallocate
                + w.dsmfillzerowrite
                + w.datafileextend
                + w.datafileflush
                + w.datafileimmediatesync
                + w.datafileprefetch
                + w.datafileread
                + w.datafilesync
                + w.datafiletruncate
                + w.datafilewrite
                + w.lockfileaddtodatadirread
                + w.lockfileaddtodatadirsync
                + w.lockfileaddtodatadirwrite
                + w.lockfilecreateread
                + w.lockfilecreatesync
                + w.lockfilecreatewrite
                + w.lockfilerecheckdatadirread
                + w.logicalrewritecheckpointsync
                + w.logicalrewritemappingsync
                + w.logicalrewritemappingwrite
                + w.logicalrewritesync
                + w.logicalrewritetruncate
                + w.logicalrewritewrite
                + w.relationmapread
                + w.relationmapreplace
                + w.relationmapwrite
                + w.reorderbufferread
                + w.reorderbufferwrite
                + w.reorderlogicalmappingread
                + w.replicationslotread
                + w.replicationslotrestoresync
                + w.replicationslotsync
                + w.replicationslotwrite
                + w.slruflushsync
                + w.slruread
                + w.slrusync
                + w.slruwrite
                + w.snapbuildread
                + w.snapbuildsync
                + w.snapbuildwrite
                + w.timelinehistoryfilesync
                + w.timelinehistoryfilewrite
                + w.timelinehistoryread
                + w.timelinehistorysync
                + w.timelinehistorywrite
                + w.twophasefileread
                + w.twophasefilesync
                + w.twophasefilewrite
                + w.versionfilesync
                + w.versionfilewrite
                + w.walbootstrapsync
                + w.walbootstrapwrite
                + w.walcopyread
                + w.walcopysync
                + w.walcopywrite
                + w.walinitsync
                + w.walinitwrite
                + w.walread
                + w.walsendertimelinehistoryread
                + w.walsync
                + w.walsyncmethodassign
                + w.walwrite
                + w.other
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type IO",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.basebackupread as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(
        basebackupread,
        basebackupsync,
        basebackupwrite,
        buffileread,
        buffiletruncate,
        buffilewrite,
        controlfileread,
        controlfilesync,
        controlfilesyncupdate,
        controlfilewrite,
        controlfilewriteupdate,
        copyfileread,
        copyfilewrite,
        dsmallocate,
        dsmfillzerowrite,
        datafileextend,
        datafileflush,
        datafileimmediatesync,
        datafileprefetch,
        datafileread,
        datafilesync,
        datafiletruncate,
        datafilewrite,
        lockfileaddtodatadirread,
        lockfileaddtodatadirsync,
        lockfileaddtodatadirwrite,
        lockfilecreateread,
        lockfilecreatesync,
        lockfilecreatewrite,
        lockfilerecheckdatadirread,
        logicalrewritecheckpointsync,
        logicalrewritemappingsync,
        logicalrewritemappingwrite,
        logicalrewritesync,
        logicalrewritetruncate,
        logicalrewritewrite,
        relationmapread,
        relationmapreplace,
        relationmapwrite,
        reorderbufferread,
        reorderbufferwrite,
        reorderlogicalmappingread,
        replicationslotread,
        replicationslotrestoresync,
        replicationslotsync,
        replicationslotwrite,
        slruflushsync,
        slruread,
        slrusync,
        slruwrite,
        snapbuildread,
        snapbuildsync,
        snapbuildwrite,
        timelinehistoryfilesync,
        timelinehistoryfilewrite,
        timelinehistoryread,
        timelinehistorysync,
        timelinehistorywrite,
        twophasefileread,
        twophasefilesync,
        twophasefilewrite,
        versionfilesync,
        versionfilewrite,
        walbootstrapsync,
        walbootstrapwrite,
        walcopyread,
        walcopysync,
        walcopywrite,
        walinitsync,
        walinitwrite,
        walread,
        walsendertimelinehistoryread,
        walsync,
        walsyncmethodassign,
        walwrite,
        other,
    );

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_ipc(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_ipc.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| {
            w.appendready
                + w.archivecleanupcommand
                + w.archivecommand
                + w.backendtermination
                + w.backupwaitwalarchive
                + w.bgworkershutdown
                + w.bgworkerstartup
                + w.btreepage
                + w.bufferio
                + w.checkpointdone
                + w.checkpointstart
                + w.executegather
                + w.hashbatchallocate
                + w.hashbatchelect
                + w.hashbatchload
                + w.hashbuildallocate
                + w.hashbuildelect
                + w.hashbuildhashinner
                + w.hashbuildhashouter
                + w.hashgrowbatchesdecide
                + w.hashgrowbatcheselect
                + w.hashgrowbatchesfinish
                + w.hashgrowbatchesreallocate
                + w.hashgrowbatchesrepartition
                + w.hashgrowbucketselect
                + w.hashgrowbucketsreallocate
                + w.hashgrowbucketsreinsert
                + w.logicalapplysenddata
                + w.logicalparallelapplystatechange
                + w.logicalsyncdata
                + w.logicalsyncstatechange
                + w.messagequeueinternal
                + w.messagequeueputmessage
                + w.messagequeuereceive
                + w.messagequeuesend
                + w.parallelbitmapscan
                + w.parallelcreateindexscan
                + w.parallelfinish
                + w.procarraygroupupdate
                + w.procsignalbarrier
                + w.promote
                + w.recoveryconflictsnapshot
                + w.recoveryconflicttablespace
                + w.recoveryendcommand
                + w.recoverypause
                + w.replicationorigindrop
                + w.replicationslotdrop
                + w.restorecommand
                + w.safesnapshot
                + w.syncrep
                + w.walreceiverexit
                + w.walreceiverwaitstart
                + w.xactgroupupdate
                + w.other
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type IPC",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.appendready as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(
        appendready,
        archivecleanupcommand,
        archivecommand,
        backendtermination,
        backupwaitwalarchive,
        bgworkershutdown,
        bgworkerstartup,
        btreepage,
        bufferio,
        checkpointdone,
        checkpointstart,
        executegather,
        hashbatchallocate,
        hashbatchelect,
        hashbatchload,
        hashbuildallocate,
        hashbuildelect,
        hashbuildhashinner,
        hashbuildhashouter,
        hashgrowbatchesdecide,
        hashgrowbatcheselect,
        hashgrowbatchesfinish,
        hashgrowbatchesreallocate,
        hashgrowbatchesrepartition,
        hashgrowbucketselect,
        hashgrowbucketsreallocate,
        hashgrowbucketsreinsert,
        logicalapplysenddata,
        logicalparallelapplystatechange,
        logicalsyncdata,
        logicalsyncstatechange,
        messagequeueinternal,
        messagequeueputmessage,
        messagequeuereceive,
        messagequeuesend,
        parallelbitmapscan,
        parallelcreateindexscan,
        parallelfinish,
        procarraygroupupdate,
        procsignalbarrier,
        promote,
        recoveryconflictsnapshot,
        recoveryconflicttablespace,
        recoveryendcommand,
        recoverypause,
        replicationorigindrop,
        replicationslotdrop,
        restorecommand,
        safesnapshot,
        syncrep,
        walreceiverexit,
        walreceiverwaitstart,
        xactgroupupdate,
        other,
    );

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_lock(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_lock.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| {
            w.advisory
                + w.applytransaction
                + w.extend
                + w.frozenid
                + w.object
                + w.page
                + w.relation
                + w.spectoken
                + w.transactionid
                + w.tuple
                + w.userlock
                + w.virtualxid
                + w.other
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type Lock",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.advisory as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(
        advisory,
        applytransaction,
        extend,
        frozenid,
        object,
        page,
        relation,
        spectoken,
        transactionid,
        tuple,
        userlock,
        virtualxid,
        other
    );

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_lwlock(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_lwlock.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| {
            w.addinsheminit
                + w.autofile
                + w.autovacuum
                + w.autovacuumschedule
                + w.backgroundworker
                + w.btreevacuum
                + w.buffercontent
                + w.buffermapping
                + w.checkpointercomm
                + w.committs
                + w.committsbuffer
                + w.committsslru
                + w.controlfile
                + w.dynamicsharedmemorycontrol
                + w.lockfastpath
                + w.lockmanager
                + w.logicalreplauncherdsa
                + w.logicalreplauncherhash
                + w.logicalrepworker
                + w.multixactgen
                + w.multixactmemberbuffer
                + w.multixactmemberslru
                + w.multixactoffsetbuffer
                + w.multixactoffsetslru
                + w.multixacttruncation
                + w.notifybuffer
                + w.notifyqueue
                + w.notifyqueuetail
                + w.notifyslru
                + w.oidgen
                + w.oldsnapshottimemap
                + w.parallelappend
                + w.parallelhashjoin
                + w.parallelquerydsa
                + w.persessiondsa
                + w.persessionrecordtype
                + w.persessionrecordtypmod
                + w.perxactpredicatelist
                + w.pgstatsdata
                + w.pgstatsdsa
                + w.pgstatshash
                + w.predicatelockmanager
                + w.procarray
                + w.relationmapping
                + w.relcacheinit
                + w.replicationorigin
                + w.replicationoriginstate
                + w.replicationslotallocation
                + w.replicationslotcontrol
                + w.replicationslotio
                + w.serialbuffer
                + w.serializablefinishedlist
                + w.serializablepredicatelist
                + w.serializablexacthash
                + w.serialslru
                + w.sharedtidbitmap
                + w.sharedtuplestore
                + w.shmemindex
                + w.sinvalread
                + w.sinvalwrite
                + w.subtransbuffer
                + w.subtransslru
                + w.syncrep
                + w.syncscan
                + w.tablespacecreate
                + w.twophasestate
                + w.walbufmapping
                + w.walinsert
                + w.walwrite
                + w.wraplimitsvacuum
                + w.xactbuffer
                + w.xactslru
                + w.xacttruncation
                + w.xidgen
                + w.other
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type LWLock",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.addinsheminit as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(
        addinsheminit,
        autofile,
        autovacuum,
        autovacuumschedule,
        backgroundworker,
        btreevacuum,
        buffercontent,
        buffermapping,
        checkpointercomm,
        committs,
        committsbuffer,
        committsslru,
        controlfile,
        dynamicsharedmemorycontrol,
        lockfastpath,
        lockmanager,
        logicalreplauncherdsa,
        logicalreplauncherhash,
        logicalrepworker,
        multixactgen,
        multixactmemberbuffer,
        multixactmemberslru,
        multixactoffsetbuffer,
        multixactoffsetslru,
        multixacttruncation,
        notifybuffer,
        notifyqueue,
        notifyqueuetail,
        notifyslru,
        oidgen,
        oldsnapshottimemap,
        parallelappend,
        parallelhashjoin,
        parallelquerydsa,
        persessiondsa,
        persessionrecordtype,
        persessionrecordtypmod,
        perxactpredicatelist,
        pgstatsdata,
        pgstatsdsa,
        pgstatshash,
        predicatelockmanager,
        procarray,
        relationmapping,
        relcacheinit,
        replicationorigin,
        replicationoriginstate,
        replicationslotallocation,
        replicationslotcontrol,
        replicationslotio,
        serialbuffer,
        serializablefinishedlist,
        serializablepredicatelist,
        serializablexacthash,
        serialslru,
        sharedtidbitmap,
        sharedtuplestore,
        shmemindex,
        sinvalread,
        sinvalwrite,
        subtransbuffer,
        subtransslru,
        syncrep,
        syncscan,
        tablespacecreate,
        twophasestate,
        walbufmapping,
        walinsert,
        walwrite,
        wraplimitsvacuum,
        xactbuffer,
        xactslru,
        xacttruncation,
        xidgen,
        other,
    );

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wait_type_timeout(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.wait_event_timeout.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value = events
        .iter()
        .map(|(_, w)| {
            w.basebackupthrottle
                + w.checkpointerwritedelay
                + w.pgsleep
                + w.recoveryapplydelay
                + w.recoveryretrieveretryinterval
                + w.registersyncrequest
                + w.spindelay
                + w.vacuumdelay
                + w.vacuumtruncate
                + w.other
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64 * 1.1;
    let sum_all_activity: usize = executor::block_on(DATA.wait_event_types.read())
        .iter()
        .map(|(_, r)| {
            r.on_cpu
                + r.activity
                + r.buffer_pin
                + r.client
                + r.extension
                + r.io
                + r.ipc
                + r.lock
                + r.lwlock
                + r.timeout
        })
        .sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wait event type Timeout",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.basebackupthrottle as f64)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10} {:>10}",
            "", "min", "max", "last", "%"
        ));
    //
    let mut color_number = 0;

    macro_rules! draw_series_if_active {
        ($first:ident $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                w.$first as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
            }
        };
        ($first:ident, $($other:tt),* $(,)?) => {
            let max = events
                .iter()
                .map(|(_,w)| w.$first)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            if max > 0 {
                let min = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap();
                let sum: usize = events
                    .iter()
                    .map(|(_,w)| w.$first)
                    .sum();
                contextarea
                    .draw_series(AreaSeries::new(
                        events.iter().map(|(timestamp, w)| {
                            (
                                *timestamp,
                                (w.$first $(+ w.$other)*) as f64,
                            )
                        }),
                        0.0,
                        Palette99::pick(color_number),
                    ))
                    .unwrap()
                    .label(format!(
                        "{:25} {:10} {:10} {:10} {:10.2}",
                        stringify!($first),
                        min,
                        max,
                        events.back().map_or(0, |(_, r)| r.$first),
                        if sum_all_activity == 0 {
                            0_f64
                        } else {
                            (sum as f64 / sum_all_activity as f64) * 100_f64
                        },
                    ))
                    .legend(move |(x, y)| {
                        Rectangle::new(
                            [(x - 3, y - 3), (x + 3, y + 3)],
                            Palette99::pick(color_number).filled(),
                        )
                    });
                color_number += 1;
            }
            draw_series_if_active!($($other,)*);
        }
    }
    draw_series_if_active!(
        basebackupthrottle,
        checkpointerwritedelay,
        pgsleep,
        recoveryapplydelay,
        recoveryretrieveretryinterval,
        registersyncrequest,
        spindelay,
        vacuumdelay,
        vacuumtruncate,
        other
    );

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wal_io_times(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.pg_stat_wal_sum.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value_write = events
        .iter()
        .map(|(_, w)| {
            if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                0_f64
            } else {
                w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let high_value_sync = events
        .iter()
        .map(|(_, w)| {
            if w.wal_sync_ps == 0_f64 {
                0_f64
            } else {
                w.wal_sync_time_ps / w.wal_sync_ps
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let high_value = high_value_write.max(high_value_sync) * 1.1_f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Wal IO latency",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Milliseconds")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.wal_records_ps)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "", "min", "max", "last"
        ));
    //
    //let mut color_number = 0;

    //    .map(|(_, w)| w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps))
    let min_write = events
        .iter()
        .map(|(_, w)| {
            if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                0_f64
            } else {
                w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
            }
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_write = events
        .iter()
        .map(|(_, w)| {
            if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                0_f64
            } else {
                w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            events.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                        0_f64
                    } else {
                        w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
                    },
                )
            }),
            GREEN,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10.3} {:10.3} {:10.3} ms",
            "Wal write",
            min_write,
            max_write,
            events.back().map_or(0_f64, |(_, r)| {
                if r.wal_buffers_full_ps + r.wal_write_ps == 0_f64 {
                    0_f64
                } else {
                    r.wal_write_time_ps / (r.wal_buffers_full_ps + r.wal_write_ps)
                }
            },)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));

    let min_sync = events
        .iter()
        .map(|(_, w)| {
            if w.wal_sync_ps == 0_f64 {
                0_f64
            } else {
                w.wal_sync_time_ps / w.wal_sync_ps
            }
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_sync = events
        .iter()
        .map(|(_, w)| {
            if w.wal_sync_ps == 0_f64 {
                0_f64
            } else {
                w.wal_sync_time_ps / w.wal_sync_ps
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            events.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    if w.wal_sync_ps == 0_f64 {
                        0_f64
                    } else {
                        w.wal_sync_time_ps / w.wal_sync_ps
                    },
                )
            }),
            BLUE,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10.3} {:10.3} {:10.3} ms",
            "Wal sync",
            min_sync,
            max_sync,
            events.back().map_or(0_f64, |(_, r)| {
                if r.wal_sync_ps == 0_f64 {
                    0_f64
                } else {
                    r.wal_sync_time_ps / r.wal_sync_ps
                }
            },)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE.filled()));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn wal_size(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let events = executor::block_on(DATA.pg_stat_wal_sum.read());
    let start_time = events.iter().map(|(timestamp, _)| timestamp).min().unwrap();
    let end_time = events.iter().map(|(timestamp, _)| timestamp).max().unwrap();
    let low_value_f64 = 0_f64;
    let high_value_bytes = events
        .iter()
        .map(|(_, w)| w.wal_bytes_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default()
        * 1.1_f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption("Wal IO size", (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE))
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_bytes)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Wal size")
        .y_label_formatter(&|size| human_bytes(*size))
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.wal_records_ps)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "", "min", "max", "last"
        ));
    //
    let min_write = events
        .iter()
        .map(|(_, w)| w.wal_bytes_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_write = events
        .iter()
        .map(|(_, w)| w.wal_bytes_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            events
                .iter()
                .map(|(timestamp, w)| (*timestamp, w.wal_bytes_ps)),
            GREEN,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "Wal size",
            human_bytes(min_write),
            human_bytes(max_write),
            human_bytes(events.back().map_or(0_f64, |(_, r)| r.wal_bytes_ps))
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn io_times(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let wal_events = executor::block_on(DATA.pg_stat_wal_sum.read());
    let database_events = executor::block_on(DATA.pg_stat_database_sum.read());
    let bgwriter_events = executor::block_on(DATA.pg_stat_bgwriter_sum.read());
    let wal_start_time = wal_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .min()
        .unwrap();
    let wal_end_time = wal_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .max()
        .unwrap();
    let database_start_time = database_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .min()
        .unwrap();
    let database_end_time = database_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .max()
        .unwrap();
    let bgwriter_start_time = bgwriter_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .min()
        .unwrap();
    let bgwriter_end_time = bgwriter_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .max()
        .unwrap();
    let start_time = wal_start_time
        .min(database_start_time)
        .min(bgwriter_start_time);
    let end_time = wal_end_time.max(database_end_time).max(bgwriter_end_time);
    let low_value_f64 = 0_f64;
    let wal_high_value_write = wal_events
        .iter()
        .map(|(_, w)| {
            if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                0_f64
            } else {
                w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let wal_high_value_sync = wal_events
        .iter()
        .map(|(_, w)| {
            if w.wal_sync_ps == 0_f64 {
                0_f64
            } else {
                w.wal_sync_time_ps / w.wal_sync_ps
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let database_high_value_read = database_events
        .iter()
        .map(|(_, d)| {
            if d.blks_read_ps == 0_f64 {
                0_f64
            } else {
                d.blk_read_time_ps / d.blks_read_ps
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let database_high_value_write = database_events
        .iter()
        .zip(bgwriter_events.iter())
        .map(|((_, d), (_, b))| {
            if b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps == 0_f64 {
                0_f64
            } else {
                d.blk_write_time_ps
                    / (b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps)
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let high_value = wal_high_value_write
        .max(wal_high_value_sync)
        .max(database_high_value_read)
        .max(database_high_value_write)
        * 1.1_f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption("IO latency", (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE))
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Milliseconds")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            wal_events
                .iter()
                .take(1)
                .map(|(timestamp, w)| (*timestamp, w.wal_records_ps)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "", "min", "max", "last"
        ));
    // wal write
    let min_write = wal_events
        .iter()
        .map(|(_, w)| {
            if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                0_f64
            } else {
                w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
            }
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_write = wal_events
        .iter()
        .map(|(_, w)| {
            if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                0_f64
            } else {
                w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            wal_events.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                        0_f64
                    } else {
                        w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
                    },
                )
            }),
            GREEN,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10.3} {:10.3} {:10.3} ms",
            "Wal write",
            min_write,
            max_write,
            wal_events.back().map_or(0_f64, |(_, r)| {
                if r.wal_buffers_full_ps + r.wal_write_ps == 0_f64 {
                    0_f64
                } else {
                    r.wal_write_time_ps / (r.wal_buffers_full_ps + r.wal_write_ps)
                }
            },)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));
    // wal sync
    let min_sync = wal_events
        .iter()
        .map(|(_, w)| {
            if w.wal_sync_ps == 0_f64 {
                0_f64
            } else {
                w.wal_sync_time_ps / w.wal_sync_ps
            }
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_sync = wal_events
        .iter()
        .map(|(_, w)| {
            if w.wal_sync_ps == 0_f64 {
                0_f64
            } else {
                w.wal_sync_time_ps / w.wal_sync_ps
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            wal_events.iter().map(|(timestamp, w)| {
                (
                    *timestamp,
                    if w.wal_sync_ps == 0_f64 {
                        0_f64
                    } else {
                        w.wal_sync_time_ps / w.wal_sync_ps
                    },
                )
            }),
            BLUE,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10.3} {:10.3} {:10.3} ms",
            "Wal sync",
            min_sync,
            max_sync,
            wal_events.back().map_or(0_f64, |(_, r)| {
                if r.wal_sync_ps == 0_f64 {
                    0_f64
                } else {
                    r.wal_sync_time_ps / r.wal_sync_ps
                }
            },)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE.filled()));
    // blocks read
    let min_database_read = database_events
        .iter()
        .map(|(_, d)| {
            if d.blks_read_ps == 0_f64 {
                0_f64
            } else {
                d.blk_read_time_ps / d.blks_read_ps
            }
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_database_read = database_events
        .iter()
        .map(|(_, d)| {
            if d.blks_read_ps == 0_f64 {
                0_f64
            } else {
                d.blk_read_time_ps / d.blks_read_ps
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            database_events.iter().map(|(timestamp, d)| {
                (
                    *timestamp,
                    if d.blks_read_ps == 0_f64 {
                        0_f64
                    } else {
                        d.blk_read_time_ps / d.blks_read_ps
                    },
                )
            }),
            BLACK,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10.3} {:10.3} {:10.3} ms",
            "Block read",
            min_database_read,
            max_database_read,
            database_events.back().map_or(0_f64, |(_, d)| {
                if d.blks_read_ps == 0_f64 {
                    0_f64
                } else {
                    d.blk_read_time_ps / d.blks_read_ps
                }
            },)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLACK.filled()));
    // blocks write
    let min_database_write = database_events
        .iter()
        .zip(bgwriter_events.iter())
        .map(|((_, d), (_, b))| {
            if b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps == 0_f64 {
                0_f64
            } else {
                d.blk_write_time_ps
                    / (b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps)
            }
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_database_write = database_events
        .iter()
        .zip(bgwriter_events.iter())
        .map(|((_, d), (_, b))| {
            if b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps == 0_f64 {
                0_f64
            } else {
                d.blk_write_time_ps
                    / (b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps)
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            database_events
                .iter()
                .zip(bgwriter_events.iter())
                .map(|((timestamp, d), (_, b))| {
                    (
                        *timestamp,
                        if b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps
                            == 0_f64
                        {
                            0_f64
                        } else {
                            d.blk_write_time_ps
                                / (b.buffers_checkpoint_ps
                                    + b.buffers_clean_ps
                                    + b.buffers_backend_ps)
                        },
                    )
                }),
            RED,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10.3} {:10.3} {:10.3} ms",
            "Block write",
            min_database_write,
            max_database_write,
            database_events
                .iter()
                .zip(bgwriter_events.iter())
                .last()
                .map_or(0_f64, |((_, d), (_, b))| {
                    if b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps == 0_f64
                    {
                        0_f64
                    } else {
                        d.blk_write_time_ps
                            / (b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps)
                    }
                },)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED.filled()));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
pub fn io_bandwidth(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let database_events = executor::block_on(DATA.pg_stat_database_sum.read());
    let bgwriter_events = executor::block_on(DATA.pg_stat_bgwriter_sum.read());
    let database_start_time = database_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .min()
        .unwrap();
    let database_end_time = database_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .max()
        .unwrap();
    let bgwriter_start_time = bgwriter_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .min()
        .unwrap();
    let bgwriter_end_time = bgwriter_events
        .iter()
        .map(|(timestamp, _)| timestamp)
        .max()
        .unwrap();
    let start_time = database_start_time.min(bgwriter_start_time);
    let end_time = database_end_time.max(bgwriter_end_time);
    let low_value_f64 = 0_f64;
    let high_value = database_events
        .iter()
        .zip(bgwriter_events.iter())
        .map(|((_, d), (_, b))| {
            (d.blks_read_ps + b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps)
                * 8192_f64
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default()
        * 1.1_f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "IO bandwidth (excluding WAL)",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Bandwidth")
        .y_label_formatter(&|size| human_bytes(*size))
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            database_events
                .iter()
                .take(1)
                .map(|(timestamp, d)| (*timestamp, d.blks_read_ps)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "", "min", "max", "last"
        ));
    // blocks read
    let min_read = database_events
        .iter()
        .map(|(_, d)| d.blks_read_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_read = database_events
        .iter()
        .map(|(_, d)| d.blks_read_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(AreaSeries::new(
            database_events
                .iter()
                .zip(bgwriter_events.iter())
                .map(|((timestamp, d), (_, b))| {
                    (
                        *timestamp,
                        (d.blks_read_ps
                            + b.buffers_checkpoint_ps
                            + b.buffers_clean_ps
                            + b.buffers_backend_ps)
                            * 8192_f64,
                    )
                }),
            0.0,
            GREEN,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "Blocks read",
            human_bytes(min_read * 8192_f64),
            human_bytes(max_read * 8192_f64),
            database_events
                .iter()
                .last()
                .map_or("".to_string(), |(_, b)| human_bytes(
                    b.blks_read_ps * 8192_f64
                ))
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));
    // blocks written checkpointer
    let min_read = bgwriter_events
        .iter()
        .map(|(_, d)| d.buffers_checkpoint_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_read = bgwriter_events
        .iter()
        .map(|(_, d)| d.buffers_checkpoint_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(AreaSeries::new(
            bgwriter_events.iter().map(|(timestamp, b)| {
                (
                    *timestamp,
                    (b.buffers_checkpoint_ps + b.buffers_clean_ps + b.buffers_backend_ps)
                        * 8192_f64,
                )
            }),
            0.0,
            BLUE,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "Checkpointer write",
            human_bytes(min_read * 8192_f64),
            human_bytes(max_read * 8192_f64),
            bgwriter_events
                .iter()
                .last()
                .map_or("".to_string(), |(_, b)| human_bytes(
                    b.buffers_checkpoint_ps * 8192_f64
                ))
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE.filled()));
    // blocks written bgwriter
    let min_read = bgwriter_events
        .iter()
        .map(|(_, d)| d.buffers_clean_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_read = bgwriter_events
        .iter()
        .map(|(_, d)| d.buffers_clean_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(AreaSeries::new(
            bgwriter_events.iter().map(|(timestamp, b)| {
                (
                    *timestamp,
                    (b.buffers_clean_ps + b.buffers_backend_ps) * 8192_f64,
                )
            }),
            0.0,
            PURPLE,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "Bgwriter write",
            human_bytes(min_read * 8192_f64),
            human_bytes(max_read * 8192_f64),
            bgwriter_events
                .iter()
                .last()
                .map_or("".to_string(), |(_, b)| human_bytes(
                    b.buffers_clean_ps * 8192_f64
                ))
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE.filled()));
    // blocks written backend
    let min_read = bgwriter_events
        .iter()
        .map(|(_, d)| d.buffers_backend_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_read = bgwriter_events
        .iter()
        .map(|(_, d)| d.buffers_backend_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(AreaSeries::new(
            bgwriter_events
                .iter()
                .map(|(timestamp, b)| (*timestamp, b.buffers_backend_ps * 8192_f64)),
            0.0,
            RED,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "Backend write",
            human_bytes(min_read * 8192_f64),
            human_bytes(max_read * 8192_f64),
            bgwriter_events
                .iter()
                .last()
                .map_or("".to_string(), |(_, b)| human_bytes(
                    b.buffers_backend_ps * 8192_f64
                ))
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED.filled()));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}

#[derive(Debug, Default)]
pub struct QueryIdAndWaitTypes {
    pub query: String,
    pub total: usize,
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

#[derive(Debug)]
struct QueryCollection {
    query_id: i64,
    query: String,
    total: usize,
    on_cpu: usize,
    activity: usize,
    buffer_pin: usize,
    client: usize,
    extension: usize,
    io: usize,
    ipc: usize,
    lock: usize,
    lwlock: usize,
    timeout: usize,
}

pub fn ash_by_query_id(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let mut samples_per_queryid: HashMap<i64, QueryIdAndWaitTypes> = HashMap::new();
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    for per_sample_vector in pg_stat_activity.iter().map(|(_, v)| v) {
        for r in per_sample_vector.iter() {
            if r.state.as_ref().unwrap_or(&"".to_string()) == "active" {
                samples_per_queryid
                    .entry(r.query_id.unwrap_or_default())
                    .or_insert(QueryIdAndWaitTypes {
                        query: r.query.as_ref().unwrap_or(&"".to_string()).clone(),
                        ..Default::default()
                    });
                match r.wait_event_type.as_deref().unwrap_or_default() {
                    "activity" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.activity += 1),
                    "bufferpin" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.buffer_pin += 1),
                    "client" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.client += 1),
                    "extension" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.extension += 1),
                    "io" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.io += 1),
                    "ipc" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.ipc += 1),
                    "lock" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.lock += 1),
                    "lwlock" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.lwlock += 1),
                    "timeout" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.timeout += 1),
                    &_ => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.on_cpu += 1),
                };
                samples_per_queryid
                    .entry(r.query_id.unwrap_or_default())
                    .and_modify(|r| r.total += 1);
            }
        }
    }

    let mut qc: Vec<QueryCollection> = Vec::new();
    for (q, d) in samples_per_queryid {
        qc.push(QueryCollection {
            query_id: q,
            query: d.query,
            total: d.total,
            on_cpu: d.on_cpu,
            activity: d.activity,
            buffer_pin: d.buffer_pin,
            client: d.client,
            extension: d.extension,
            io: d.io,
            ipc: d.ipc,
            lock: d.lock,
            lwlock: d.lwlock,
            timeout: d.timeout,
        });
    }
    qc.sort_by(|b, a| b.total.cmp(&a.total));

    let total_number_queryids = if qc.len() > 1 { qc.len() - 1 } else { qc.len() };
    let max_total_queryids = (qc.iter().map(|d| d.total).max().unwrap_or_default() * 110) / 100;
    let tot_total: usize = qc.iter().map(|d| d.total).sum();
    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, 200)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .caption(
            "Query id by number of samples",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(
            0..max_total_queryids,
            (0..total_number_queryids).into_segmented(),
        )
        .unwrap();
    contextarea
        .configure_mesh()
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .y_labels(qc.len())
        .y_label_formatter(&|v| {
            format!(
                "{}",
                qc.iter()
                    .map(|r| r.query_id)
                    .nth({
                        if let SegmentValue::CenterOf(val) = v {
                            *val
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0)
            )
        })
        .x_desc("Samples")
        .x_label_formatter(&|n| n.to_string())
        .draw()
        .unwrap();

    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (0, SegmentValue::Exact(y)),
                    (x.on_cpu, SegmentValue::Exact(y + 1)),
                ],
                GREEN.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "On CPU",
            qc.iter().map(|d| d.on_cpu).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.on_cpu).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (x.on_cpu, SegmentValue::Exact(y)),
                    (x.on_cpu + x.io, SegmentValue::Exact(y + 1)),
                ],
                BLUE_600.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE_600.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "IO",
            qc.iter().map(|d| d.io).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.io).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (x.on_cpu + x.io, SegmentValue::Exact(y)),
                    (x.on_cpu + x.io + x.lock, SegmentValue::Exact(y + 1)),
                ],
                RED.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "Lock",
            qc.iter().map(|d| d.lock).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.lock).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (x.on_cpu + x.io + x.lock, SegmentValue::Exact(y)),
                    (
                        x.on_cpu + x.io + x.lock + x.lwlock,
                        SegmentValue::Exact(y + 1),
                    ),
                ],
                RED_900.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED_900.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "LWLock",
            qc.iter().map(|d| d.lwlock).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.lwlock).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (x.on_cpu + x.io + x.lock + x.lwlock, SegmentValue::Exact(y)),
                    (
                        x.on_cpu + x.io + x.lock + x.lwlock + x.ipc,
                        SegmentValue::Exact(y + 1),
                    ),
                ],
                PINK_A100.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PINK_A100.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "IPC",
            qc.iter().map(|d| d.ipc).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.ipc).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (
                        x.on_cpu + x.io + x.lock + x.lwlock + x.ipc,
                        SegmentValue::Exact(y),
                    ),
                    (
                        x.on_cpu + x.io + x.lock + x.lwlock + x.ipc + x.timeout,
                        SegmentValue::Exact(y + 1),
                    ),
                ],
                BROWN.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BROWN.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "Timeout",
            qc.iter().map(|d| d.timeout).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.timeout).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (
                        x.on_cpu + x.io + x.lock + x.lwlock + x.ipc + x.timeout,
                        SegmentValue::Exact(y),
                    ),
                    (
                        x.on_cpu + x.io + x.lock + x.lwlock + x.ipc + x.timeout + x.extension,
                        SegmentValue::Exact(y + 1),
                    ),
                ],
                GREEN_800.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN_800.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "Extension",
            qc.iter().map(|d| d.extension).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.extension).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (
                        x.on_cpu + x.io + x.lock + x.lwlock + x.ipc + x.timeout + x.extension,
                        SegmentValue::Exact(y),
                    ),
                    (
                        x.on_cpu
                            + x.io
                            + x.lock
                            + x.lwlock
                            + x.ipc
                            + x.timeout
                            + x.extension
                            + x.client,
                        SegmentValue::Exact(y + 1),
                    ),
                ],
                GREY.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREY.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "Client",
            qc.iter().map(|d| d.client).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.client).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (
                        x.on_cpu
                            + x.io
                            + x.lock
                            + x.lwlock
                            + x.ipc
                            + x.timeout
                            + x.extension
                            + x.client,
                        SegmentValue::Exact(y),
                    ),
                    (
                        x.on_cpu
                            + x.io
                            + x.lock
                            + x.lwlock
                            + x.ipc
                            + x.timeout
                            + x.extension
                            + x.client
                            + x.buffer_pin,
                        SegmentValue::Exact(y + 1),
                    ),
                ],
                LIGHTBLUE_300.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], LIGHTBLUE_300.filled())
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "Buffer Pin",
            qc.iter().map(|d| d.buffer_pin).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.buffer_pin).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = Rectangle::new(
                [
                    (
                        x.on_cpu
                            + x.io
                            + x.lock
                            + x.lwlock
                            + x.ipc
                            + x.timeout
                            + x.extension
                            + x.client
                            + x.buffer_pin,
                        SegmentValue::Exact(y),
                    ),
                    (
                        x.on_cpu
                            + x.io
                            + x.lock
                            + x.lwlock
                            + x.ipc
                            + x.timeout
                            + x.extension
                            + x.client
                            + x.buffer_pin
                            + x.activity,
                        SegmentValue::Exact(y + 1),
                    ),
                ],
                PURPLE.filled(),
            );
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE.filled()))
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "Activity",
            qc.iter().map(|d| d.activity).sum::<usize>(),
            if tot_total == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.activity).sum::<usize>() as f64 / tot_total as f64 * 100_f64
            },
        ));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(LowerRight)
        .draw()
        .unwrap();
}
pub fn show_queries(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let mut samples_per_queryid: HashMap<i64, QueryIdAndWaitTypes> = HashMap::new();
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    for per_sample_vector in pg_stat_activity.iter().map(|(_, v)| v) {
        for r in per_sample_vector.iter() {
            if r.state.as_ref().unwrap_or(&"".to_string()) == "active" {
                samples_per_queryid
                    .entry(r.query_id.unwrap_or_default())
                    .or_insert(QueryIdAndWaitTypes {
                        query: r.query.as_ref().unwrap_or(&"".to_string()).clone(),
                        ..Default::default()
                    });
                match r.wait_event_type.as_deref().unwrap_or_default() {
                    "activity" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.activity += 1),
                    "bufferpin" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.buffer_pin += 1),
                    "client" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.client += 1),
                    "extension" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.extension += 1),
                    "io" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.io += 1),
                    "ipc" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.ipc += 1),
                    "lock" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.lock += 1),
                    "lwlock" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.lwlock += 1),
                    "timeout" => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.timeout += 1),
                    &_ => samples_per_queryid
                        .entry(r.query_id.unwrap_or_default())
                        .and_modify(|r| r.on_cpu += 1),
                };
                samples_per_queryid
                    .entry(r.query_id.unwrap_or_default())
                    .and_modify(|r| r.total += 1);
            }
        }
    }

    let mut qc: Vec<QueryCollection> = Vec::new();
    for (q, d) in samples_per_queryid {
        qc.push(QueryCollection {
            query_id: q,
            query: d.query,
            total: d.total,
            on_cpu: d.on_cpu,
            activity: d.activity,
            buffer_pin: d.buffer_pin,
            client: d.client,
            extension: d.extension,
            io: d.io,
            ipc: d.ipc,
            lock: d.lock,
            lwlock: d.lwlock,
            timeout: d.timeout,
        });
    }
    qc.sort_by(|a, b| b.total.cmp(&a.total));
    let grand_total_samples: f64 = qc.iter().map(|r| r.total as f64).sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();

    let mut y_counter = 0;
    for query in qc.iter() {
        multi_backend[backend_number]
            .draw(&Text::new(
                format!(
                    "{:>20}  {:6.2}% {:8} {}",
                    query.query_id,
                    query.total as f64 / grand_total_samples * 100_f64,
                    query.total,
                    if query.query_id == 0 {
                        "*".to_string()
                    } else {
                        query.query.to_string()
                    }
                ),
                (10, y_counter),
                (MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE).into_font(),
            ))
            .unwrap();
        y_counter += 20;
    }
    multi_backend[backend_number]
        .draw(&Text::new(
            format!(
                "{:>20}  {:6.2}% {:8} {}",
                "total", 100_f64, grand_total_samples, ""
            ),
            (10, y_counter),
            (MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE).into_font(),
        ))
        .unwrap();
}

pub fn xid_age(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let xid_age = executor::block_on(DATA.pg_database_xid_limits.read());
    let start_time = xid_age
        .iter()
        .map(|(timestamp, _)| timestamp)
        .min()
        .unwrap();
    let end_time = xid_age
        .iter()
        .map(|(timestamp, _)| timestamp)
        .max()
        .unwrap();
    let low_value_f64 = 0_f64;
    let high_value = 2_i64.pow(31) as f64 * 1.1_f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Transaction ID age",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("XID age")
        .y_label_formatter(&|age| format!("{}", *age))
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .take(1)
                .map(|(timestamp, d)| (*timestamp, d.age_datminmxid)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "", "min", "max", "last"
        ));

    let min_vmfma = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_multixact_freeze_min_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vmfma = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_multixact_freeze_min_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.vacuum_multixact_freeze_min_age)),
            ORANGE_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_multixact_freeze_min_age",
            min_vmfma,
            max_vmfma,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_multixact_freeze_min_age)
        ))
        .legend(move |(x, y)| {
            Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], ORANGE_200.filled())
        });
    let min_vmfta = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_multixact_freeze_table_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vmfta = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_multixact_freeze_table_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.vacuum_multixact_freeze_table_age)),
            GREEN_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_multixact_freeze_table_age",
            min_vmfta,
            max_vmfta,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_multixact_freeze_table_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN_200.filled()));
    let min_amfma = xid_age
        .iter()
        .map(|(_, d)| d.autovacuum_multixact_freeze_max_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_amfma = xid_age
        .iter()
        .map(|(_, d)| d.autovacuum_multixact_freeze_max_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.autovacuum_multixact_freeze_max_age)),
            BLUE_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "autovacuum_multixact_freeze_max_age",
            min_amfma,
            max_amfma,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.autovacuum_multixact_freeze_max_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE_200.filled()));
    let min_vmfa = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_multixact_failsafe_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vmfa = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_multixact_failsafe_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.vacuum_multixact_failsafe_age)),
            PURPLE_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_multixact_failsafe_age",
            min_vmfa,
            max_vmfa,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_multixact_failsafe_age)
        ))
        .legend(move |(x, y)| {
            Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE_200.filled())
        });
    let min_mxid_xid = xid_age
        .iter()
        .map(|(_, d)| d.age_datminmxid)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_mxid_xid = xid_age
        .iter()
        .map(|(_, d)| d.age_datminmxid)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.age_datminmxid)),
            ShapeStyle {
                color: GREY.into(),
                filled: true,
                stroke_width: 3,
            },
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "multixact XID",
            min_mxid_xid,
            max_mxid_xid,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.age_datminmxid)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREY.filled()));
    let min_vfma = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_freeze_min_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vfma = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_freeze_min_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.vacuum_freeze_min_age)),
            ORANGE,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_freeze_min_age",
            min_vfma,
            max_vfma,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_freeze_min_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], ORANGE.filled()));
    let min_vfta = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_freeze_table_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vfta = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_freeze_table_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.vacuum_freeze_table_age)),
            GREEN,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_freeze_table_age",
            min_vfta,
            max_vfta,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_freeze_table_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));
    let min_amfa = xid_age
        .iter()
        .map(|(_, d)| d.autovacuum_freeze_max_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_amfa = xid_age
        .iter()
        .map(|(_, d)| d.autovacuum_freeze_max_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.autovacuum_freeze_max_age)),
            BLUE,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "autovacuum_freeze_max_age",
            min_amfa,
            max_amfa,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.autovacuum_freeze_max_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE.filled()));
    let min_vfa = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_failsafe_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vfa = xid_age
        .iter()
        .map(|(_, d)| d.vacuum_failsafe_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.vacuum_failsafe_age)),
            PURPLE,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_failsafe_age",
            min_vfa,
            max_vfa,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_failsafe_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE.filled()));
    let min_frozen_xid = xid_age
        .iter()
        .map(|(_, d)| d.age_datfrozenxid)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_frozen_xid = xid_age
        .iter()
        .map(|(_, d)| d.age_datfrozenxid)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, d)| (*timestamp, d.age_datfrozenxid)),
            ShapeStyle {
                color: BLACK.into(),
                filled: true,
                stroke_width: 3,
            },
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "Frozen XID",
            min_frozen_xid,
            max_frozen_xid,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.age_datfrozenxid)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLACK.filled()));

    // readonly point
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, _)| (*timestamp, (2_i64.pow(31) - 3_000_000_i64) as f64)),
            RED_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "readonly point",
            "",
            "",
            2_i64.pow(31) - 3_000_000_i64
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED_200.filled()));
    // the absolute limit
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .map(|(timestamp, _)| (*timestamp, 2_i64.pow(31) as f64)),
            RED,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "absolute limit",
            "",
            "",
            2_i64.pow(31)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED.filled()));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
