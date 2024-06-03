use chrono::{DateTime, Local};
use std::collections::BTreeMap;
use std::ops::Bound::Included;

use crate::webserver::wait_type_color;
use crate::DATA;
use crate::{
    CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE,
    LABEL_AREA_SIZE_BOTTOM, LABEL_AREA_SIZE_LEFT, LABEL_AREA_SIZE_RIGHT, MESH_STYLE_FONT,
    MESH_STYLE_FONT_SIZE,
};
use futures::executor;
use plotters::backend::RGBPixel;
use plotters::chart::SeriesLabelPosition::UpperLeft;
use plotters::coord::Shift;
use plotters::prelude::*;

pub fn wait_event_type_plot(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    #[derive(Debug, Default)]
    struct DynamicDateAndWaits {
        timestamp: DateTime<Local>,
        waits: BTreeMap<String, usize>,
    }
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    let mut wait_event_counter: BTreeMap<String, usize> = BTreeMap::new();
    let mut timestamp_and_waits: Vec<DynamicDateAndWaits> = Vec::new();
    let mut max_active = 0;
    for (timestamp, per_sample_vector) in pg_stat_activity.iter() {
        let mut current_timestamp_data = DynamicDateAndWaits {
            timestamp: *timestamp,
            ..Default::default()
        };
        let mut current_waits_data: BTreeMap<String, usize> = BTreeMap::new();
        let mut current_max_active = 0;
        for row in per_sample_vector.iter() {
            if row.state.as_deref().unwrap_or_default() == "active" {
                current_max_active += 1;
                let wait_event = if row.wait_event_type.as_deref().unwrap_or_default() == "" {
                    "~on_cpu".to_string()
                } else {
                    row.wait_event_type
                        .as_deref()
                        .unwrap_or_default()
                        .to_string()
                };

                wait_event_counter
                    .entry(wait_event.clone())
                    .and_modify(|r| *r += 1_usize)
                    .or_insert(1_usize);

                current_waits_data
                    .entry(wait_event)
                    .and_modify(|r| *r += 1_usize)
                    .or_insert(1_usize);
            }
            max_active = max_active.max(current_max_active);
        }
        current_timestamp_data.waits = current_waits_data;
        timestamp_and_waits.push(current_timestamp_data);
    }
    // add in the missing waits that are zero
    for vector in timestamp_and_waits.iter_mut() {
        for (wait, _) in wait_event_counter.clone() {
            vector.waits.entry(wait).or_insert(0_usize);
        }
    }

    let start_time = timestamp_and_waits
        .iter()
        .map(|v| v.timestamp)
        .min()
        .unwrap();
    let end_time = timestamp_and_waits
        .iter()
        .map(|v| v.timestamp)
        .max()
        .unwrap();
    let low_value = 0_usize;
    let high_value = max_active;

    multi_backend[backend_number].fill(&WHITE).unwrap();

    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Active sessions",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(start_time..end_time, low_value..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions by wait event type")
        .y_label_formatter(&|sessions| format!("{:4.0}", sessions))
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    contextarea
        .draw_series(LineSeries::new(
            timestamp_and_waits
                .iter()
                .take(1)
                .map(|v| (v.timestamp, 0_usize)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!("{:25} {:>10}  {:>5}", "", "tot", "%"));

    let last_key = wait_event_counter
        .keys()
        .max()
        .unwrap_or(&"".to_string())
        .clone();
    let total_samples = wait_event_counter.values().sum::<usize>();
    for wait_event in wait_event_counter.keys() {
        //println!("last key: {}, current wait: {}", last_key, wait_event,);
        contextarea
            .draw_series(AreaSeries::new(
                timestamp_and_waits.iter().map(|v| {
                    (
                        v.timestamp,
                        v.waits
                            .range::<str, _>((
                                Included(wait_event.as_str()),
                                Included(last_key.as_str()),
                            ))
                            .map(|(_, v)| *v as isize)
                            .sum::<isize>() as usize,
                    )
                }),
                0,
                //Palette99::pick(color_number),
                wait_type_color(wait_event),
            ))
            .unwrap()
            .label(format!(
                "{:25} {:>10}  {:>5.2}",
                wait_event,
                wait_event_counter.get(wait_event).unwrap(),
                *wait_event_counter.get(wait_event).unwrap() as f64 / total_samples as f64
                    * 100_f64
            ))
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    //Palette99::pick(color_number).filled(),
                    wait_type_color(wait_event).filled(),
                )
            });
    }

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}

