use crate::DATA;
use crate::{
    CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE,
    LABEL_AREA_SIZE_BOTTOM, LABEL_AREA_SIZE_LEFT, LABEL_AREA_SIZE_RIGHT, MESH_STYLE_FONT,
    MESH_STYLE_FONT_SIZE,
};
use full_palette::{AMBER, DEEPORANGE};
use futures::executor;
use human_bytes::human_bytes;
use plotters::backend::RGBPixel;
use plotters::element::Circle;
use plotters::prelude::full_palette::PURPLE;
use plotters::prelude::*;
use plotters::{chart::SeriesLabelPosition::UpperLeft, style::full_palette::GREEN_800};
use plotters::{coord::Shift, style::full_palette::RED_300};

pub fn iops(
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
    let high_value_io = wal_events
        .iter()
        .zip(bgwriter_events.iter())
        .zip(database_events.iter())
        .map(|(((_, w), (_, b)), (_, d))| {
            w.wal_buffers_full_ps
                + w.wal_write_ps
                + b.buffers_checkpoint_ps
                + b.buffers_clean_ps
                + b.buffers_backend_ps
                + d.blks_read_ps
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let high_value_wal_sync = wal_events
        .iter()
        .filter(|(_, w)| w.wal_sync_ps > 0_f64)
        .map(|(_, w)| w.wal_sync_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let high_value = high_value_io.max(high_value_wal_sync) * 1.1_f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption("IOPS", (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE))
        .build_cartesian_2d(*start_time..*end_time, low_value_f64..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("IOPS")
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // checkpoints timed
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(_, b)| b.checkpoints_timed > 0_f64)
                .map(|(timestamp, _)| TriangleMarker::new((*timestamp, high_value), 5, GREEN_800)),
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
                .filter(|(_, b)| b.checkpoints_req > 0_f64)
                .map(|(timestamp, _)| TriangleMarker::new((*timestamp, high_value), 5, RED_300)),
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
    //
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
    // wal buffers full
    let min_wal_buffers_full = wal_events
        .iter()
        .filter(|(_, w)| w.wal_buffers_full_ps > 0_f64)
        .map(|(_, w)| w.wal_buffers_full_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_wal_buffers_full = wal_events
        .iter()
        .map(|(_, w)| w.wal_buffers_full_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            wal_events
                .iter()
                .filter(|(_, w)| w.wal_buffers_full_ps > 0_f64)
                .map(|(timestamp, w)| {
                    Circle::new((*timestamp, w.wal_buffers_full_ps), 4, PURPLE.filled())
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:10.2} {:10.2} {:10.2}",
            "wal_buffers_full",
            min_wal_buffers_full,
            max_wal_buffers_full,
            wal_events
                .back()
                .map_or(0_f64, |(_, r)| r.wal_buffers_full_ps)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE.filled()));
    // wal write
    let min_wal_write = wal_events
        .iter()
        .filter(|(_, w)| w.wal_write_ps > 0_f64)
        .map(|(_, w)| w.wal_write_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_wal_write = wal_events
        .iter()
        .map(|(_, w)| w.wal_write_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            wal_events
                .iter()
                .filter(|(_, w)| w.wal_write_ps > 0_f64)
                .map(|(timestamp, w)| {
                    Circle::new((*timestamp, w.wal_write_ps), 3, DEEPORANGE.filled())
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:10.2} {:10.2} {:10.2}",
            "wal_write",
            min_wal_write,
            max_wal_write,
            wal_events.back().map_or(0_f64, |(_, r)| r.wal_write_ps)
        ))
        .legend(move |(x, y)| {
            Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], DEEPORANGE.filled())
        });
    /*
        // wal sync
        let min_sync = wal_events
            .iter()
            .filter(|(_, w)| w.wal_sync_ps > 0_f64)
            .map(|(_, w)| w.wal_sync_ps)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();
        let max_sync = wal_events
            .iter()
            .map(|(_, w)| w.wal_sync_ps)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default();
        contextarea
            .draw_series(
                wal_events
                    .iter()
                    .filter(|(_, w)| w.wal_sync_ps > 0_f64)
                    .map(|(timestamp, w)| Cross::new((*timestamp, w.wal_sync_ps), 4, BLUE.filled())),
            )
            .unwrap()
            .label(format!(
                "{:25} {:10.2} {:10.2} {:10.2}",
                "wal_sync",
                min_sync,
                max_sync,
                wal_events.back().map_or(0_f64, |(_, r)| r.wal_sync_ps)
            ))
            .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE.filled()));
    */
    // database blocks read
    let min_blocks_read = database_events
        .iter()
        .filter(|(_, d)| d.blks_read_ps > 0_f64)
        .map(|(_, d)| d.blks_read_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_blocks_read = database_events
        .iter()
        .map(|(_, d)| d.blks_read_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            database_events
                .iter()
                .filter(|(_, d)| d.blks_read_ps > 0_f64)
                .map(|(timestamp, d)| Circle::new((*timestamp, d.blks_read_ps), 2, GREEN.filled())),
        )
        .unwrap()
        .label(format!(
            "{:25} {:10.2} {:10.2} {:10.2}",
            "blks_read",
            min_blocks_read,
            max_blocks_read,
            database_events
                .back()
                .map_or(0_f64, |(_, d)| d.blks_read_ps)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));
    // bgwriter buffers written backend
    let min_buf_backend = bgwriter_events
        .iter()
        .filter(|(_, b)| b.buffers_backend_ps > 0_f64)
        .map(|(_, b)| b.buffers_backend_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_buf_backend = bgwriter_events
        .iter()
        .map(|(_, b)| b.buffers_backend_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(_, b)| b.buffers_backend_ps > 0_f64)
                .map(|(timestamp, b)| {
                    Circle::new((*timestamp, b.buffers_backend_ps), 4, RED.filled())
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:10.2} {:10.2} {:10.2}",
            "buffers_backend",
            min_buf_backend,
            max_buf_backend,
            bgwriter_events
                .back()
                .map_or(0_f64, |(_, b)| b.buffers_backend_ps)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED.filled()));
    // bgwriter buffers clean
    let min_buf_clean = bgwriter_events
        .iter()
        .filter(|(_, b)| b.buffers_clean_ps > 0_f64)
        .map(|(_, b)| b.buffers_clean_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_buf_clean = bgwriter_events
        .iter()
        .map(|(_, b)| b.buffers_clean_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(_, b)| b.buffers_clean_ps > 0_f64)
                .map(|(timestamp, b)| {
                    Circle::new((*timestamp, b.buffers_clean_ps), 4, AMBER.filled())
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:10.2} {:10.2} {:10.2}",
            "buffers_clean",
            min_buf_clean,
            max_buf_clean,
            bgwriter_events
                .back()
                .map_or(0_f64, |(_, b)| b.buffers_clean_ps)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], AMBER.filled()));
    // bgwriter buffers checkpoint
    let min_buf_checkpoint = bgwriter_events
        .iter()
        .filter(|(_, b)| b.buffers_checkpoint_ps > 0_f64)
        .map(|(_, b)| b.buffers_checkpoint_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_buf_checkpoint = bgwriter_events
        .iter()
        .map(|(_, b)| b.buffers_checkpoint_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(_, b)| b.buffers_checkpoint_ps > 0_f64)
                .map(|(timestamp, b)| {
                    Circle::new((*timestamp, b.buffers_checkpoint_ps), 4, BLUE.filled())
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:10.2} {:10.2} {:10.2}",
            "buffers_checkpoint",
            min_buf_checkpoint,
            max_buf_checkpoint,
            bgwriter_events
                .back()
                .map_or(0_f64, |(_, b)| b.buffers_checkpoint_ps)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE.filled()));
    // total IO
    let min_tot_io = wal_events
        .iter()
        .zip(bgwriter_events.iter())
        .zip(database_events.iter())
        .map(|(((_, w), (_, b)), (_, d))| {
            w.wal_buffers_full_ps
                + w.wal_write_ps
                + b.buffers_checkpoint_ps
                + b.buffers_clean_ps
                + b.buffers_backend_ps
                + d.blks_read_ps
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_tot_io = wal_events
        .iter()
        .zip(bgwriter_events.iter())
        .zip(database_events.iter())
        .map(|(((_, w), (_, b)), (_, d))| {
            w.wal_buffers_full_ps
                + w.wal_write_ps
                + b.buffers_checkpoint_ps
                + b.buffers_clean_ps
                + b.buffers_backend_ps
                + d.blks_read_ps
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(LineSeries::new(
            wal_events
                .iter()
                .zip(bgwriter_events.iter())
                .zip(database_events.iter())
                .map(|(((timestamp, w), (_, b)), (_, d))| {
                    (
                        *timestamp,
                        w.wal_buffers_full_ps
                            + w.wal_write_ps
                            + b.buffers_checkpoint_ps
                            + b.buffers_clean_ps
                            + b.buffers_backend_ps
                            + d.blks_read_ps,
                    )
                }),
            BLACK,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:10.2} {:10.2} {:10.2}",
            "IO total",
            min_tot_io,
            max_tot_io,
            bgwriter_events
                .back()
                .map_or(0_f64, |(_, b)| b.buffers_checkpoint_ps
                    + b.buffers_clean_ps
                    + b.buffers_backend_ps)
                + wal_events
                    .back()
                    .map_or(0_f64, |(_, w)| w.wal_buffers_full_ps + w.wal_write_ps)
                + database_events
                    .back()
                    .map_or(0_f64, |(_, d)| d.blks_read_ps)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLACK.filled()));
    /*
    let high_value_io = wal_events
        .iter()
        .zip(bgwriter_events.iter())
        .zip(database_events.iter())
        .map(|(((_, w), (_, b)), (_, d))| {
            w.wal_buffers_full_ps
                + w.wal_write_ps
                + b.buffers_checkpoint_ps
                + b.buffers_clean_ps
                + b.buffers_backend_ps
                + d.blks_read_ps
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
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
    */

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

    // checkpoints timed
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(_, b)| b.checkpoints_timed > 0_f64)
                .map(|(timestamp, _)| TriangleMarker::new((*timestamp, high_value), 5, GREEN_800)),
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
                .filter(|(_, b)| b.checkpoints_req > 0_f64)
                .map(|(timestamp, _)| TriangleMarker::new((*timestamp, high_value), 5, RED_300)),
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
    //
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
    // checkpoints timed
    contextarea
        .draw_series(
            bgwriter_events
                .iter()
                .filter(|(_, b)| b.checkpoints_timed > 0_f64)
                .map(|(timestamp, _)| TriangleMarker::new((*timestamp, high_value), 5, GREEN_800)),
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
                .filter(|(_, b)| b.checkpoints_req > 0_f64)
                .map(|(timestamp, _)| TriangleMarker::new((*timestamp, high_value), 5, RED_300)),
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

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
