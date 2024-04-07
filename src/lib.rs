//use clap::{Parser, ValueEnum};
use clap::Parser;
use once_cell::sync::Lazy;
use bounded_vec_deque::BoundedVecDeque;
//use std::sync::RwLock;
use tokio::sync::RwLock;
use chrono::{DateTime, Local};

pub mod processor;
pub mod webserver;

use processor::PgStatActivity;
use processor::PgCurrentWaitTypes;

static LABEL_AREA_SIZE_LEFT: i32 = 100;
static LABEL_AREA_SIZE_RIGHT: i32 = 100;
static LABEL_AREA_SIZE_BOTTOM: i32 = 50;
static CAPTION_STYLE_FONT: &str = "monospace";
static CAPTION_STYLE_FONT_SIZE: i32 = 30;
static MESH_STYLE_FONT: &str = "monospace";
static MESH_STYLE_FONT_SIZE: i32 = 17;
static LABELS_STYLE_FONT: &str = "monospace";
static LABELS_STYLE_FONT_SIZE: i32 = 15;

#[derive(Debug, Parser, Clone)]
#[clap(version, about, long_about = None)]
pub struct Opts {
    /// Interval
    #[arg(short = 'i', long, value_name = "time (s)", default_value = "1")]
    pub interval: u64,
    /// History
    #[arg(short = 'n', long, value_name = "nr statistics", default_value = "10800")]
    pub history: usize,
    /// Webserver port
    #[arg(short = 'P', long, value_name = "webserver port", default_value = "1112")]
    pub webserver_port: u64,
        /// graph buffer width
    #[arg(short = 'W', long, value_name = "graph buffer width", default_value = "1800")]
    pub graph_width: u32,
    /// graph buffer heighth
    #[arg(short = 'H', long, value_name = "graph buffer height", default_value = "1200")]
    pub graph_height: u32,
}

pub static ARGS: Lazy<Opts> = Lazy::new(|| { Opts::parse() });

#[derive(Debug)]
pub struct Data {
    pub pg_stat_activity: RwLock<BoundedVecDeque<(DateTime<Local>, Vec<PgStatActivity>)>>,
    pub wait_event_types: RwLock<BoundedVecDeque<(DateTime<Local>, PgCurrentWaitTypes)>>,
}

impl Data {
    pub fn new(history: usize) -> Data {
        Data {
            pg_stat_activity: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_types: RwLock::new(BoundedVecDeque::new(history)),
        }
    }
}

pub static DATA: Lazy<Data> = Lazy::new(|| { Data::new(Opts::parse().history) });
