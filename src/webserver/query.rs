use crate::{ARGS, DATA};
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

/*
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
*/
pub fn show_queries(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
) {
    #[derive(Debug)]
    struct QueryAndTotal {
        query: String,
        total: usize,
    }
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    let mut samples_per_queryid: HashMap<i64, QueryAndTotal> =
        HashMap::with_capacity(pg_stat_activity.len());
    for per_sample_vector in pg_stat_activity.iter().map(|(_, v)| v) {
        for r in per_sample_vector.iter() {
            if r.state.as_deref().unwrap_or_default() == "active" {
                samples_per_queryid
                    .entry(r.query_id.unwrap_or_default())
                    .and_modify(|r| r.total += 1)
                    .or_insert(QueryAndTotal {
                        query: r.query.as_deref().unwrap_or_default().to_string(),
                        total: 1,
                    });
            }
        }
    }

    #[derive(Debug, Default)]
    struct QueryIdQueryTotal {
        query_id: i64,
        query: String,
        total: usize,
    }
    let mut qc: Vec<QueryIdQueryTotal> = Vec::new();
    for (query_id, vector) in samples_per_queryid {
        qc.push(QueryIdQueryTotal {
            query_id,
            query: vector.query,
            total: vector.total,
        });
    }
    qc.sort_by(|a, b| b.total.cmp(&a.total));
    let grand_total_samples: f64 = qc.iter().map(|r| r.total as f64).sum();

    multi_backend[backend_number].fill(&WHITE).unwrap();

    let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
    let max_number_queries = (y_size / 21) - 1;
    let mut others = QueryIdQueryTotal {
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

pub fn show_queries_queryid_html(queryid: &i64) -> String {
    #[derive(Debug)]
    struct QueryidAndTotal {
        queryid: i64,
        total: usize,
    }
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    let mut samples_per_query: HashMap<String, QueryidAndTotal> =
        HashMap::with_capacity(pg_stat_activity.len());
    for per_sample_vector in pg_stat_activity.iter().map(|(_, v)| v) {
        for r in per_sample_vector
            .iter()
            .filter(|r| r.query_id.as_ref().unwrap_or(&0) == queryid)
        {
            if r.state.as_deref().unwrap_or_default() == "active" {
                samples_per_query
                    .entry(r.query.as_deref().unwrap_or_default().to_string())
                    .and_modify(|r| r.total += 1)
                    .or_insert(QueryidAndTotal {
                        queryid: *r.query_id.as_ref().unwrap_or(&0_i64),
                        total: 1,
                    });
            }
        }
    }

    #[derive(Debug)]
    struct QueryIdQueryTotal {
        queryid: i64,
        query: String,
        total: usize,
    }
    let mut qc: Vec<QueryIdQueryTotal> = Vec::new();
    for (query, vector) in samples_per_query {
        qc.push(QueryIdQueryTotal {
            query,
            queryid: vector.queryid,
            total: vector.total,
        });
    }
    qc.sort_by(|a, b| b.total.cmp(&a.total));
    let grand_total_samples: f64 = qc.iter().map(|r| r.total as f64).sum();

    let mut html_output = format!(
        r#"<table border=1>
            <colgroup>
                <col style="width:160px;">
                <col style="width:80;">
                <col style="width:100px;">
                <col style="width:{}px;">
            </colgroup>
            <tr>
                <th align=right>Query ID</th>
                <th align=right>Percent</th>
                <th align=right>Total</th>
                <th>Query</th>
            </tr>"#,
        ARGS.graph_width - (160_u32 + 80_u32 + 100_u32)
    );

    for query in qc.iter() {
        html_output += format!(
            r#"<tr>
                <td align=right>{}
                </td>
                <td align=right>{:6.2}%</td>
                <td align=right>{:8}</td>
                <td>{}</td>
            </tr>"#,
            query.queryid,
            query.total as f64 / grand_total_samples * 100_f64,
            query.total,
            query.query
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
    html_output
}
pub fn show_queries_html() -> String {
    #[derive(Debug)]
    struct QueryAndTotal {
        query: String,
        total: usize,
    }
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    let mut samples_per_queryid: HashMap<i64, QueryAndTotal> =
        HashMap::with_capacity(pg_stat_activity.len());
    for per_sample_vector in pg_stat_activity.iter().map(|(_, v)| v) {
        for r in per_sample_vector.iter() {
            if r.state.as_deref().unwrap_or_default() == "active" {
                samples_per_queryid
                    .entry(r.query_id.unwrap_or_default())
                    .and_modify(|r| r.total += 1)
                    .or_insert(QueryAndTotal {
                        query: r.query.as_deref().unwrap_or_default().to_string(),
                        total: 1,
                    });
            }
        }
    }

    #[derive(Debug)]
    struct QueryIdQueryTotal {
        query_id: i64,
        query: String,
        total: usize,
    }
    let mut qc: Vec<QueryIdQueryTotal> = Vec::new();
    for (query_id, vector) in samples_per_queryid {
        qc.push(QueryIdQueryTotal {
            query_id,
            query: vector.query,
            total: vector.total,
        });
    }
    qc.sort_by(|a, b| b.total.cmp(&a.total));
    let grand_total_samples: f64 = qc.iter().map(|r| r.total as f64).sum();

    let mut html_output = format!(
        r#"<table border=1>
            <colgroup>
                <col style="width:160px;">
                <col style="width:80;">
                <col style="width:100px;">
                <col style="width:{}px;">
            </colgroup>
            <tr>
                <th align=right>Query ID</th>
                <th align=right>Percent</th>
                <th align=right>Total</th>
                <th>Query</th>
            </tr>"#,
        ARGS.graph_width - (160_u32 + 80_u32 + 100_u32)
    );

    for query in qc.iter() {
        html_output += format!(
            r#"<tr>
                <td align=right>
                  <a href="/dual_handler/ash_wait_query_by_queryid/all_queries/{}">{:>20}</a>
                </td>
                <td align=right>{:6.2}%</td>
                <td align=right>{:8}</td>
                <td>{}</td>
            </tr>"#,
            query.query_id,
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
    html_output
}
pub fn waits_by_query_id(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
    queryid_filter: &bool,
    queryid: &i64,
) {
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    let mut wait_event_counter: BTreeMap<String, usize> = BTreeMap::new();
    let mut queryid_waits: HashMap<i64, BTreeMap<String, usize>> =
        HashMap::with_capacity(pg_stat_activity.len());
    for (_, per_sample_vector) in pg_stat_activity.iter() {
        for row in per_sample_vector
            .iter()
            .filter(|r| !*queryid_filter || r.query_id.as_ref().unwrap_or(&0) == queryid)
        {
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

    // samples_max is the highest number of samples that is found for all of the queryid's.
    // this is needed to define the length of the graph for the horizontal stacked bars.
    let mut samples_max = 0;
    for (_, waits) in queryid_waits.iter() {
        samples_max =
            samples_max.max(waits.iter().map(|(_, nr)| *nr as isize).sum::<isize>() as usize);
    }
    #[derive(Debug, Default)]
    struct DynamicQueryIdTotalWaits {
        queryid: i64,
        total: usize,
        others: bool,
        waits: BTreeMap<String, usize>,
    }

    let mut queryid_total_waits: Vec<DynamicQueryIdTotalWaits> =
        Vec::with_capacity(queryid_waits.len());

    for (queryid, waits) in queryid_waits.iter() {
        queryid_total_waits.push(DynamicQueryIdTotalWaits {
            queryid: *queryid,
            waits: waits.clone(),
            others: false,
            total: {
                waits
                    .iter()
                    .map(|(_, nr)| *nr as isize)
                    .sum::<isize>()
                    .try_into()
                    .unwrap()
            },
        })
    }
    //queryid_total_waits.sort_by_key(|k| k.total);
    queryid_total_waits.sort_by(|a, b| b.total.cmp(&a.total));

    let mut queryid_total_waits_count = if queryid_total_waits.len() > 1 {
        queryid_total_waits.len() - 1
    } else {
        queryid_total_waits.len()
    };

    let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
    let qtw_max = y_size as usize / 20;

    if queryid_total_waits.len() > qtw_max {
        let mut others = DynamicQueryIdTotalWaits {
            ..Default::default()
        };
        others.total = queryid_total_waits.len() - qtw_max;
        others.others = true;
        queryid_total_waits.truncate(qtw_max);
        queryid_total_waits.push(others);
        queryid_total_waits_count = queryid_total_waits.len() - 1;
    }

    queryid_total_waits.reverse();

    // build the graph
    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, 200)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .caption(
            "Query id by number of samples",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(
            0..samples_max,
            (0..queryid_total_waits_count).into_segmented(),
        )
        .unwrap();
    contextarea
        .configure_mesh()
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .y_labels(queryid_total_waits_count)
        .y_label_formatter(&|v| {
            if queryid_total_waits
                .iter()
                .map(|r| r.others)
                .nth({
                    if let SegmentValue::CenterOf(val) = v {
                        *val
                    } else {
                        0
                    }
                })
                .unwrap_or_default()
            {
                format!(
                    "..others: ({})",
                    &queryid_total_waits
                        .iter()
                        .map(|r| r.total)
                        .nth({
                            if let SegmentValue::CenterOf(val) = v {
                                *val
                            } else {
                                0
                            }
                        })
                        .unwrap_or_default()
                )
            } else {
                queryid_total_waits
                    .iter()
                    .map(|r| r.queryid)
                    .nth({
                        if let SegmentValue::CenterOf(val) = v {
                            *val
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0)
                    .to_string()
            }
        })
        .x_desc("Samples")
        .x_label_formatter(&|n| n.to_string())
        .draw()
        .unwrap();
    //
    for (color_number, wait_event) in wait_event_counter.keys().enumerate() {
        contextarea
            .draw_series((0..).zip(queryid_total_waits.iter()).map(|(y, x)| {
                let mut bar = if x.others {
                    // others has the tendency to gather a lot of events, potentially making it
                    // far bigger than a single queryid, even for the highest single querid.
                    // therefore it's shown with no bar graph.
                    Rectangle::new(
                        [(0, SegmentValue::Exact(y)), (0, SegmentValue::Exact(y + 1))],
                        Palette99::pick(color_number).filled(),
                    )
                } else {
                    Rectangle::new(
                        [
                            (0, SegmentValue::Exact(y)),
                            (
                                x.waits
                                    .range::<str, _>((
                                        Included(wait_event.as_str()),
                                        Included(last_key.as_str()),
                                    ))
                                    .map(|(_, v)| *v as isize)
                                    .sum::<isize>() as usize,
                                SegmentValue::Exact(y + 1),
                            ),
                        ],
                        Palette99::pick(color_number).filled(),
                    )
                };
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
                "{:25} {:>8} {:6.2}%",
                wait_event,
                wait_event_counter.get(wait_event).unwrap(),
                *wait_event_counter.get(wait_event).unwrap() as f64
                    / wait_event_counter.values().map(|v| *v as f64).sum::<f64>()
                    * 100_f64,
            ));
    }

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(LowerRight)
        .draw()
        .unwrap();
}
