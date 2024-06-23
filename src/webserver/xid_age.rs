use crate::DATA;
use crate::{
    CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE,
    LABEL_AREA_SIZE_BOTTOM, LABEL_AREA_SIZE_LEFT, LABEL_AREA_SIZE_RIGHT, MESH_STYLE_FONT,
    MESH_STYLE_FONT_SIZE,
};
use chrono::{DateTime, Local};
use futures::executor;
use plotters::backend::RGBPixel;
use plotters::chart::SeriesLabelPosition::UpperLeft;
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::full_palette::{
    BLUE_200, GREEN_200, GREY, ORANGE, ORANGE_200, PURPLE, PURPLE_200, RED_200,
};

pub fn xid_age(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
    start_time: Option<DateTime<Local>>,
    end_time: Option<DateTime<Local>>,
) {
    let xid_age = executor::block_on(DATA.pg_database_xid_limits.read());
    let final_start_time = if let Some(final_start_time) = start_time {
        final_start_time
    } else {
        xid_age
            .iter()
            .map(|(timestamp, _)| *timestamp)
            .min()
            .unwrap_or_default()
    };
    let final_end_time = if let Some(final_end_time) = end_time {
        final_end_time
    } else {
        xid_age
            .iter()
            .map(|(timestamp, _)| *timestamp)
            .max()
            .unwrap_or_default()
    };
    /*
        let start_time = xid_age
            .iter()
            .map(|(timestamp, _)| timestamp)
            .min()
            .unwrap();
        let end_time = xid_age
            .iter()
            .map(|(timestamp, _)| timestamp)
            .max()
            .unwrap();
    */
    let low_value_f64 = 0_f64;
    let high_value = 2_i64.pow(31) as f64 * 1.1_f64;

    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, LABEL_AREA_SIZE_LEFT)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .set_label_area_size(LabelAreaPosition::Right, LABEL_AREA_SIZE_RIGHT)
        .caption(
            "Transaction ID age",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(final_start_time..final_end_time, low_value_f64..high_value)
        .unwrap();
    contextarea
        .configure_mesh()
        .x_labels(6)
        .x_label_formatter(&|timestamp| timestamp.format("%Y-%m-%dT%H:%M:%S").to_string())
        .x_desc("Time")
        .y_desc("XID age")
        .y_label_formatter(&|age| format!("{}", *age))
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .draw()
        .unwrap();

    // This is a dummy plot for the sole intention to write a header in the legend.
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .take(1)
                .map(|(timestamp, d)| (*timestamp, d.age_datminmxid)),
            ShapeStyle {
                color: TRANSPARENT,
                filled: false,
                stroke_width: 1,
            },
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "", "min", "max", "last"
        ));

    let min_vmfma = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_multixact_freeze_min_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vmfma = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_multixact_freeze_min_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.vacuum_multixact_freeze_min_age)),
            ORANGE_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_multixact_freeze_min_age",
            min_vmfma,
            max_vmfma,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_multixact_freeze_min_age)
        ))
        .legend(move |(x, y)| {
            Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], ORANGE_200.filled())
        });
    let min_vmfta = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_multixact_freeze_table_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vmfta = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_multixact_freeze_table_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.vacuum_multixact_freeze_table_age)),
            GREEN_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_multixact_freeze_table_age",
            min_vmfta,
            max_vmfta,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_multixact_freeze_table_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN_200.filled()));
    let min_amfma = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.autovacuum_multixact_freeze_max_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_amfma = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.autovacuum_multixact_freeze_max_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.autovacuum_multixact_freeze_max_age)),
            BLUE_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "autovacuum_multixact_freeze_max_age",
            min_amfma,
            max_amfma,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.autovacuum_multixact_freeze_max_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE_200.filled()));
    let min_vmfa = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_multixact_failsafe_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vmfa = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_multixact_failsafe_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.vacuum_multixact_failsafe_age)),
            PURPLE_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_multixact_failsafe_age",
            min_vmfa,
            max_vmfa,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_multixact_failsafe_age)
        ))
        .legend(move |(x, y)| {
            Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE_200.filled())
        });
    let min_mxid_xid = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.age_datminmxid)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_mxid_xid = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.age_datminmxid)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.age_datminmxid)),
            ShapeStyle {
                color: GREY.into(),
                filled: true,
                stroke_width: 3,
            },
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "multixact XID",
            min_mxid_xid,
            max_mxid_xid,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.age_datminmxid)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREY.filled()));
    let min_vfma = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_freeze_min_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vfma = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_freeze_min_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.vacuum_freeze_min_age)),
            ORANGE,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_freeze_min_age",
            min_vfma,
            max_vfma,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_freeze_min_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], ORANGE.filled()));
    let min_vfta = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_freeze_table_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vfta = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_freeze_table_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.vacuum_freeze_table_age)),
            GREEN,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_freeze_table_age",
            min_vfta,
            max_vfta,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_freeze_table_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], GREEN.filled()));
    let min_amfa = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.autovacuum_freeze_max_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_amfa = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.autovacuum_freeze_max_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.autovacuum_freeze_max_age)),
            BLUE,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "autovacuum_freeze_max_age",
            min_amfa,
            max_amfa,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.autovacuum_freeze_max_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLUE.filled()));
    let min_vfa = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_failsafe_age)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_vfa = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.vacuum_failsafe_age)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.vacuum_failsafe_age)),
            PURPLE,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "vacuum_failsafe_age",
            min_vfa,
            max_vfa,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.vacuum_failsafe_age)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], PURPLE.filled()));
    let min_frozen_xid = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.age_datfrozenxid)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_frozen_xid = xid_age
        .iter()
        .filter(|(timestamp, _)| *timestamp >= final_start_time && *timestamp <= final_end_time)
        .map(|(_, d)| d.age_datfrozenxid)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, d)| (*timestamp, d.age_datfrozenxid)),
            ShapeStyle {
                color: BLACK.into(),
                filled: true,
                stroke_width: 3,
            },
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "Frozen XID",
            min_frozen_xid,
            max_frozen_xid,
            xid_age
                .iter()
                .last()
                .map_or(0_f64, |(_, b)| b.age_datfrozenxid)
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], BLACK.filled()));

    // readonly point
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, _)| (*timestamp, (2_i64.pow(31) - 3_000_000_i64) as f64)),
            RED_200,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "readonly point",
            "",
            "",
            2_i64.pow(31) - 3_000_000_i64
        ))
        .legend(move |(x, y)| Rectangle::new([(x - 3, y - 3), (x + 3, y + 3)], RED_200.filled()));
    // the absolute limit
    contextarea
        .draw_series(LineSeries::new(
            xid_age
                .iter()
                .filter(|(timestamp, _)| {
                    *timestamp >= final_start_time && *timestamp <= final_end_time
                })
                .map(|(timestamp, _)| (*timestamp, 2_i64.pow(31) as f64)),
            RED,
        ))
        .unwrap()
        .label(format!(
            "{:50} {:>10} {:>10} {:>10}",
            "absolute limit",
            "",
            "",
            2_i64.pow(31)
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
