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
use plotters::style::full_palette::{
    BLUE_600, BROWN, GREEN_800, GREY, LIGHTBLUE_300, PINK_A100, PURPLE, RED_900,
};
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
