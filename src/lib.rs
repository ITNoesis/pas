//use clap::{Parser, ValueEnum};
use bounded_vec_deque::BoundedVecDeque;
use clap::Parser;
use once_cell::sync::Lazy;
//use std::sync::RwLock;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub mod archiver;
pub mod processor;
pub mod reader;
pub mod webserver;

use processor::{
    PgDatabaseXidLimits, PgStatActivity, PgStatBgWriterSum, PgStatDatabaseSum, PgStatWalSum,
    PgWaitTypeActivity, PgWaitTypeBufferPin, PgWaitTypeClient, PgWaitTypeExtension, PgWaitTypeIO,
    PgWaitTypeIPC, PgWaitTypeLWLock, PgWaitTypeLock, PgWaitTypeTimeout, PgWaitTypes,
};

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
    #[arg(
        short = 'n',
        long,
        value_name = "nr statistics",
        default_value = "10800"
    )]
    pub history: usize,
    /// Enable webserver
    #[arg(short = 'w', long, value_name = "enable webserver")]
    pub webserver: bool,
    /// Webserver port
    #[arg(
        short = 'P',
        long,
        value_name = "webserver port",
        default_value = "1112"
    )]
    pub webserver_port: u64,
    /// Enable archiver
    #[arg(short = 'A', long, value_name = "enable archiver")]
    pub archiver: bool,
    /// Archiver interval
    #[arg(
        short = 'I',
        long,
        value_name = "archiver interval (minutes)",
        default_value = "10"
    )]
    pub archiver_interval: i64,
    /// graph buffer width
    #[arg(
        short = 'W',
        long,
        value_name = "graph buffer width",
        default_value = "1400"
    )]
    pub graph_width: u32,
    /// graph buffer heighth
    #[arg(
        short = 'H',
        long,
        value_name = "graph buffer height",
        default_value = "1000"
    )]
    pub graph_height: u32,
    /// Read history file(s), don't do active fetching
    #[arg(short = 'r', long, value_name = "read archives")]
    pub read: Option<String>,
    /// Connection specification
    #[arg(
        short = 'c',
        long,
        value_name = "connection string",
        default_value = "postgres:///"
    )]
    pub connection_string: String,
}

pub static ARGS: Lazy<Opts> = Lazy::new(Opts::parse);

#[derive(Debug)]
pub struct Data {
    pub pg_stat_activity: RwLock<BoundedVecDeque<(DateTime<Local>, Vec<PgStatActivity>)>>,
    pub wait_event_types: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypes)>>,
    pub wait_event_activity: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeActivity)>>,
    pub wait_event_bufferpin: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeBufferPin)>>,
    pub wait_event_client: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeClient)>>,
    pub wait_event_extension: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeExtension)>>,
    pub wait_event_io: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeIO)>>,
    pub wait_event_ipc: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeIPC)>>,
    pub wait_event_lock: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeLock)>>,
    pub wait_event_lwlock: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeLWLock)>>,
    pub wait_event_timeout: RwLock<BoundedVecDeque<(DateTime<Local>, PgWaitTypeTimeout)>>,
    pub pg_stat_database_sum: RwLock<BoundedVecDeque<(DateTime<Local>, PgStatDatabaseSum)>>,
    pub pg_stat_bgwriter_sum: RwLock<BoundedVecDeque<(DateTime<Local>, PgStatBgWriterSum)>>,
    pub pg_stat_wal_sum: RwLock<BoundedVecDeque<(DateTime<Local>, PgStatWalSum)>>,
    pub pg_database_xid_limits: RwLock<BoundedVecDeque<(DateTime<Local>, PgDatabaseXidLimits)>>,
}

impl Data {
    pub fn new(history: usize) -> Data {
        Data {
            pg_stat_activity: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_types: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_activity: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_bufferpin: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_client: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_extension: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_io: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_ipc: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_lock: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_lwlock: RwLock::new(BoundedVecDeque::new(history)),
            wait_event_timeout: RwLock::new(BoundedVecDeque::new(history)),
            pg_stat_database_sum: RwLock::new(BoundedVecDeque::new(history)),
            pg_stat_bgwriter_sum: RwLock::new(BoundedVecDeque::new(history)),
            pg_stat_wal_sum: RwLock::new(BoundedVecDeque::new(history)),
            pg_database_xid_limits: RwLock::new(BoundedVecDeque::new(history)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DataTransit {
    pub pg_stat_activity: Vec<(DateTime<Local>, Vec<PgStatActivity>)>,
    pub wait_event_types: Vec<(DateTime<Local>, PgWaitTypes)>,
    pub wait_event_activity: Vec<(DateTime<Local>, PgWaitTypeActivity)>,
    pub wait_event_bufferpin: Vec<(DateTime<Local>, PgWaitTypeBufferPin)>,
    pub wait_event_client: Vec<(DateTime<Local>, PgWaitTypeClient)>,
    pub wait_event_extension: Vec<(DateTime<Local>, PgWaitTypeExtension)>,
    pub wait_event_io: Vec<(DateTime<Local>, PgWaitTypeIO)>,
    pub wait_event_ipc: Vec<(DateTime<Local>, PgWaitTypeIPC)>,
    pub wait_event_lock: Vec<(DateTime<Local>, PgWaitTypeLock)>,
    pub wait_event_lwlock: Vec<(DateTime<Local>, PgWaitTypeLWLock)>,
    pub wait_event_timeout: Vec<(DateTime<Local>, PgWaitTypeTimeout)>,
    pub pg_stat_database_sum: Vec<(DateTime<Local>, PgStatDatabaseSum)>,
    pub pg_stat_bgwriter_sum: Vec<(DateTime<Local>, PgStatBgWriterSum)>,
    pub pg_stat_wal_sum: Vec<(DateTime<Local>, PgStatWalSum)>,
    pub pg_database_xid_limits: Vec<(DateTime<Local>, PgDatabaseXidLimits)>,
}

pub static DATA: Lazy<Data> = Lazy::new(|| Data::new(Opts::parse().history));
