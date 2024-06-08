use crate::{ARGS, DATA};
use crate::{
    CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE, LABELS_STYLE_FONT, LABELS_STYLE_FONT_SIZE,
    LABEL_AREA_SIZE_BOTTOM, MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use futures::executor;
use plotters::backend::RGBPixel;
use plotters::chart::SeriesLabelPosition::LowerRight;
use plotters::coord::Shift;
use plotters::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::ops::Bound::Included;
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

pub fn show_queries_query_html(query: &str) -> String {
    #[derive(Debug)]
    struct QueryidAndTotal {
        queryid: i64,
        total: usize,
    }
    let query = URL_SAFE.decode(query).unwrap();

    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    let mut samples_per_query: HashMap<String, QueryidAndTotal> =
        HashMap::with_capacity(pg_stat_activity.len());
    for per_sample_vector in pg_stat_activity.iter().map(|(_, v)| v) {
        for r in per_sample_vector.iter().filter(|r| {
            r.query.as_deref().unwrap_or_default() == String::from_utf8(query.clone()).unwrap()
        }) {
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
            query.query,
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
                <col style="width:80;">
                <col style="width:200;">
                <col style="width:80;">
                <col style="width:100px;">
                <col style="width:{}px;">
            </colgroup>
            <tr>
                <th align=right>Nr</th>
                <th align=right>Query ID</th>
                <th align=right>Percent</th>
                <th align=right>Total</th>
                <th>Query</th>
            </tr>"#,
        ARGS.graph_width - (80_u32 + 200_u32 + 80_u32 + 100_u32)
    );

    for (nr, query) in qc.iter().enumerate() {
        html_output += format!(
            r#"<tr>
                <td align=right>{:8}</td>
                <td align=right>{}</td>
                <td align=right>{:6.2}%</td>
                <td align=right>{:8}</td>
                <td>
                  <a href="/dual_handler/ash_wait_query_by_query/selected_queries/{}">{}</a>
                </td>
            </tr>"#,
            nr,
            query.queryid,
            query.total as f64 / grand_total_samples * 100_f64,
            query.total,
            URL_SAFE.encode(query.query.clone()),
            query.query
        )
        .as_str();
    }
    html_output += format!(
        "<tr>
                <td>{}</td>
                <td align=right>{:>20}</td>
                <td align=right>{:6.2}%</td>
                <td align=right>{:8}</td>
                <td>{}</td>
            </tr>",
        "", "total", 100_f64, grand_total_samples, ""
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
                <td>
                  <a href="/dual_handler/ash_wait_query_by_query/selected_queries/{}">{}</a>
                </td>
            </tr>"#,
            query.query_id,
            query.query_id,
            query.total as f64 / grand_total_samples * 100_f64,
            query.total,
            if query.query_id == 0 {
                "*".to_string()
            } else {
                URL_SAFE.encode(query.query.to_string())
            },
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
pub fn waits_by_query_text(
    multi_backend: &mut [DrawingArea<BitMapBackend<RGBPixel>, Shift>],
    backend_number: usize,
    queryid_filter: &bool,
    queryid: &i64,
) {
    let pg_stat_activity = executor::block_on(DATA.pg_stat_activity.read());
    let mut wait_event_counter: BTreeMap<String, usize> = BTreeMap::new();
    let mut query_waits: HashMap<String, BTreeMap<String, usize>> =
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
                query_waits
                    .entry(row.query.as_deref().unwrap_or_default().to_string())
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
    for (_, map) in query_waits.iter_mut() {
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
    for (_, waits) in query_waits.iter() {
        samples_max =
            samples_max.max(waits.iter().map(|(_, nr)| *nr as isize).sum::<isize>() as usize);
    }
    #[derive(Debug, Default)]
    struct DynamicQueryTextTotalWaits {
        _query: String,
        total: usize,
        others: bool,
        nr: usize,
        waits: BTreeMap<String, usize>,
    }

    let mut query_total_waits: Vec<DynamicQueryTextTotalWaits> =
        Vec::with_capacity(query_waits.len());

    for (query, waits) in query_waits.iter() {
        query_total_waits.push(DynamicQueryTextTotalWaits {
            _query: query.to_string(),
            waits: waits.clone(),
            others: false,
            nr: 0,
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
    query_total_waits.sort_by(|a, b| b.total.cmp(&a.total));
    for (nr, record) in query_total_waits.iter_mut().enumerate() {
        record.nr = nr;
    }

    let mut query_total_waits_count = if query_total_waits.len() > 1 {
        query_total_waits.len() - 1
    } else {
        query_total_waits.len()
    };

    let (_, y_size) = multi_backend[backend_number].dim_in_pixel();
    let qtw_max = y_size as usize / 20;

    if query_total_waits.len() > qtw_max {
        let mut others = DynamicQueryTextTotalWaits {
            ..Default::default()
        };
        others.total = query_total_waits.len() - qtw_max;
        others.others = true;
        query_total_waits.truncate(qtw_max);
        query_total_waits.push(others);
        query_total_waits_count = query_total_waits.len() - 1;
    }

    query_total_waits.reverse();

    // build the graph
    multi_backend[backend_number].fill(&WHITE).unwrap();
    let mut contextarea = ChartBuilder::on(&multi_backend[backend_number])
        .set_label_area_size(LabelAreaPosition::Left, 200)
        .set_label_area_size(LabelAreaPosition::Bottom, LABEL_AREA_SIZE_BOTTOM)
        .caption(
            "Query by number of samples",
            (CAPTION_STYLE_FONT, CAPTION_STYLE_FONT_SIZE),
        )
        .build_cartesian_2d(
            0..samples_max,
            (0..query_total_waits_count).into_segmented(),
        )
        .unwrap();
    contextarea
        .configure_mesh()
        .label_style((MESH_STYLE_FONT, MESH_STYLE_FONT_SIZE))
        .y_labels(query_total_waits_count)
        .y_label_formatter(&|v| {
            if query_total_waits
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
                    &query_total_waits
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
                format!(
                    "{}",
                    query_total_waits
                        .iter()
                        .map(|r| r.nr)
                        .nth({
                            if let SegmentValue::CenterOf(val) = v {
                                *val
                            } else {
                                0
                            }
                        })
                        .unwrap_or(0)
                )
            }
        })
        .x_desc("Samples")
        .x_label_formatter(&|n| n.to_string())
        .draw()
        .unwrap();
    //
    for (color_number, wait_event) in wait_event_counter.keys().enumerate() {
        contextarea
            .draw_series((0..).zip(query_total_waits.iter()).map(|(y, x)| {
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