pub fn wait_event_plot(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    #[derive(Debug, Default)]
    struct DynamicDateAndWaits {
        timestamp: DateTime<Local>,
        waits: BTreeMap<String, usize>,
    }
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    let mut wait_event_counter: BTreeMap<String, usize> = BTreeMap::new();
    let mut timestamp_and_waits: Vec<DynamicDateAndWaits> = Vec::new();
    let mut max_active = 0;
    for (timestamp, per_sample_vector) in pg_stat_activity.iter() {
        let mut current_timestamp_data = DynamicDateAndWaits {
            timestamp: *timestamp,
            ..Default::default()
        };
        let mut current_waits_data: BTreeMap<String, usize> = BTreeMap::new();
        let mut current_max_active = 0;
        for row in per_sample_vector.iter() {
            if row.state.as_deref().unwrap_or_default() == "active" {
                current_max_active += 1;
                let wait_event = if format!(
                    "{}:{}",
                    row.wait_event_type.as_deref().unwrap_or_default(),
                    row.wait_event.as_deref().unwrap_or_default()
                ) == ":"
                {
                    "on_cpu".to_string()
                } else {
                    format!(
                        "{}:{}",
                        row.wait_event_type.as_deref().unwrap_or_default(),
                        row.wait_event.as_deref().unwrap_or_default()
                    )
                };

                wait_event_counter
                    .entry(wait_event.clone())
                    .and_modify(|r| *r += 1_usize)
                    .or_insert(1_usize);

                current_waits_data
                    .entry(wait_event)
                    .and_modify(|r| *r += 1_usize)
                    .or_insert(1_usize);
            }
            max_active = max_active.max(current_max_active);
        }
        current_timestamp_data.waits = current_waits_data;
        timestamp_and_waits.push(current_timestamp_data);
    }
    // add in the missing waits that are zero
    for vector in timestamp_and_waits.iter_mut() {
        for (wait, _) in wait_event_counter.clone() {
            vector.waits.entry(wait).or_insert(0_usize);
        }
    }

    let start_time = timestamp_and_waits
        .iter()
        .map(|v| v.timestamp)
        .min()
        .unwrap();
    let end_time = timestamp_and_waits
        .iter()
        .map(|v| v.timestamp)
        .max()
        .unwrap();
    let low_value = 0_usize;
    let high_value = max_active;

    multi_backend[backend_number].fill(&WHITE).unwrap();

    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Active sessions",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(start_time..end_time, low_value..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Active sessions")
        .y_label_formatter(&|sessions| format!("{:4.0}", sessions))
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    contextarea
        .draw_series(LineSeries::new(
            timestamp_and_waits
                .iter()
                .take(1)
                .map(|v| (v.timestamp, 0_usize)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!("{:25} {:>10}  {:>5}", "", "tot", "%"));

    let last_key = wait_event_counter
        .keys()
        .max()
        .unwrap_or(&"".to_string())
        .clone();
    let total_samples = wait_event_counter.values().sum::<usize>();
    for (color_number, wait_event) in wait_event_counter.keys().enumerate() {
        //println!("last key: {}, current wait: {}", last_key, wait_event,);
        contextarea
            .draw_series(AreaSeries::new(
                timestamp_and_waits.iter().map(|v| {
                    (
                        v.timestamp,
                        v.waits
                            .range::<str, _>((
                                Included(wait_event.as_str()),
                                Included(last_key.as_str()),
                            ))
                            .map(|(_, v)| *v as isize)
                            .sum::<isize>() as usize,
                    )
                }),
                0,
                Palette99::pick(color_number),
            ))
            .unwrap()
            .label(format!(
                "{:25} {:>10}  {:>5.2}",
                wait_event,
                wait_event_counter.get(wait_event).unwrap(),
                *wait_event_counter.get(wait_event).unwrap() as f64 / total_samples as f64
                    * 100_f64
            ))
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    Palette99::pick(color_number).filled(),
                )
            });
    }

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
