use crate::webserver::wait_type_color;
use crate::DATA;
use crate::{
    CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE,
    LABEL_AREA_SIZE_BOTTOM, MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE,
};
use futures::executor;
use plotters::backend::RGBPixel;
use plotters::chart::SeriesLabelPosition::LowerRight;
use plotters::coord::Shift;
use plotters::prelude::*;
use std::collections::HashMap;

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

#[derive(Debug, Default)]
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

    let qc_count = if qc.len() > 1 { qc.len() - 1 } else { qc.len() };
    let qc_total_max = (qc.iter().map(|d| d.total).max().unwrap_or_default() * 110) / 100;
    let qc_total_sum: usize = qc.iter().map(|d| d.total).sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();

    let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
    if y_size < 317 {
        multi_backend[backend_number]
            .draw(&Text::new(
                "The set heigth is too small to display this graph (query id by number of samples)"
                    .to_string(),
                (10, 10),
                (MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE).into_font(),
            ))
            .unwrap();
        return;
    }

    //println!("count:{}, y:{}", qc_count, y_size);

    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, 200)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .caption(
            "Query id by number of samples",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(0..qc_total_max, (0..qc_count).into_segmented())
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

    macro_rules! draw_bars_with_wait_types {
        ($wait_type:ident $(,)?) => {

            contextarea
                .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                    let mut bar = Rectangle::new(
                        [
                            (0, SegmentValue::Exact(y)),
                            (x.$wait_type, SegmentValue::Exact(y + 1)),
                        ],
                        wait_type_color(stringify!($wait_type)).filled(),
                    );
                    bar.set_margin(2, 2, 0, 0);
                    bar
                }))
                .unwrap()
                .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], wait_type_color(stringify!($wait_type)).filled()))
                .label(format!(
                    "{:10} {:>8} {:>6.2}%",
                    stringify!($wait_type),
                    qc.iter().map(|d| d.$wait_type).sum::<usize>(),
                    if qc_total_sum == 0 {
                        0_f64
                    } else {
                        qc.iter().map(|d| d.$wait_type).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                    },
                ));

            };
        ($wait_type:ident, $($other_types:tt),* $(,)?) => {

            contextarea
                .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                    let mut bar = Rectangle::new(
                        [
                            (0, SegmentValue::Exact(y)),
                            (x.$wait_type, SegmentValue::Exact(y + 1)),
                        ],
                        wait_type_color(stringify!($wait_type)).filled(),
                    );
                    bar.set_margin(2, 2, 0, 0);
                    bar
                }))
                .unwrap()
                .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], wait_type_color(stringify!($wait_type)).filled()))
                .label(format!(
                    "{:10} {:>8} {:>6.2}%",
                    stringify!($wait_type),
                    qc.iter().map(|d| d.$wait_type).sum::<usize>(),
                    if qc_total_sum == 0 {
                        0_f64
                    } else {
                        qc.iter().map(|d| d.$wait_type).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                    },
                ));

            draw_bars_with_wait_types!($($other_types,)*);
        }
    }
    draw_bars_with_wait_types!(
        on_cpu, io, lock, lwlock, ipc, timeout, extension, client, buffer_pin, activity
    );

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

    let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
    let max_number_queries = (y_size / 21) - 1;
    let mut others = QueryCollection {
        query: "others".to_string(),
        ..Default::default()
    };
    let mut others_counter = 0;

    let mut y_counter = 0;
    for query in qc.iter() {
        if qc.len() as u32 <= max_number_queries
            || (qc.len() as u32 > max_number_queries && (y_counter / 20) < max_number_queries)
        {
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
                    (10, y_counter as i32),
                    (MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE).into_font(),
                ))
                .unwrap();
            y_counter += 20;
        } else {
            others_counter += 1;
            others.on_cpu += query.on_cpu;
            others.io += query.io;
            others.ipc += query.ipc;
            others.buffer_pin += query.buffer_pin;
            others.extension += query.extension;
            others.activity += query.activity;
            others.timeout += query.timeout;
            others.lwlock += query.lwlock;
            others.client += query.client;
            others.lock += query.lock;
            others.total += query.total;
        }
    }
    if others_counter > 0 {
        multi_backend[backend_number]
            .draw(&Text::new(
                format!(
                    "{:>20}  {:6.2}% {:8} {}",
                    "..others",
                    others.total as f64 / grand_total_samples * 100_f64,
                    others.total,
                    "*"
                ),
                (10, y_counter as i32),
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
            (10, y_counter as i32),
            (MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE).into_font(),
        ))
        .unwrap();
}
