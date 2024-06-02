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
use std::collections::{BTreeMap, HashMap};
use std::ops::Bound::Included;

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
    // this will sort the query collection (qc) by total.
    // this makes the qc vector be ordered from low to high.
    qc.sort_by(|a, b| b.total.cmp(&a.total));

    let mut qc_count = if qc.len() > 1 { qc.len() - 1 } else { qc.len() };
    let mut qc_total_max = (qc.iter().map(|d| d.total).max().unwrap_or_default() * 110) / 100;
    let qc_total_sum: usize = qc.iter().map(|d| d.total).sum();
    let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
    let qc_max_size = y_size as usize / 20;
    let mut others_counter = 0;

    if qc.len() > qc_max_size {
        let mut others = QueryCollection {
            ..Default::default()
        };
        for q in &qc[qc_max_size..] {
            others_counter += 1;
            others.total += q.total;
            others.on_cpu += q.on_cpu;
            others.activity += q.activity;
            others.buffer_pin += q.buffer_pin;
            others.client += q.client;
            others.extension += q.extension;
            others.io += q.io;
            others.ipc += q.ipc;
            others.lock += q.lock;
            others.lwlock += q.lwlock;
            others.timeout += q.timeout;
        }
        others.query = format!("..others ({})", others_counter);
        others.query_id = -1;
        qc.truncate(qc_max_size);
        qc.push(others);
        qc_count = qc.len() - 1;
        qc_total_max = (qc.iter().map(|d| d.total).max().unwrap_or_default() * 110) / 100;
    }

    // this will show the query with the highest total amount on top
    qc.reverse();

    multi_backend[backend_number].fill(&WHITE).unwrap();
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
            let query_id = qc
                .iter()
                .map(|r| r.query_id)
                .nth({
                    if let SegmentValue::CenterOf(val) = v {
                        *val
                    } else {
                        0
                    }
                })
                .unwrap_or(0);
            match query_id {
                -1 => qc
                    .iter()
                    .find(|r| r.query_id == query_id)
                    .map(|r| r.query.clone())
                    .unwrap(),
                _ => query_id.to_string(),
            }
        })
        .x_desc("Samples")
        .x_label_formatter(&|n| n.to_string())
        .draw()
        .unwrap();

    /*
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
    */
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                    wait_type_color("on_cpu").filled(),
                )
            } else {
                Rectangle::new(
                    [
                        (0, SegmentValue::Exact(y)),
                        (x.on_cpu, SegmentValue::Exact(y + 1)),
                    ],
                    wait_type_color("on_cpu").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("on_cpu").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "on_cpu",
            qc.iter().map(|d| d.on_cpu).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.on_cpu).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                    wait_type_color("io").filled(),
                )
            } else {
                Rectangle::new(
                    [
                        (x.on_cpu, SegmentValue::Exact(y)),
                        (x.on_cpu + x.io, SegmentValue::Exact(y + 1)),
                    ],
                    wait_type_color("io").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("io").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "io",
            qc.iter().map(|d| d.io).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.io).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                    wait_type_color("lock").filled(),
                )
            } else {
                Rectangle::new(
                    [
                        (x.on_cpu + x.io, SegmentValue::Exact(y)),
                        (x.on_cpu + x.io + x.lock, SegmentValue::Exact(y + 1)),
                    ],
                    wait_type_color("lock").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("lock").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "lock",
            qc.iter().map(|d| d.lock).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.lock).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                    wait_type_color("lwlock").filled(),
                )
            } else {
                Rectangle::new(
                    [
                        (x.on_cpu + x.io + x.lock, SegmentValue::Exact(y)),
                        (
                            x.on_cpu + x.io + x.lock + x.lwlock,
                            SegmentValue::Exact(y + 1),
                        ),
                    ],
                    wait_type_color("lwlock").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("lwlock").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "lwlock",
            qc.iter().map(|d| d.lwlock).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.lwlock).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                    wait_type_color("ipc").filled(),
                )
            } else {
                Rectangle::new(
                    [
                        (x.on_cpu + x.io + x.lock + x.lwlock, SegmentValue::Exact(y)),
                        (
                            x.on_cpu + x.io + x.lock + x.lwlock + x.ipc,
                            SegmentValue::Exact(y + 1),
                        ),
                    ],
                    wait_type_color("ipc").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("ipc").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "ipc",
            qc.iter().map(|d| d.ipc).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.ipc).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                    wait_type_color("timeout").filled(),
                )
            } else {
                Rectangle::new(
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
                    wait_type_color("timeout").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("timeout").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "timeout",
            qc.iter().map(|d| d.timeout).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.timeout).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                    wait_type_color("extension").filled(),
                )
            } else {
                Rectangle::new(
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
                    wait_type_color("extension").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("extension").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "extension",
            qc.iter().map(|d| d.extension).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.extension).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                    wait_type_color("client").filled(),
                )
            } else {
                Rectangle::new(
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
                    wait_type_color("client").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("client").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "client",
            qc.iter().map(|d| d.client).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.client).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                    wait_type_color("buffer_pin").filled(),
                )
            } else {
                Rectangle::new(
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
                    wait_type_color("buffer_pin").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("buffer_pin").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "buffer_pin",
            qc.iter().map(|d| d.buffer_pin).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.buffer_pin).sum::<usize>() as f64 / qc_total_sum as f64
                    * 100_f64
            },
        ));
    contextarea
        .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
            let mut bar = if x.query_id == -1 {
                Rectangle::new(
                    [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                    wait_type_color("activity").filled(),
                )
            } else {
                Rectangle::new(
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
                    wait_type_color("activity").filled(),
                )
            };
            bar.set_margin(2, 2, 0, 0);
            bar
        }))
        .unwrap()
        .legend(move |(x, y)| {
            Rectangle::new(
                [(x - 3, y - 3), (x + 3, y + 3)],
                wait_type_color("activity").filled(),
            )
        })
        .label(format!(
            "{:10} {:>8} {:>6.2}%",
            "activity",
            qc.iter().map(|d| d.activity).sum::<usize>(),
            if qc_total_sum == 0 {
                0_f64
            } else {
                qc.iter().map(|d| d.activity).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
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
                    format!("..others ({})", others_counter),
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

pub fn show_queries_html(//multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    //backend_number: usize,
) -> String {
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

    //multi_backend[backend_number].fill(&WHITE).unwrap();

    //let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
    //let max_number_queries = (y_size / 21) - 1;
    //let mut others = QueryCollection {
    //    query: "others".to_string(),
    //    ..Default::default()
    //};
    //let mut others_counter = 0;
    let mut html_output = String::from(
        r#"<table border=1>
            <colgroup>
                <col style="width:160px;">
                <col style="width:80;">
                <col style="width:100px;">
                <col style="width:1000px;">
            </colgroup>
            <tr>
                <th align=right>Query ID</th>
                <th align=right>Percent</th>
                <th align=right>Total</th>
                <th>Query</th>
            </tr>"#,
    );

    //let mut y_counter = 0;

    for query in qc.iter() {
        html_output += format!(
            "<tr>
                <td align=right>{:>20}</td>
                <td align=right>{:6.2}%</td>
                <td align=right>{:8}</td>
                <td>{}</td>
            </tr>",
            query.query_id,
            query.total as f64 / grand_total_samples * 100_f64,
            query.total,
            if query.query_id == 0 {
                "*".to_string()
            } else {
                query.query.to_string()
            }
        )
        .as_str();
    }
    html_output += format!(
        "<tr>
                <td align=right>{:>20}</td>
                <td align=right>{:6.2}%</td>
                <td align=right>{:8}</td>
                <td>{}</td>
            </tr>",
        "total", 100_f64, grand_total_samples, ""
    )
    .as_str();

    html_output += "</table>";
    /*
        if others_counter > 0 {
            multi_backend[backend_number]
                .draw(&Text::new(
                    format!(
                        "{:>20}  {:6.2}% {:8} {}",
                        format!("..others ({})", others_counter),
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
    */
    //html_output.into()
    html_output
}
pub fn waits_by_query_id(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    let mut queryid_waits: HashMap<i64, BTreeMap<String, usize>> = HashMap::new();
    let mut wait_event_counter: BTreeMap<String, usize> = BTreeMap::new();
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    for (_, per_sample_vector) in pg_stat_activity.iter() {
        for row in per_sample_vector.iter() {
            if row.state.as_deref().unwrap_or_default() == "active" {
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
                queryid_waits
                    .entry(row.query_id.unwrap_or_default())
                    .and_modify(|r| {
                        r.entry(wait_event.clone())
                            .and_modify(|w| *w += 1)
                            .or_insert(1_usize);
                    })
                    .or_insert(BTreeMap::from([(wait_event.clone(), 1_usize)]));
            }
        }
    }
    // insert all wait events to the btreemap of all queryid's
    for (_, map) in queryid_waits.iter_mut() {
        for (wait, _) in wait_event_counter.clone() {
            map.entry(wait).or_insert(0_usize);
        }
    }
    // get the "last" key from the wait_event_counter btreemap
    let last_key = wait_event_counter
        .keys()
        .max()
        .unwrap_or(&"".to_string())
        .clone();
    let queryid_count = queryid_waits.len();

    // samples_max is the highest number of samples that is found for all of the queryid's.
    // this is needed to define the length of the graph for the horizontal stacked bars.
    let mut samples_max = 0;
    for (_, waits) in queryid_waits.iter() {
        samples_max =
            samples_max.max(waits.iter().map(|(_, nr)| *nr as isize).sum::<isize>() as usize);
    }

    // build the graph
    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, 200)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .caption(
            "Query id by number of samples",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(0..samples_max, (0..queryid_count).into_segmented())
        .unwrap();
    contextarea
        .configure_mesh()
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        //.y_labels(qc.len())
        /*
                .y_label_formatter(&|v| {
                    let query_id = qc
                        .iter()
                        .map(|r| r.query_id)
                        .nth({
                            if let SegmentValue::CenterOf(val) = v {
                                *val
                            } else {
                                0
                            }
                        })
                        .unwrap_or(0);
                    match query_id {
                        -1 => qc
                            .iter()
                            .find(|r| r.query_id == query_id)
                            .map(|r| r.query.clone())
                            .unwrap(),
                        _ => query_id.to_string(),
                    }
                })
        */
        .x_desc("Samples")
        .x_label_formatter(&|n| n.to_string())
        .draw()
        .unwrap();
    //
    //
    // build a vector of struct: query_id and waits: btreemap<string, usize>
    // then peak at wait_event_plot
    //
    //
    //
    //
    //
    //let mut counter = 0;
    //for (query_id, waits) in queryid_waits {
    for (color_number, wait_event) in wait_event_counter.keys().enumerate() {
        contextarea
            .draw_series((0..).zip(queryid_waits.clone()).map(|(y, x)| {
                let mut bar = Rectangle::new(
                    [
                        (0, SegmentValue::Exact(y)),
                        (
                            x.1.range::<str, _>((
                                Included(wait_event.as_str()),
                                Included(last_key.as_str()),
                            ))
                            .map(|(_, v)| *v as isize)
                            .sum::<isize>() as usize,
                            SegmentValue::Exact(y + 1),
                        ),
                    ],
                    Palette99::pick(color_number).filled(),
                );
                /*
                                    let mut bar = if x.query_id == -1 {
                                        Rectangle::new(
                                            [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                                            wait_type_color("on_cpu").filled(),
                                        )
                                    } else {
                                        Rectangle::new(
                                            [
                                                (0, SegmentValue::Exact(y)),
                                                (x.on_cpu, SegmentValue::Exact(y + 1)),
                                            ],
                                            Palette99::pick(color_number).filled(),
                                        )
                                    };
                */
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    Palette99::pick(color_number).filled(),
                )
            })
            .label(format!(
                //"{:10} {:>8} {:>6.2}%",
                "{:25} {:>8} {:6.2}%",
                wait_event,
                wait_event_counter.get(wait_event).unwrap(),
                *wait_event_counter.get(wait_event).unwrap() as f64
                    / wait_event_counter.values().map(|v| *v as f64).sum::<f64>()
                    * 100_f64,
                /*
                                    qc.iter().map(|d| d.on_cpu).sum::<usize>(),
                                    if qc_total_sum == 0 {
                                        0_f64
                                    } else {
                                        qc.iter().map(|d| d.on_cpu).sum::<usize>() as f64 / qc_total_sum as f64
                                            * 100_f64
                                    },
                */
            ));
        //color_number += 1;
    }
    //}

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(LowerRight)
        .draw()
        .unwrap();

    /*
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
        // this will sort the query collection (qc) by total.
        // this makes the qc vector be ordered from low to high.
        qc.sort_by(|a, b| b.total.cmp(&a.total));

        let mut qc_count = if qc.len() > 1 { qc.len() - 1 } else { qc.len() };
        let mut qc_total_max = (qc.iter().map(|d| d.total).max().unwrap_or_default() * 110) / 100;
        let qc_total_sum: usize = qc.iter().map(|d| d.total).sum();
        let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
        let qc_max_size = y_size as usize / 20;
        let mut others_counter = 0;

        if qc.len() > qc_max_size {
            let mut others = QueryCollection {
                ..Default::default()
            };
            for q in &qc[qc_max_size..] {
                others_counter += 1;
                others.total += q.total;
                others.on_cpu += q.on_cpu;
                others.activity += q.activity;
                others.buffer_pin += q.buffer_pin;
                others.client += q.client;
                others.extension += q.extension;
                others.io += q.io;
                others.ipc += q.ipc;
                others.lock += q.lock;
                others.lwlock += q.lwlock;
                others.timeout += q.timeout;
            }
            others.query = format!("..others ({})", others_counter);
            others.query_id = -1;
            qc.truncate(qc_max_size);
            qc.push(others);
            qc_count = qc.len() - 1;
            qc_total_max = (qc.iter().map(|d| d.total).max().unwrap_or_default() * 110) / 100;
        }

        // this will show the query with the highest total amount on top
        qc.reverse();

        multi_backend[backend_number].fill(&WHITE).unwrap();
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
                let query_id = qc
                    .iter()
                    .map(|r| r.query_id)
                    .nth({
                        if let SegmentValue::CenterOf(val) = v {
                            *val
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                match query_id {
                    -1 => qc
                        .iter()
                        .find(|r| r.query_id == query_id)
                        .map(|r| r.query.clone())
                        .unwrap(),
                    _ => query_id.to_string(),
                }
            })
            .x_desc("Samples")
            .x_label_formatter(&|n| n.to_string())
            .draw()
            .unwrap();

        /*
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
        */
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                        wait_type_color("on_cpu").filled(),
                    )
                } else {
                    Rectangle::new(
                        [
                            (0, SegmentValue::Exact(y)),
                            (x.on_cpu, SegmentValue::Exact(y + 1)),
                        ],
                        wait_type_color("on_cpu").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("on_cpu").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "on_cpu",
                qc.iter().map(|d| d.on_cpu).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.on_cpu).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                        wait_type_color("io").filled(),
                    )
                } else {
                    Rectangle::new(
                        [
                            (x.on_cpu, SegmentValue::Exact(y)),
                            (x.on_cpu + x.io, SegmentValue::Exact(y + 1)),
                        ],
                        wait_type_color("io").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("io").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "io",
                qc.iter().map(|d| d.io).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.io).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                        wait_type_color("lock").filled(),
                    )
                } else {
                    Rectangle::new(
                        [
                            (x.on_cpu + x.io, SegmentValue::Exact(y)),
                            (x.on_cpu + x.io + x.lock, SegmentValue::Exact(y + 1)),
                        ],
                        wait_type_color("lock").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("lock").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "lock",
                qc.iter().map(|d| d.lock).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.lock).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                        wait_type_color("lwlock").filled(),
                    )
                } else {
                    Rectangle::new(
                        [
                            (x.on_cpu + x.io + x.lock, SegmentValue::Exact(y)),
                            (
                                x.on_cpu + x.io + x.lock + x.lwlock,
                                SegmentValue::Exact(y + 1),
                            ),
                        ],
                        wait_type_color("lwlock").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("lwlock").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "lwlock",
                qc.iter().map(|d| d.lwlock).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.lwlock).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                        wait_type_color("ipc").filled(),
                    )
                } else {
                    Rectangle::new(
                        [
                            (x.on_cpu + x.io + x.lock + x.lwlock, SegmentValue::Exact(y)),
                            (
                                x.on_cpu + x.io + x.lock + x.lwlock + x.ipc,
                                SegmentValue::Exact(y + 1),
                            ),
                        ],
                        wait_type_color("ipc").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("ipc").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "ipc",
                qc.iter().map(|d| d.ipc).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.ipc).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                        wait_type_color("timeout").filled(),
                    )
                } else {
                    Rectangle::new(
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
                        wait_type_color("timeout").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("timeout").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "timeout",
                qc.iter().map(|d| d.timeout).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.timeout).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                        wait_type_color("extension").filled(),
                    )
                } else {
                    Rectangle::new(
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
                        wait_type_color("extension").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("extension").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "extension",
                qc.iter().map(|d| d.extension).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.extension).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                        wait_type_color("client").filled(),
                    )
                } else {
                    Rectangle::new(
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
                        wait_type_color("client").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("client").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "client",
                qc.iter().map(|d| d.client).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.client).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                        wait_type_color("buffer_pin").filled(),
                    )
                } else {
                    Rectangle::new(
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
                        wait_type_color("buffer_pin").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("buffer_pin").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "buffer_pin",
                qc.iter().map(|d| d.buffer_pin).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.buffer_pin).sum::<usize>() as f64 / qc_total_sum as f64
                        * 100_f64
                },
            ));
        contextarea
            .draw_series((0..).zip(qc.iter()).map(|(y, x)| {
                let mut bar = if x.query_id == -1 {
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y))],
                        wait_type_color("activity").filled(),
                    )
                } else {
                    Rectangle::new(
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
                        wait_type_color("activity").filled(),
                    )
                };
                bar.set_margin(2, 2, 0, 0);
                bar
            }))
            .unwrap()
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 3, y - 3), (x + 3, y + 3)],
                    wait_type_color("activity").filled(),
                )
            })
            .label(format!(
                "{:10} {:>8} {:>6.2}%",
                "activity",
                qc.iter().map(|d| d.activity).sum::<usize>(),
                if qc_total_sum == 0 {
                    0_f64
                } else {
                    qc.iter().map(|d| d.activity).sum::<usize>() as f64 / qc_total_sum as f64 * 100_f64
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
    */
}
