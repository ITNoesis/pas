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

    let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
    if y_size < 317 {
        multi_backend[backend_number]
            .draw(&Text::new(
                "The set heigth is too small to display this graph (wait event types)".to_string(),
                (10, 10),
                (MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE).into_font(),
            ))
            .unwrap();
        return;
    }

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
        .y_label_formatter(&|sessions| format!("{:4.0}", sessions))
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

    macro_rules! draw_wait_types {
        ($wait_type:ident $(,)?) => {
            let min = wait_event_type
                .iter()
                .map(|(_, w)| w.$wait_type)
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let max = wait_event_type
                .iter()
                .map(|(_, w)| w.$wait_type)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let sum: usize = wait_event_type.iter().map(|(_, w)| w.$wait_type).sum();
            contextarea
                .draw_series(AreaSeries::new(
                    wait_event_type
                        .iter()
                        .map(|(timestamp, w)| (*timestamp, w.$wait_type as f64)),
                    0.0,
                    wait_type_color(stringify!($wait_type)),
                ))
                .unwrap()
                .label(format!(
                    "{:25} {:10} {:10} {:10} {:10.2}",
                    stringify!($wait_type),
                    min,
                    max,
                    wait_event_type.back().map_or(0, |(_, r)| r.$wait_type),
                    if sum_all_activity == 0 {
                        0_f64
                    } else {
                        (sum as f64 / sum_all_activity as f64) * 100_f64
                    },
                ))
                .legend(move |(x, y)| {
                    Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], wait_type_color(stringify!($wait_type)).filled())
                });
        };
        ($wait_type:ident, $($other_types:tt),* $(,)?) => {
            let min = wait_event_type
                .iter()
                .map(|(_, w)| w.$wait_type)
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let max = wait_event_type
                .iter()
                .map(|(_, w)| w.$wait_type)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let sum: usize = wait_event_type.iter().map(|(_, w)| w.$wait_type).sum();
            contextarea
                .draw_series(AreaSeries::new(
                    wait_event_type
                        .iter()
                        .map(|(timestamp, w)| (*timestamp, (w.$wait_type $(+ w.$other_types)*) as f64)),
                    0.0,
                    wait_type_color(stringify!($wait_type)),
                ))
                .unwrap()
                .label(format!(
                    "{:25} {:10} {:10} {:10} {:10.2}",
                    stringify!($wait_type),
                    min,
                    max,
                    wait_event_type.back().map_or(0, |(_, r)| r.$wait_type),
                    if sum_all_activity == 0 {
                        0_f64
                    } else {
                        (sum as f64 / sum_all_activity as f64) * 100_f64
                    },
                ))
                .legend(move |(x, y)| {
                    Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], wait_type_color(stringify!($wait_type)).filled())
                });

            draw_wait_types!($($other_types,)*);
        }
    }
    draw_wait_types!(
        activity, buffer_pin, client, extension, timeout, ipc, lwlock, lock, io, on_cpu,
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
