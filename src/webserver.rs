use std::time::Duration;
use axum::{response::IntoResponse, response::Html, extract::Path, Router, routing::get};
use plotters::style::full_palette::{BLUE_600, BROWN, GREEN_800, GREY, LIGHTBLUE_300, PINK_A100, PURPLE, RED_900};
use tokio::time::sleep;
use std::io::Cursor;
use anyhow::Result;
use image::{DynamicImage, ImageFormat};
use futures::executor;

use plotters::coord::Shift;
use plotters::backend::RGBPixel;
use plotters::chart::SeriesLabelPosition::UpperLeft;
use plotters::prelude::*; 

use crate::{CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, MESH_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE, LABEL_AREA_SIZE_LEFT, LABEL_AREA_SIZE_BOTTOM, LABEL_AREA_SIZE_RIGHT, MESH_STYLE_FONT};


use crate::{ARGS, DATA};

pub async fn webserver() -> Result<()> {
    let app = Router::new()
        .route("/handler/:plot_1", get(handler_html))
        .route("/plotter/:plot_1", get(handler_plotter))
        .route("/", get(root_handler));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", ARGS.webserver_port)).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();

    Ok(())
}

pub async fn root_handler() -> Html<String> {
    loop {
        if DATA.wait_event_types.read().await.iter().count() > 0 {
            break
        } else {
            let _ = sleep(Duration::from_secs(1));
        }
    }
    format!(r##"<!doctype html>
 <html>
   <head>
   <style>
    .container {{ }}
    .column_left {{ width: 10%; float:left; }}
    .column_right {{ width: 90%; height: 3000px; float:right; }}
   </style>
  </head>
  <body>
  <div class = "container">
   <div class = "column_left">
    <nav>
     <li><a href="/" target="right">Home</a></li>
     <li><a href="/handler/session_history" target="right">session history</a></li>
    </nav>
   </div>
   <div class = "column_right">
    <iframe name="right" id="right" width="100%" height="100%">
   </div>
  </div>
  </body>
 </html>
 "##).into()
}

pub async fn handler_html(Path(plot_1): Path<String>) -> Html<String> {
    format!(r#"<img src="/plotter/{}">"#, plot_1).into()
}

pub async fn handler_plotter(Path(plot_1): Path<String>) -> impl IntoResponse {
    let mut buffer = vec![0; (ARGS.graph_width * ARGS.graph_height * 3).try_into().unwrap()];
    match plot_1.as_str() {
        "session_history" => create_wait_event_type_plot(&mut buffer),
        &_ => todo!(),
    }
    let rgb_image = DynamicImage::ImageRgb8(image::RgbImage::from_raw(ARGS.graph_width, ARGS.graph_height, buffer).unwrap());
    let mut cursor = Cursor::new(Vec::new());
    rgb_image.write_to(&mut cursor, ImageFormat::Png).unwrap();
    cursor.into_inner()
}

pub fn create_wait_event_type_plot(buffer: &mut [u8]) {
    let backend = BitMapBackend::with_buffer(buffer, (ARGS.graph_width, ARGS.graph_height)).into_drawing_area();
    let mut multi_backend = backend.split_evenly((1, 1));
    wait_event_type_plot(&mut multi_backend, 0);
}

pub fn wait_event_type_plot(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,   
) {
    let wait_event_type = executor::block_on(DATA.wait_event_types.read());
    let start_time = wait_event_type
        .iter()
        .map(|(timestamp,_)| timestamp)
        .min()
        .unwrap();
    let end_time = wait_event_type
        .iter()
        .map(|(timestamp,_)| timestamp)
        .max()
        .unwrap();
    let low_value_f64 = 0_f64;
    let high_value = wait_event_type
        .iter()
        .map(|(_,w)| w.on_cpu+w.activity+w.buffer_pin+w.client+w.extension+w.io+w.ipc+w.lock+w.lwlock+w.timeout)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let high_value_f64 = high_value as f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption("Active sessions", (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE))
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value_f64)
        .unwrap();
    contextarea.configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea.draw_series(LineSeries::new(wait_event_type
            .iter()
            .take(1)
            .map(|(timestamp,w)| (*timestamp, w.on_cpu as f64)), ShapeStyle { color: TRANSPARENT, filled: false, stroke_width: 1} ))
        .unwrap()
        .label(format!("{:25} {:>10} {:>10} {:>10}", "", "min", "max", "last"));
    // 
    let min_activity = wait_event_type.iter().map(|(_,w)| w.activity).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_activity = wait_event_type.iter().map(|(_,w)| w.activity).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc + w.timeout + w.extension + w.client + w.buffer_pin + w.activity) as f64)), 0.0, PURPLE))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "Activity", min_activity, max_activity, wait_event_type.back().map_or(0, |(_,r)| r.activity )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE.filled()));
    // 
    let min_buffer_pin = wait_event_type.iter().map(|(_,w)| w.buffer_pin).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_buffer_pin = wait_event_type.iter().map(|(_,w)| w.buffer_pin).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc + w.timeout + w.extension + w.client + w.buffer_pin) as f64)), 0.0, LIGHTBLUE_300))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "Buffer Pin", min_buffer_pin, max_buffer_pin, wait_event_type.back().map_or(0, |(_,r)| r.buffer_pin )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], LIGHTBLUE_300.filled()));
    // 
    let min_client = wait_event_type.iter().map(|(_,w)| w.client).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_client = wait_event_type.iter().map(|(_,w)| w.client).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc + w.timeout + w.extension + w.client) as f64)), 0.0, GREY))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "Client", min_client, max_client, wait_event_type.back().map_or(0, |(_,r)| r.client )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREY.filled()));
    // 
    let min_extension = wait_event_type.iter().map(|(_,w)| w.extension).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_extension = wait_event_type.iter().map(|(_,w)| w.extension).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc + w.timeout + w.extension) as f64)), 0.0, GREEN_800))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "Extension", min_extension, max_extension, wait_event_type.back().map_or(0, |(_,r)| r.extension )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN_800.filled()));
    // 
    let min_timeout = wait_event_type.iter().map(|(_,w)| w.timeout).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_timeout = wait_event_type.iter().map(|(_,w)| w.timeout).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc + w.timeout) as f64)), 0.0, BROWN))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "Timeout", min_timeout, max_timeout, wait_event_type.back().map_or(0, |(_,r)| r.timeout )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BROWN.filled()));
    // 
    let min_ipc = wait_event_type.iter().map(|(_,w)| w.ipc).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_ipc = wait_event_type.iter().map(|(_,w)| w.ipc).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock + w.lwlock + w.ipc) as f64)), 0.0, PINK_A100))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "IPC", min_ipc, max_ipc, wait_event_type.back().map_or(0, |(_,r)| r.ipc )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PINK_A100.filled()));
    // 
    let min_lwlock = wait_event_type.iter().map(|(_,w)| w.lwlock).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_lwlock = wait_event_type.iter().map(|(_,w)| w.lwlock).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock + w.lwlock) as f64)), 0.0, RED_900))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "LWLock", min_lwlock, max_lwlock, wait_event_type.back().map_or(0, |(_,r)| r.lwlock )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED_900.filled()));
    // 
    let min_lock = wait_event_type.iter().map(|(_,w)| w.lock).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_lock = wait_event_type.iter().map(|(_,w)| w.lock).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io + w.lock) as f64)), 0.0, RED))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "Lock", min_lock, max_lock, wait_event_type.back().map_or(0, |(_,r)| r.lock )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED.filled()));
    // 
    let min_io = wait_event_type.iter().map(|(_,w)| w.io).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_io = wait_event_type.iter().map(|(_,w)| w.io).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, (w.on_cpu + w.io) as f64)), 0.0, BLUE_600))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "IO", min_io, max_io, wait_event_type.back().map_or(0, |(_,r)| r.io )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE_600.filled()));
    // 
    let min_on_cpu = wait_event_type.iter().map(|(_,w)| w.on_cpu).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_on_cpu = wait_event_type.iter().map(|(_,w)| w.on_cpu).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
            .map(|(timestamp, w)| (*timestamp, w.on_cpu as f64)), 0.0, GREEN))
        .unwrap()
        .label(format!("{:25} {:10} {:10} {:10}", "On CPU", min_on_cpu, max_on_cpu, wait_event_type.back().map_or(0, |(_,r)| r.on_cpu )))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));

    contextarea.configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();


}

