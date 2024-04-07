use std::time::Duration;
use axum::{response::IntoResponse, response::Html, extract::Path, Router, routing::get};
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

    let min_on_cpu = wait_event_type.iter().map(|(_,w)| w.on_cpu).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_on_cpu = wait_event_type.iter().map(|(_,w)| w.on_cpu).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

    contextarea.draw_series(AreaSeries::new(
        wait_event_type.iter()
                .map(|(timestamp, w)| (*timestamp, w.on_cpu as f64)), 0.0, GREEN))
            .unwrap()
        //.label(format!("{:25} {:10.2} {:10.2}, {:10.2}", "on cpu", min_on_cpu, max_on_cpu, wait_event_type.back().unwrap().on_cpu))
        .label(format!("{:25} {:10.2} {:10.2}, {:10.2}", "on cpu", min_on_cpu, max_on_cpu, 0))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));


}

