use crate::DATA;
use crate::{
    CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE,
    LABEL_AREA_SIZE_BOTTOM, LABEL_AREA_SIZE_LEFT, LABEL_AREA_SIZE_RIGHT, MESH_STYLE_FONT,
    MESH_STYLE_FONT_SIZE,
};
use chrono::{DateTime, Local};
use full_palette::ORANGE;
use futures::executor;
use plotters::backend::RGBPixel;
use plotters::chart::SeriesLabelPosition::UpperLeft;
use plotters::coord::Shift;
use plotters::prelude::*;

pub fn tuples_processed(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
    start_time: Option<DateTime<Local>>,
    end_time: Option<DateTime<Local>>,
) {
    let pg_stat_database = executor::block_on(DATA.pg_stat_database_sum.read());
    let final_start_time = if let Some(final_start_time) = start_time {
        final_start_time
    } else {
        pg_stat_database
            .iter()
            .map(|(timestamp, _)| *timestamp)
            .min()
            .unwrap_or_default()
    };
    let final_end_time = if let Some(final_end_time) = end_time {
        final_end_time
    } else {
        pg_stat_database
            .iter()
            .map(|(timestamp, _)| *timestamp)
            .max()
            .unwrap_or_default()
    };
    /*
        let start_time = pg_stat_database
            .iter()
            .map(|(timestamp, _)| timestamp)
            .min()
            .unwrap();
        let end_time = pg_stat_database
            .iter()
            .map(|(timestamp, _)| timestamp)
            .max()
            .unwrap();
    */
    let low_value = 0_f64;
    let high_value = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.tup_fetched_ps + d.tup_inserted_ps + d.tup_updated_ps + d.tup_deleted_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default()
        * 1.1_f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption("Tuples", (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE))
        .build_cartesian_2d(final_start_time..final_end_time, low_value..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("Tuples per second")
        .y_label_formatter(&|age| format!("{}", *age))
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            pg_stat_database
                .iter()
                .take(1)
                .map(|(timestamp, d)| (*timestamp, d.tup_fetched_ps)),
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
    let min_fetched = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, d)| d.tup_fetched_ps > 0_f64)
        .map(|(_, d)| d.tup_fetched_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_fetched = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.tup_fetched_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            pg_stat_database
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, d)| d.tup_fetched_ps > 0_f64)
                .map(|(timestamp, d)| {
                    Circle::new((*timestamp, d.tup_fetched_ps), 3, GREEN.filled())
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10.0} {:>10.0} {:>10.0}",
            "tuples fetched",
            min_fetched,
            max_fetched,
            pg_stat_database
                .back()
                .map_or(0_f64, |(_, d)| d.tup_fetched_ps)
        ))
        .legend(move |(x, y)| Circle::new((x, y), 3, GREEN.filled()));
    let min_inserted = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, d)| d.tup_inserted_ps > 0_f64)
        .map(|(_, d)| d.tup_inserted_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_inserted = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.tup_inserted_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            pg_stat_database
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, d)| d.tup_inserted_ps > 0_f64)
                .map(|(timestamp, d)| {
                    Circle::new((*timestamp, d.tup_inserted_ps), 3, BLUE.filled())
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10.0} {:>10.0} {:>10.0}",
            "tuples inserted",
            min_inserted,
            max_inserted,
            pg_stat_database
                .back()
                .map_or(0_f64, |(_, d)| d.tup_inserted_ps)
        ))
        .legend(move |(x, y)| Circle::new((x, y), 3, BLUE.filled()));
    let min_updated = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, d)| d.tup_updated_ps > 0_f64)
        .map(|(_, d)| d.tup_updated_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_updated = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.tup_updated_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            pg_stat_database
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, d)| d.tup_updated_ps > 0_f64)
                .map(|(timestamp, d)| {
                    Circle::new((*timestamp, d.tup_updated_ps), 3, ORANGE.filled())
                }),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10.0} {:>10.0} {:>10.0}",
            "tuples updated",
            min_updated,
            max_updated,
            pg_stat_database
                .back()
                .map_or(0_f64, |(_, d)| d.tup_updated_ps)
        ))
        .legend(move |(x, y)| Circle::new((x, y), 3, ORANGE.filled()));
    let min_deleted = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, d)| d.tup_deleted_ps > 0_f64)
        .map(|(_, d)| d.tup_deleted_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_deleted = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.tup_deleted_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(
            pg_stat_database
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .filter(|(_, d)| d.tup_deleted_ps > 0_f64)
                .map(|(timestamp, d)| Circle::new((*timestamp, d.tup_deleted_ps), 3, RED.filled())),
        )
        .unwrap()
        .label(format!(
            "{:25} {:>10.0} {:>10.0} {:>10.0}",
            "tuples deleted",
            min_deleted,
            max_deleted,
            pg_stat_database
                .back()
                .map_or(0_f64, |(_, d)| d.tup_deleted_ps)
        ))
        .legend(move |(x, y)| Circle::new((x, y), 3, RED.filled()));

    let min_total = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .filter(|(_, d)| {
            d.tup_fetched_ps + d.tup_inserted_ps + d.tup_updated_ps + d.tup_deleted_ps > 0_f64
        })
        .map(|(_, d)| d.tup_fetched_ps + d.tup_inserted_ps + d.tup_updated_ps + d.tup_deleted_ps)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    let max_total = pg_stat_database
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.tup_fetched_ps + d.tup_inserted_ps + d.tup_updated_ps + d.tup_deleted_ps)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or_default();
    contextarea
        .draw_series(LineSeries::new(
            pg_stat_database
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| {
                    (
                        *timestamp,
                        d.tup_fetched_ps + d.tup_inserted_ps + d.tup_updated_ps + d.tup_deleted_ps,
                    )
                }),
            BLACK,
        ))
        .unwrap()
        .label(format!(
            "{:25} {:>10.0} {:>10.0} {:>10.0}",
            "tuples total",
            min_total,
            max_total,
            pg_stat_database
                .back()
                .map_or(0_f64, |(_, d)| d.tup_fetched_ps
                    + d.tup_inserted_ps
                    + d.tup_updated_ps
                    + d.tup_deleted_ps)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLACK.filled()));

    contextarea
        .configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.7))
        .label_font((LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE))
        .position(UpperLeft)
        .draw()
        .unwrap();
}
