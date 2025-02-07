use crate::DATA;
use crate::{
    CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE,
    LABEL_AREA_SIZE_BOTTOM, LABEL_AREA_SIZE_LEFT, LABEL_AREA_SIZE_RIGHT, MESH_STYLE_FONT,
    MESH_STYLE_FONT_SIZE,
};
use chrono::{DateTime, Local};
use full_palette::{GREEN_800, LIGHTBLUE, RED_300};
use futures::executor;
use human_bytes::human_bytes;
use plotters::backend::RGBPixel;
use plotters::chart::SeriesLabelPosition::UpperLeft;
use plotters::coord::Shift;
use plotters::prelude::*;

pub fn wal_io_times(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
    start_time: Option<DateTime<Local>>,
    end_time: Option<DateTime<Local>>,
) {
    let wal_events = executor::block_on(DATA.pg_stat_wal_sum.read());
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
    let found_start_time = wal_start_time.min(bgwriter_start_time);
    let found_end_time = wal_end_time.min(bgwriter_end_time);

    let final_start_time = if let Some(final_start_time) = start_time {
        final_start_time
    } else {
        *found_start_time
    };
    let final_end_time = if let Some(final_end_time) = end_time {
        final_end_time
    } else {
        *found_end_time
    };
    let low_value_f64 = 0_f64;
    let high_value_write = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, w)| {
            if w.wal_buffers_full_ps + w.wal_write_ps == 0_f64 {
                0_f64
            } else {
                w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps)
            }
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let high_value_sync = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
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
        .build_cartesian_2d(final_start_time..final_end_time, low_value_f64..high_value)
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

    // checkpoints timed
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, b)| b.checkpoints_timed > 0_f64)
                .map(|(timestamp, _)| TriangleMarker::new((*timestamp, high_value), 5, GREEN_800)),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10}",
            "checkpoints_timed",
            bgwriter_events
                .iter()
                .filter(
                    |(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time
                )
                .map(|(_, b)| b.checkpoints_timed)
                .sum::<f64>()
        ))
        .legend(move |(x, y)| TriangleMarker::new((x, y), 5, GREEN_800.filled()));
    // checkpoints req
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, b)| b.checkpoints_req > 0_f64)
                .map(|(timestamp, _)| TriangleMarker::new((*timestamp, high_value), 5, RED)),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10}",
            "checkpoints_req",
            bgwriter_events
                .iter()
                .map(|(_, b)| b.checkpoints_req)
                .sum::<f64>()
        ))
        .legend(move |(x, y)| TriangleMarker::new((x, y), 5, RED.filled()));
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

    let min_write = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, w)| {
            w.wal_buffers_full_ps + w.wal_write_ps > 0_f64 && w.wal_write_time_ps > 0_f64
        })
        .map(|(_, w)| w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps))
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_write = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, w)| {
            w.wal_buffers_full_ps + w.wal_write_ps > 0_f64 && w.wal_write_time_ps > 0_f64
        })
        .map(|(_, w)| w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps))
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            wal_events
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, w)| {
                    w.wal_buffers_full_ps + w.wal_write_ps > 0_f64 && w.wal_write_time_ps > 0_f64
                })
                .map(|(timestamp, w)| {
                    Circle::new(
                        (
                            *timestamp,
                            w.wal_write_time_ps / (w.wal_buffers_full_ps + w.wal_write_ps),
                        ),
                        4,
                        BLACK,
                    )
                }),
        )
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
        .legend(move |(x, y)| Circle::new((x, y), 4, BLACK.filled()));

    let min_sync = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, w)| w.wal_sync_ps > 0_f64 && w.wal_sync_time_ps > 0_f64)
        .map(|(_, w)| w.wal_sync_time_ps / w.wal_sync_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_sync = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, w)| w.wal_sync_ps > 0_f64 && w.wal_sync_time_ps > 0_f64)
        .map(|(_, w)| w.wal_sync_time_ps / w.wal_sync_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            wal_events
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, w)| w.wal_sync_ps > 0_f64 && w.wal_sync_time_ps > 0_f64)
                .map(|(timestamp, w)| {
                    Circle::new(
                        (*timestamp, w.wal_sync_time_ps / w.wal_sync_ps),
                        3,
                        LIGHTBLUE,
                    )
                }),
        )
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
        .legend(move |(x, y)| Circle::new((x, y), 3, LIGHTBLUE.filled()));

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
    start_time: Option<DateTime<Local>>,
    end_time: Option<DateTime<Local>>,
) {
    let bgwriter_events = executor::block_on(DATA.pg_stat_bgwriter_sum.read());
    let wal_events = executor::block_on(DATA.pg_stat_wal_sum.read());
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
    let found_start_time = wal_start_time.min(bgwriter_start_time);
    let found_end_time = wal_end_time.min(bgwriter_end_time);
    let final_start_time = if let Some(final_start_time) = start_time {
        final_start_time
    } else {
        *found_start_time
    };
    let final_end_time = if let Some(final_end_time) = end_time {
        final_end_time
    } else {
        *found_end_time
    };
    let low_value_f64 = 0_f64;
    let high_value_bytes = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
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
        .build_cartesian_2d(
            final_start_time..final_end_time,
            low_value_f64..high_value_bytes,
        )
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

    // checkpoints timed
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, b)| b.checkpoints_timed > 0_f64)
                .map(|(timestamp, _)| {
                    TriangleMarker::new((*timestamp, high_value_bytes), 5, GREEN_800)
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10}",
            "checkpoints_timed",
            bgwriter_events
                .iter()
                .map(|(_, b)| b.checkpoints_timed)
                .sum::<f64>()
        ))
        .legend(move |(x, y)| TriangleMarker::new((x, y), 5, GREEN_800.filled()));
    // checkpoints req
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, b)| b.checkpoints_req > 0_f64)
                .map(|(timestamp, _)| {
                    TriangleMarker::new((*timestamp, high_value_bytes), 5, RED_300)
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10}",
            "checkpoints_req",
            bgwriter_events
                .iter()
                .map(|(_, b)| b.checkpoints_req)
                .sum::<f64>()
        ))
        .legend(move |(x, y)| TriangleMarker::new((x, y), 5, RED_300.filled()));
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
    //
    let min_write = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, w)| w.wal_bytes_ps > 0_f64)
        .map(|(_, w)| w.wal_bytes_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_write = wal_events
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, w)| w.wal_bytes_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            wal_events
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, w)| w.wal_bytes_ps > 0_f64)
                .map(|(timestamp, w)| Circle::new((*timestamp, w.wal_bytes_ps), 3, BLACK.filled())),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10} {:>10} {:>10}",
            "Wal size",
            human_bytes(min_write),
            human_bytes(max_write),
            human_bytes(wal_events.back().map_or(0_f64, |(_, r)| r.wal_bytes_ps))
        ))
        .legend(move |(x, y)| Circle::new((x, y), 3, BLACK.filled()));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
