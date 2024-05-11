use crate::ARGS;
use crate::DATA;

use anyhow::Result;
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Local};
use once_cell::sync::Lazy;
use sqlx::{
    postgres::{types::Oid, types::PgInterval, PgPoolOptions},
    query_as,
    types::BigDecimal,
    FromRow, Pool,
};
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::RwLock,
    time::{self, MissedTickBehavior},
};

// this pg_stat_activity is consistent with postgres version 15
#[derive(Debug, FromRow, Clone)]
pub struct PgStatActivity {
    pub timestamp: DateTime<Local>,
    pub datid: Option<Oid>,
    pub datname: Option<String>,
    pub pid: i32,
    pub leader_pid: Option<i32>,
    pub usesysid: Option<Oid>,
    pub usename: Option<String>,
    pub application_name: Option<String>,
    pub client_addr: Option<String>,
    pub client_hostname: Option<String>,
    pub client_port: Option<i32>,
    pub backend_time: Option<PgInterval>,
    pub xact_time: Option<PgInterval>,
    pub query_time: Option<PgInterval>,
    pub state_time: Option<PgInterval>,
    pub wait_event_type: Option<String>,
    pub wait_event: Option<String>,
    pub state: Option<String>,
    pub backend_xid: Option<i32>,
    pub backend_xmin: Option<i32>,
    pub query_id: Option<i64>,
    pub query: Option<String>,
    pub backend_type: Option<String>,
}

impl PgStatActivity {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_stat_activity = PgStatActivity::query(pool).await;
        let current_timestamp = Local::now();
        DATA.pg_stat_activity
            .write()
            .await
            .push_back((current_timestamp, pg_stat_activity.clone()));
        DATA.wait_event_types.write().await.push_back((
            current_timestamp,
            PgCurrentWaitTypes::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_activity.write().await.push_back((
            current_timestamp,
            PgWaitTypeActivity::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_bufferpin.write().await.push_back((
            current_timestamp,
            PgWaitTypeBufferPin::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_client.write().await.push_back((
            current_timestamp,
            PgWaitTypeClient::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_extension.write().await.push_back((
            current_timestamp,
            PgWaitTypeExtension::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_io.write().await.push_back((
            current_timestamp,
            PgWaitTypeIO::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_ipc.write().await.push_back((
            current_timestamp,
            PgWaitTypeIPC::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_lock.write().await.push_back((
            current_timestamp,
            PgWaitTypeLock::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_lwlock.write().await.push_back((
            current_timestamp,
            PgWaitTypeLWLock::process_pg_stat_activity(pg_stat_activity.clone()).await,
        ));
        DATA.wait_event_timeout.write().await.push_back((
            current_timestamp,
            PgWaitTypeTimeout::process_pg_stat_activity(pg_stat_activity).await,
        ));
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgStatActivity> {
        let mut sql_rows: Vec<PgStatActivity> = query_as(
            "
            select clock_timestamp() as timestamp,
                   datid, 
                   datname, 
                   pid,
                   leader_pid,
                   usesysid,
                   usename, 
                   application_name, 
                   client_addr,
                   client_hostname,
                   client_port,
                   clock_timestamp()-backend_start as backend_time, 
                   clock_timestamp()-xact_start as xact_time, 
                   clock_timestamp()-query_start as query_time, 
                   clock_timestamp()-state_change as state_time, 
                   lower(wait_event_type) as wait_event_type,
                   lower(wait_event) as wait_event,
                   state, 
                   backend_xid::text::int,
                   backend_xmin::text::int,
                   query_id, 
                   query, 
                   backend_type 
             from  pg_stat_activity 
             where pid != pg_backend_pid() 
        ",
        )
        .fetch_all(pool)
        .await
        .expect("error executing query");
        sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        sql_rows.reverse();
        sql_rows
    }
}

#[derive(Debug)]
pub struct PgCurrentWaitTypes {
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

impl PgCurrentWaitTypes {
    pub async fn process_pg_stat_activity(
        pg_stat_activity: Vec<PgStatActivity>,
    ) -> PgCurrentWaitTypes {
        let on_cpu = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.is_none())
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let activity = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "activity")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let buffer_pin = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "bufferpin")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let client = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "client")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let extension = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "extension")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let io = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "io")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let ipc = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "ipc")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let lock = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "lock")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let lwlock = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "lwlock")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let timeout = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "timeout")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        PgCurrentWaitTypes {
            on_cpu,
            activity,
            buffer_pin,
            client,
            extension,
            io,
            ipc,
            lock,
            lwlock,
            timeout,
        }
    }
}

#[derive(Debug, Default)]
pub struct PgWaitTypeActivity {
    pub archivermain: usize,
    pub autovacuummain: usize,
    pub bgwriterhibernate: usize,
    pub bgwritermain: usize,
    pub checkpointermain: usize,
    pub logicalapplymain: usize,
    pub logicallaunchermain: usize,
    pub logicalparallelapplymain: usize,
    pub recoverywalstream: usize,
    pub sysloggermain: usize,
    pub walreceivermain: usize,
    pub walsendermain: usize,
    pub walwritermain: usize,
    pub other: usize,
}
impl PgWaitTypeActivity {
    pub async fn new() -> Self {
        PgWaitTypeActivity::default()
    }
    pub async fn process_pg_stat_activity(
        pg_stat_activity: Vec<PgStatActivity>,
    ) -> PgWaitTypeActivity {
        let mut pgwaittypeactivity = PgWaitTypeActivity::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("activity".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "archivermain" => pgwaittypeactivity.archivermain += 1,
                    "autovacuummain" => pgwaittypeactivity.autovacuummain += 1,
                    "bgwriterhibernate" => pgwaittypeactivity.bgwriterhibernate += 1,
                    "bgwritermain" => pgwaittypeactivity.bgwritermain += 1,
                    "checkpointermain" => pgwaittypeactivity.checkpointermain += 1,
                    "logicalapplymain" => pgwaittypeactivity.logicalapplymain += 1,
                    "logicallaunchermain" => pgwaittypeactivity.logicallaunchermain += 1,
                    "logicalparallelapplymain" => pgwaittypeactivity.logicalparallelapplymain += 1,
                    "recoverywalstream" => pgwaittypeactivity.recoverywalstream += 1,
                    "sysloggermain" => pgwaittypeactivity.sysloggermain += 1,
                    "walreceivermain" => pgwaittypeactivity.walreceivermain += 1,
                    "walsendermain" => pgwaittypeactivity.walsendermain += 1,
                    "walwritermain" => pgwaittypeactivity.walwritermain += 1,
                    &_ => pgwaittypeactivity.other += 1,
                };
            });
        pgwaittypeactivity
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeBufferPin {
    pub bufferpin: usize,
    pub other: usize,
}
impl PgWaitTypeBufferPin {
    pub async fn new() -> Self {
        PgWaitTypeBufferPin::default()
    }
    pub async fn process_pg_stat_activity(
        pg_stat_activity: Vec<PgStatActivity>,
    ) -> PgWaitTypeBufferPin {
        let mut pgwaittypebufferpin = PgWaitTypeBufferPin::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("bufferpin".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "bufferpin" => pgwaittypebufferpin.bufferpin += 1,
                    &_ => pgwaittypebufferpin.other += 1,
                };
            });
        pgwaittypebufferpin
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeClient {
    pub clientread: usize,
    pub clientwrite: usize,
    pub gssopenserver: usize,
    pub libpqwalreceiverconnect: usize,
    pub libpqwalreceiverreceive: usize,
    pub sslopenserver: usize,
    pub walsenderwaitforwal: usize,
    pub walsenderwritedata: usize,
    pub other: usize,
}
impl PgWaitTypeClient {
    pub async fn new() -> Self {
        PgWaitTypeClient::default()
    }
    pub async fn process_pg_stat_activity(
        pg_stat_activity: Vec<PgStatActivity>,
    ) -> PgWaitTypeClient {
        let mut pgwaittypeclient = PgWaitTypeClient::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("client".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "clientread" => pgwaittypeclient.clientread += 1,
                    "clientwrite" => pgwaittypeclient.clientwrite += 1,
                    "gssopenserver" => pgwaittypeclient.gssopenserver += 1,
                    "libpqwalreceiverconnect" => pgwaittypeclient.libpqwalreceiverconnect += 1,
                    "Libpqwalreceiverreceive" => pgwaittypeclient.libpqwalreceiverreceive += 1,
                    "sslopenserver" => pgwaittypeclient.sslopenserver += 1,
                    "walsenderwaitforwal" => pgwaittypeclient.walsenderwaitforwal += 1,
                    "walsenderwritedata" => pgwaittypeclient.walsenderwritedata += 1,
                    &_ => pgwaittypeclient.other += 1,
                };
            });
        pgwaittypeclient
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeExtension {
    pub extension: usize,
    pub other: usize,
}
impl PgWaitTypeExtension {
    pub async fn new() -> Self {
        PgWaitTypeExtension::default()
    }
    pub async fn process_pg_stat_activity(
        pg_stat_activity: Vec<PgStatActivity>,
    ) -> PgWaitTypeExtension {
        let mut pgwaittypeextension = PgWaitTypeExtension::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("extension".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "extension" => pgwaittypeextension.extension += 1,
                    &_ => pgwaittypeextension.other += 1,
                };
            });
        pgwaittypeextension
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeIO {
    pub basebackupread: usize,
    pub basebackupsync: usize,
    pub basebackupwrite: usize,
    pub buffileread: usize,
    pub buffiletruncate: usize,
    pub buffilewrite: usize,
    pub controlfileread: usize,
    pub controlfilesync: usize,
    pub controlfilesyncupdate: usize,
    pub controlfilewrite: usize,
    pub controlfilewriteupdate: usize,
    pub copyfileread: usize,
    pub copyfilewrite: usize,
    pub dsmallocate: usize,
    pub dsmfillzerowrite: usize,
    pub datafileextend: usize,
    pub datafileflush: usize,
    pub datafileimmediatesync: usize,
    pub datafileprefetch: usize,
    pub datafileread: usize,
    pub datafilesync: usize,
    pub datafiletruncate: usize,
    pub datafilewrite: usize,
    pub lockfileaddtodatadirread: usize,
    pub lockfileaddtodatadirsync: usize,
    pub lockfileaddtodatadirwrite: usize,
    pub lockfilecreateread: usize,
    pub lockfilecreatesync: usize,
    pub lockfilecreatewrite: usize,
    pub lockfilerecheckdatadirread: usize,
    pub logicalrewritecheckpointsync: usize,
    pub logicalrewritemappingsync: usize,
    pub logicalrewritemappingwrite: usize,
    pub logicalrewritesync: usize,
    pub logicalrewritetruncate: usize,
    pub logicalrewritewrite: usize,
    pub relationmapread: usize,
    pub relationmapreplace: usize,
    pub relationmapwrite: usize,
    pub reorderbufferread: usize,
    pub reorderbufferwrite: usize,
    pub reorderlogicalmappingread: usize,
    pub replicationslotread: usize,
    pub replicationslotrestoresync: usize,
    pub replicationslotsync: usize,
    pub replicationslotwrite: usize,
    pub slruflushsync: usize,
    pub slruread: usize,
    pub slrusync: usize,
    pub slruwrite: usize,
    pub snapbuildread: usize,
    pub snapbuildsync: usize,
    pub snapbuildwrite: usize,
    pub timelinehistoryfilesync: usize,
    pub timelinehistoryfilewrite: usize,
    pub timelinehistoryread: usize,
    pub timelinehistorysync: usize,
    pub timelinehistorywrite: usize,
    pub twophasefileread: usize,
    pub twophasefilesync: usize,
    pub twophasefilewrite: usize,
    pub versionfilesync: usize,
    pub versionfilewrite: usize,
    pub walbootstrapsync: usize,
    pub walbootstrapwrite: usize,
    pub walcopyread: usize,
    pub walcopysync: usize,
    pub walcopywrite: usize,
    pub walinitsync: usize,
    pub walinitwrite: usize,
    pub walread: usize,
    pub walsendertimelinehistoryread: usize,
    pub walsync: usize,
    pub walsyncmethodassign: usize,
    pub walwrite: usize,
    pub other: usize,
}
impl PgWaitTypeIO {
    pub async fn new() -> Self {
        PgWaitTypeIO::default()
    }
    pub async fn process_pg_stat_activity(pg_stat_activity: Vec<PgStatActivity>) -> PgWaitTypeIO {
        let mut pgwaittypeio = PgWaitTypeIO::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("io".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "basebackupread" => pgwaittypeio.basebackupread += 1,
                    "basebackupsync" => pgwaittypeio.basebackupsync += 1,
                    "basebackupwrite" => pgwaittypeio.basebackupwrite += 1,
                    "buffileread" => pgwaittypeio.buffileread += 1,
                    "buffiletruncate" => pgwaittypeio.buffiletruncate += 1,
                    "buffilewrite" => pgwaittypeio.buffilewrite += 1,
                    "controlfileread" => pgwaittypeio.controlfileread += 1,
                    "controlfilesync" => pgwaittypeio.controlfilesync += 1,
                    "controlfilesyncupdate" => pgwaittypeio.controlfilesyncupdate += 1,
                    "controlfilewrite" => pgwaittypeio.controlfilewrite += 1,
                    "controlfilewriteupdate" => pgwaittypeio.controlfilewriteupdate += 1,
                    "copyfileread" => pgwaittypeio.copyfileread += 1,
                    "copyfilewrite" => pgwaittypeio.copyfilewrite += 1,
                    "dsmallocate" => pgwaittypeio.dsmallocate += 1,
                    "dsmfillzerowrite" => pgwaittypeio.dsmfillzerowrite += 1,
                    "datafileextend" => pgwaittypeio.datafileextend += 1,
                    "datafileflush" => pgwaittypeio.datafileflush += 1,
                    "datafileimmediatesync" => pgwaittypeio.datafileimmediatesync += 1,
                    "datafileprefetch" => pgwaittypeio.datafileprefetch += 1,
                    "datafileread" => pgwaittypeio.datafileread += 1,
                    "datafilesync" => pgwaittypeio.datafilesync += 1,
                    "datafiletruncate" => pgwaittypeio.datafiletruncate += 1,
                    "datafilewrite" => pgwaittypeio.datafilewrite += 1,
                    "lockfileaddtodatadirread" => pgwaittypeio.lockfileaddtodatadirread += 1,
                    "lockfileaddtodatadirsync" => pgwaittypeio.lockfileaddtodatadirsync += 1,
                    "lockfileaddtodatadirwrite" => pgwaittypeio.lockfileaddtodatadirwrite += 1,
                    "lockfilecreateread" => pgwaittypeio.lockfilecreateread += 1,
                    "lockfilecreatesync" => pgwaittypeio.lockfilecreatesync += 1,
                    "lockfilecreatewrite" => pgwaittypeio.lockfilecreatewrite += 1,
                    "lockfilerecheckdatadirread" => pgwaittypeio.lockfilerecheckdatadirread += 1,
                    "logicalrewritecheckpointsync" => {
                        pgwaittypeio.logicalrewritecheckpointsync += 1
                    }
                    "logicalrewritemappingsync" => pgwaittypeio.logicalrewritemappingsync += 1,
                    "logicalrewritemappingwrite" => pgwaittypeio.logicalrewritemappingwrite += 1,
                    "logicalrewritesync" => pgwaittypeio.logicalrewritesync += 1,
                    "logicalrewritetruncate" => pgwaittypeio.logicalrewritetruncate += 1,
                    "logicalrewritewrite" => pgwaittypeio.logicalrewritewrite += 1,
                    "relationmapread" => pgwaittypeio.relationmapread += 1,
                    "relationmapreplace" => pgwaittypeio.relationmapreplace += 1,
                    "relationmapwrite" => pgwaittypeio.relationmapwrite += 1,
                    "reorderbufferread" => pgwaittypeio.reorderbufferread += 1,
                    "reorderbufferwrite" => pgwaittypeio.reorderbufferwrite += 1,
                    "reorderlogicalmappingread" => pgwaittypeio.reorderlogicalmappingread += 1,
                    "replicationslotread" => pgwaittypeio.replicationslotread += 1,
                    "replicationslotrestoresync" => pgwaittypeio.replicationslotrestoresync += 1,
                    "replicationslotsync" => pgwaittypeio.replicationslotsync += 1,
                    "replicationslotwrite" => pgwaittypeio.replicationslotwrite += 1,
                    "slruflushsync" => pgwaittypeio.slruflushsync += 1,
                    "slruread" => pgwaittypeio.slruread += 1,
                    "slrusync" => pgwaittypeio.slrusync += 1,
                    "slruwrite" => pgwaittypeio.slruwrite += 1,
                    "snapbuildread" => pgwaittypeio.snapbuildread += 1,
                    "snapbuildsync" => pgwaittypeio.snapbuildsync += 1,
                    "snapbuildwrite" => pgwaittypeio.snapbuildwrite += 1,
                    "timelinehistoryfilesync" => pgwaittypeio.timelinehistoryfilesync += 1,
                    "timelinehistoryfilewrite" => pgwaittypeio.timelinehistoryfilewrite += 1,
                    "timelinehistoryread" => pgwaittypeio.timelinehistoryread += 1,
                    "timelinehistorysync" => pgwaittypeio.timelinehistorysync += 1,
                    "timelinehistorywrite" => pgwaittypeio.timelinehistorywrite += 1,
                    "twophasefileread" => pgwaittypeio.twophasefileread += 1,
                    "twophasefilesync" => pgwaittypeio.twophasefilesync += 1,
                    "twophasefilewrite" => pgwaittypeio.twophasefilewrite += 1,
                    "versionfilesync" => pgwaittypeio.versionfilesync += 1,
                    "versionfilewrite" => pgwaittypeio.versionfilewrite += 1,
                    "walbootstrapsync" => pgwaittypeio.walbootstrapsync += 1,
                    "walbootstrapwrite" => pgwaittypeio.walbootstrapwrite += 1,
                    "walcopyread" => pgwaittypeio.walcopyread += 1,
                    "walcopysync" => pgwaittypeio.walcopysync += 1,
                    "walcopywrite" => pgwaittypeio.walcopywrite += 1,
                    "walinitsync" => pgwaittypeio.walinitsync += 1,
                    "walinitwrite" => pgwaittypeio.walinitwrite += 1,
                    "walread" => pgwaittypeio.walread += 1,
                    "walsendertimelinehistoryread" => {
                        pgwaittypeio.walsendertimelinehistoryread += 1
                    }
                    "walsync" => pgwaittypeio.walsync += 1,
                    "walsyncmethodassign" => pgwaittypeio.walsyncmethodassign += 1,
                    "walwrite" => pgwaittypeio.walwrite += 1,
                    &_ => pgwaittypeio.other += 1,
                };
            });
        pgwaittypeio
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeIPC {
    pub appendready: usize,
    pub archivecleanupcommand: usize,
    pub archivecommand: usize,
    pub backendtermination: usize,
    pub backupwaitwalarchive: usize,
    pub bgworkershutdown: usize,
    pub bgworkerstartup: usize,
    pub btreepage: usize,
    pub bufferio: usize,
    pub checkpointdone: usize,
    pub checkpointstart: usize,
    pub executegather: usize,
    pub hashbatchallocate: usize,
    pub hashbatchelect: usize,
    pub hashbatchload: usize,
    pub hashbuildallocate: usize,
    pub hashbuildelect: usize,
    pub hashbuildhashinner: usize,
    pub hashbuildhashouter: usize,
    pub hashgrowbatchesdecide: usize,
    pub hashgrowbatcheselect: usize,
    pub hashgrowbatchesfinish: usize,
    pub hashgrowbatchesreallocate: usize,
    pub hashgrowbatchesrepartition: usize,
    pub hashgrowbucketselect: usize,
    pub hashgrowbucketsreallocate: usize,
    pub hashgrowbucketsreinsert: usize,
    pub logicalapplysenddata: usize,
    pub logicalparallelapplystatechange: usize,
    pub logicalsyncdata: usize,
    pub logicalsyncstatechange: usize,
    pub messagequeueinternal: usize,
    pub messagequeueputmessage: usize,
    pub messagequeuereceive: usize,
    pub messagequeuesend: usize,
    pub parallelbitmapscan: usize,
    pub parallelcreateindexscan: usize,
    pub parallelfinish: usize,
    pub procarraygroupupdate: usize,
    pub procsignalbarrier: usize,
    pub promote: usize,
    pub recoveryconflictsnapshot: usize,
    pub recoveryconflicttablespace: usize,
    pub recoveryendcommand: usize,
    pub recoverypause: usize,
    pub replicationorigindrop: usize,
    pub replicationslotdrop: usize,
    pub restorecommand: usize,
    pub safesnapshot: usize,
    pub syncrep: usize,
    pub walreceiverexit: usize,
    pub walreceiverwaitstart: usize,
    pub xactgroupupdate: usize,
    pub other: usize,
}
impl PgWaitTypeIPC {
    pub async fn new() -> Self {
        PgWaitTypeIPC::default()
    }
    pub async fn process_pg_stat_activity(pg_stat_activity: Vec<PgStatActivity>) -> PgWaitTypeIPC {
        let mut pgwaittypeipc = PgWaitTypeIPC::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("ipc".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "appendready" => pgwaittypeipc.appendready += 1,
                    "archivecleanupcommand" => pgwaittypeipc.archivecleanupcommand += 1,
                    "archivecommand" => pgwaittypeipc.archivecommand += 1,
                    "backendtermination" => pgwaittypeipc.backendtermination += 1,
                    "backupwaitwalarchive" => pgwaittypeipc.backupwaitwalarchive += 1,
                    "bgworkershutdown" => pgwaittypeipc.bgworkershutdown += 1,
                    "bgworkerstartup" => pgwaittypeipc.bgworkerstartup += 1,
                    "btreepage" => pgwaittypeipc.btreepage += 1,
                    "bufferio" => pgwaittypeipc.bufferio += 1,
                    "checkpointdone" => pgwaittypeipc.checkpointdone += 1,
                    "checkpointstart" => pgwaittypeipc.checkpointstart += 1,
                    "executegather" => pgwaittypeipc.executegather += 1,
                    "hashbatchallocate" => pgwaittypeipc.hashbatchallocate += 1,
                    "hashbatchelect" => pgwaittypeipc.hashbatchelect += 1,
                    "hashbatchload" => pgwaittypeipc.hashbatchload += 1,
                    "hashbuildallocate" => pgwaittypeipc.hashbuildallocate += 1,
                    "hashbuildelect" => pgwaittypeipc.hashbuildelect += 1,
                    "hashbuildhashinner" => pgwaittypeipc.hashbuildhashinner += 1,
                    "hashbuildhashouter" => pgwaittypeipc.hashbuildhashouter += 1,
                    "hashgrowbatchesdecide" => pgwaittypeipc.hashgrowbatchesdecide += 1,
                    "hashgrowbatcheselect" => pgwaittypeipc.hashgrowbatcheselect += 1,
                    "hashgrowbatchesfinish" => pgwaittypeipc.hashgrowbatchesfinish += 1,
                    "hashgrowbatchesreallocate" => pgwaittypeipc.hashgrowbatchesreallocate += 1,
                    "hashgrowbatchesrepartition" => pgwaittypeipc.hashgrowbatchesrepartition += 1,
                    "hashgrowbucketselect" => pgwaittypeipc.hashgrowbucketselect += 1,
                    "hashgrowbucketsreallocate" => pgwaittypeipc.hashgrowbucketsreallocate += 1,
                    "hashgrowbucketsreinsert" => pgwaittypeipc.hashgrowbucketsreinsert += 1,
                    "logicalapplysenddata" => pgwaittypeipc.logicalapplysenddata += 1,
                    "logicalparallelapplystatechange" => {
                        pgwaittypeipc.logicalparallelapplystatechange += 1
                    }
                    "logicalsyncdata" => pgwaittypeipc.logicalsyncdata += 1,
                    "logicalsyncstatechange" => pgwaittypeipc.logicalsyncstatechange += 1,
                    "messagequeueinternal" => pgwaittypeipc.messagequeueinternal += 1,
                    "messagequeueputmessage" => pgwaittypeipc.messagequeueputmessage += 1,
                    "messagequeuereceive" => pgwaittypeipc.messagequeuereceive += 1,
                    "messagequeuesend" => pgwaittypeipc.messagequeuesend += 1,
                    "parallelbitmapscan" => pgwaittypeipc.parallelbitmapscan += 1,
                    "parallelcreateindexscan" => pgwaittypeipc.parallelcreateindexscan += 1,
                    "parallelfinish" => pgwaittypeipc.parallelfinish += 1,
                    "procarraygroupupdate" => pgwaittypeipc.procarraygroupupdate += 1,
                    "procsignalbarrier" => pgwaittypeipc.procsignalbarrier += 1,
                    "promote" => pgwaittypeipc.promote += 1,
                    "recoveryconflictsnapshot" => pgwaittypeipc.recoveryconflictsnapshot += 1,
                    "recoveryconflicttablespace" => pgwaittypeipc.recoveryconflicttablespace += 1,
                    "recoveryendcommand" => pgwaittypeipc.recoveryendcommand += 1,
                    "recoverypause" => pgwaittypeipc.recoverypause += 1,
                    "replicationorigindrop" => pgwaittypeipc.replicationorigindrop += 1,
                    "replicationslotdrop" => pgwaittypeipc.replicationslotdrop += 1,
                    "restorecommand" => pgwaittypeipc.restorecommand += 1,
                    "safesnapshot" => pgwaittypeipc.safesnapshot += 1,
                    "syncrep" => pgwaittypeipc.syncrep += 1,
                    "walreceiverexit" => pgwaittypeipc.walreceiverexit += 1,
                    "walreceiverwaitstart" => pgwaittypeipc.walreceiverwaitstart += 1,
                    "xactgroupupdate" => pgwaittypeipc.xactgroupupdate += 1,
                    &_ => pgwaittypeipc.other += 1,
                };
            });
        pgwaittypeipc
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeLock {
    pub advisory: usize,
    pub applytransaction: usize,
    pub extend: usize,
    pub frozenid: usize,
    pub object: usize,
    pub page: usize,
    pub relation: usize,
    pub spectoken: usize,
    pub transactionid: usize,
    pub tuple: usize,
    pub userlock: usize,
    pub virtualxid: usize,
    pub other: usize,
}
impl PgWaitTypeLock {
    pub async fn new() -> Self {
        PgWaitTypeLock::default()
    }
    pub async fn process_pg_stat_activity(pg_stat_activity: Vec<PgStatActivity>) -> PgWaitTypeLock {
        let mut pgwaittypelock = PgWaitTypeLock::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("lock".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "advisory" => pgwaittypelock.advisory += 1,
                    "applytransaction" => pgwaittypelock.applytransaction += 1,
                    "extend" => pgwaittypelock.extend += 1,
                    "frozenid" => pgwaittypelock.frozenid += 1,
                    "object" => pgwaittypelock.object += 1,
                    "page" => pgwaittypelock.page += 1,
                    "relation" => pgwaittypelock.relation += 1,
                    "spectoken" => pgwaittypelock.spectoken += 1,
                    "transactionid" => pgwaittypelock.transactionid += 1,
                    "tuple" => pgwaittypelock.tuple += 1,
                    "userlock" => pgwaittypelock.userlock += 1,
                    "virtualxid" => pgwaittypelock.virtualxid += 1,
                    &_ => pgwaittypelock.other += 1,
                };
            });
        pgwaittypelock
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeLWLock {
    pub addinsheminit: usize,
    pub autofile: usize,
    pub autovacuum: usize,
    pub autovacuumschedule: usize,
    pub backgroundworker: usize,
    pub btreevacuum: usize,
    pub buffercontent: usize,
    pub buffermapping: usize,
    pub checkpointercomm: usize,
    pub committs: usize,
    pub committsbuffer: usize,
    pub committsslru: usize,
    pub controlfile: usize,
    pub dynamicsharedmemorycontrol: usize,
    pub lockfastpath: usize,
    pub lockmanager: usize,
    pub logicalreplauncherdsa: usize,
    pub logicalreplauncherhash: usize,
    pub logicalrepworker: usize,
    pub multixactgen: usize,
    pub multixactmemberbuffer: usize,
    pub multixactmemberslru: usize,
    pub multixactoffsetbuffer: usize,
    pub multixactoffsetslru: usize,
    pub multixacttruncation: usize,
    pub notifybuffer: usize,
    pub notifyqueue: usize,
    pub notifyqueuetail: usize,
    pub notifyslru: usize,
    pub oidgen: usize,
    pub oldsnapshottimemap: usize,
    pub parallelappend: usize,
    pub parallelhashjoin: usize,
    pub parallelquerydsa: usize,
    pub persessiondsa: usize,
    pub persessionrecordtype: usize,
    pub persessionrecordtypmod: usize,
    pub perxactpredicatelist: usize,
    pub pgstatsdata: usize,
    pub pgstatsdsa: usize,
    pub pgstatshash: usize,
    pub predicatelockmanager: usize,
    pub procarray: usize,
    pub relationmapping: usize,
    pub relcacheinit: usize,
    pub replicationorigin: usize,
    pub replicationoriginstate: usize,
    pub replicationslotallocation: usize,
    pub replicationslotcontrol: usize,
    pub replicationslotio: usize,
    pub serialbuffer: usize,
    pub serializablefinishedlist: usize,
    pub serializablepredicatelist: usize,
    pub serializablexacthash: usize,
    pub serialslru: usize,
    pub sharedtidbitmap: usize,
    pub sharedtuplestore: usize,
    pub shmemindex: usize,
    pub sinvalread: usize,
    pub sinvalwrite: usize,
    pub subtransbuffer: usize,
    pub subtransslru: usize,
    pub syncrep: usize,
    pub syncscan: usize,
    pub tablespacecreate: usize,
    pub twophasestate: usize,
    pub walbufmapping: usize,
    pub walinsert: usize,
    pub walwrite: usize,
    pub wraplimitsvacuum: usize,
    pub xactbuffer: usize,
    pub xactslru: usize,
    pub xacttruncation: usize,
    pub xidgen: usize,
    pub other: usize,
}
impl PgWaitTypeLWLock {
    pub async fn new() -> Self {
        PgWaitTypeLWLock::default()
    }
    pub async fn process_pg_stat_activity(
        pg_stat_activity: Vec<PgStatActivity>,
    ) -> PgWaitTypeLWLock {
        let mut pgwaittypelwlock = PgWaitTypeLWLock::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("lwlock".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "addinshmeminit" => pgwaittypelwlock.addinsheminit += 1,
                    "autofile" => pgwaittypelwlock.autofile += 1,
                    "autovacuum" => pgwaittypelwlock.autovacuum += 1,
                    "autovacuumschedule" => pgwaittypelwlock.autovacuumschedule += 1,
                    "backgroundworker" => pgwaittypelwlock.backgroundworker += 1,
                    "btreevacuum" => pgwaittypelwlock.btreevacuum += 1,
                    "buffercontent" => pgwaittypelwlock.buffercontent += 1,
                    "buffermapping" => pgwaittypelwlock.buffermapping += 1,
                    "checkpointercomm" => pgwaittypelwlock.checkpointercomm += 1,
                    "committs" => pgwaittypelwlock.committs += 1,
                    "committsbuffer" => pgwaittypelwlock.committsbuffer += 1,
                    "committsslru" => pgwaittypelwlock.committsslru += 1,
                    "controlfile" => pgwaittypelwlock.controlfile += 1,
                    "dynamicsharedmemorycontrol" => {
                        pgwaittypelwlock.dynamicsharedmemorycontrol += 1
                    }
                    "lockfastpath" => pgwaittypelwlock.lockfastpath += 1,
                    "lockmanager" => pgwaittypelwlock.lockmanager += 1,
                    "logicalreplauncerdsa" => pgwaittypelwlock.logicalreplauncherdsa += 1,
                    "logicalreplauncerhash" => pgwaittypelwlock.logicalreplauncherhash += 1,
                    "logicalrepworker" => pgwaittypelwlock.logicalrepworker += 1,
                    "multixactgen" => pgwaittypelwlock.multixactgen += 1,
                    "multixactmemberbuffer" => pgwaittypelwlock.multixactmemberbuffer += 1,
                    "multixactmemberslru" => pgwaittypelwlock.multixactmemberslru += 1,
                    "multixactoffsetbuffer" => pgwaittypelwlock.multixactoffsetbuffer += 1,
                    "multixactoffsetslru" => pgwaittypelwlock.multixactoffsetslru += 1,
                    "multixacttruncation" => pgwaittypelwlock.multixacttruncation += 1,
                    "notifybuffer" => pgwaittypelwlock.notifybuffer += 1,
                    "notifyqueue" => pgwaittypelwlock.notifyqueue += 1,
                    "notifyqueuetail" => pgwaittypelwlock.notifyqueuetail += 1,
                    "notifyslru" => pgwaittypelwlock.notifyslru += 1,
                    "oidgen" => pgwaittypelwlock.oidgen += 1,
                    "oldsnapshottimemap" => pgwaittypelwlock.oldsnapshottimemap += 1,
                    "parallelappend" => pgwaittypelwlock.parallelappend += 1,
                    "parallelhashjoin" => pgwaittypelwlock.parallelhashjoin += 1,
                    "parallelquerydsa" => pgwaittypelwlock.parallelquerydsa += 1,
                    "persessiondsa" => pgwaittypelwlock.persessiondsa += 1,
                    "persessionrecordtype" => pgwaittypelwlock.persessionrecordtype += 1,
                    "persessionrecordtypmod" => pgwaittypelwlock.persessionrecordtypmod += 1,
                    "perxactpredicatelist" => pgwaittypelwlock.perxactpredicatelist += 1,
                    "pgstatsdata" => pgwaittypelwlock.pgstatsdata += 1,
                    "pgstatsdsa" => pgwaittypelwlock.pgstatsdsa += 1,
                    "pgstatshash" => pgwaittypelwlock.pgstatshash += 1,
                    "predicatelockmanager" => pgwaittypelwlock.predicatelockmanager += 1,
                    "procarray" => pgwaittypelwlock.procarray += 1,
                    "relationmapping" => pgwaittypelwlock.relationmapping += 1,
                    "relcacheinit" => pgwaittypelwlock.relcacheinit += 1,
                    "replicationorigin" => pgwaittypelwlock.replicationorigin += 1,
                    "replicationoriginstate" => pgwaittypelwlock.replicationoriginstate += 1,
                    "replicationslotallocation" => pgwaittypelwlock.replicationslotallocation += 1,
                    "replicationslotcontrol" => pgwaittypelwlock.replicationslotcontrol += 1,
                    "replicationslotio" => pgwaittypelwlock.replicationslotio += 1,
                    "serialbuffer" => pgwaittypelwlock.serialbuffer += 1,
                    "serializablefinishedlist" => pgwaittypelwlock.serializablefinishedlist += 1,
                    "serializablepredicatelist" => pgwaittypelwlock.serializablepredicatelist += 1,
                    "serializablexacthash" => pgwaittypelwlock.serializablexacthash += 1,
                    "serialslru" => pgwaittypelwlock.serialslru += 1,
                    "sharedtidbitmap" => pgwaittypelwlock.sharedtidbitmap += 1,
                    "sharedtuplestore" => pgwaittypelwlock.sharedtuplestore += 1,
                    "shmemindex" => pgwaittypelwlock.shmemindex += 1,
                    "sinvalread" => pgwaittypelwlock.sinvalread += 1,
                    "sinvalwrite" => pgwaittypelwlock.sinvalwrite += 1,
                    "subtransbuffer" => pgwaittypelwlock.subtransbuffer += 1,
                    "subtransslru" => pgwaittypelwlock.subtransslru += 1,
                    "syncrep" => pgwaittypelwlock.syncrep += 1,
                    "syncscan" => pgwaittypelwlock.syncscan += 1,
                    "tablespacecreate" => pgwaittypelwlock.tablespacecreate += 1,
                    "twophasestate" => pgwaittypelwlock.twophasestate += 1,
                    "walbufmapping" => pgwaittypelwlock.walbufmapping += 1,
                    "walinsert" => pgwaittypelwlock.walinsert += 1,
                    "walwrite" => pgwaittypelwlock.walwrite += 1,
                    "wraplimitsvacuum" => pgwaittypelwlock.wraplimitsvacuum += 1,
                    "xactbuffer" => pgwaittypelwlock.xactbuffer += 1,
                    "xactslru" => pgwaittypelwlock.xactslru += 1,
                    "xacttruncation" => pgwaittypelwlock.xacttruncation += 1,
                    "xidgen" => pgwaittypelwlock.xidgen += 1,
                    &_ => pgwaittypelwlock.other += 1,
                };
            });
        pgwaittypelwlock
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeTimeout {
    pub basebackupthrottle: usize,
    pub checkpointerwritedelay: usize,
    pub pgsleep: usize,
    pub recoveryapplydelay: usize,
    pub recoveryretrieveretryinterval: usize,
    pub registersyncrequest: usize,
    pub spindelay: usize,
    pub vacuumdelay: usize,
    pub vacuumtruncate: usize,
    pub other: usize,
}
impl PgWaitTypeTimeout {
    pub async fn new() -> Self {
        PgWaitTypeTimeout::default()
    }
    pub async fn process_pg_stat_activity(
        pg_stat_activity: Vec<PgStatActivity>,
    ) -> PgWaitTypeTimeout {
        let mut pgwaittypetimeout = PgWaitTypeTimeout::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| {
                r.wait_event_type == Some("timeout".to_string())
                    && r.state.as_ref().unwrap_or(&"".to_string()) == "active"
            })
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "basebackupthrottle" => pgwaittypetimeout.basebackupthrottle += 1,
                    "checkpoinwritedelay" => pgwaittypetimeout.checkpointerwritedelay += 1,
                    "pgsleep" => pgwaittypetimeout.pgsleep += 1,
                    "recoveryapplydelay" => pgwaittypetimeout.recoveryapplydelay += 1,
                    "recoveryretrieveretryinterval" => {
                        pgwaittypetimeout.recoveryretrieveretryinterval += 1
                    }
                    "registersyncrequest" => pgwaittypetimeout.registersyncrequest += 1,
                    "spindelay" => pgwaittypetimeout.spindelay += 1,
                    "vacuumdelay" => pgwaittypetimeout.vacuumdelay += 1,
                    "vacuumtruncate" => pgwaittypetimeout.vacuumtruncate += 1,
                    &_ => pgwaittypetimeout.other += 1,
                };
            });
        pgwaittypetimeout
    }
}

#[derive(Debug)]
pub struct PgStatDatabaseSum {
    pub xact_commit_ps: f64,
    pub xact_rollback_ps: f64,
    pub blks_read_ps: f64,
    pub blks_hit_ps: f64,
    pub tup_returned_ps: f64,
    pub tup_fetched_ps: f64,
    pub tup_inserted_ps: f64,
    pub tup_updated_ps: f64,
    pub tup_deleted_ps: f64,
    pub blk_read_time_ps: f64,
    pub blk_write_time_ps: f64,
}

impl PgStatDatabaseSum {
    pub async fn process_pg_stat_database(pg_stat_database: Vec<PgStatDatabase>) {
        let pg_stat_database_timestamp = pg_stat_database.last().map(|r| r.timestamp).unwrap();
        DeltaTable::add_or_update(
            "pg_stat_database.xact_commit",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.xact_commit)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.xact_rollback",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.xact_rollback)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.blks_read",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.blks_read)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.blk_read_time",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.blk_read_time)
                .fold(0_f64, |sum, b| sum + b),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.blks_hit",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.blks_hit)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.blk_write_time",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.blk_write_time)
                .fold(0_f64, |sum, b| sum + b),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_returned",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_returned)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_fetched",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_fetched)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_inserted",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_inserted)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_updated",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_updated)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_database.tup_deleted",
            pg_stat_database_timestamp,
            pg_stat_database
                .iter()
                .map(|r| r.tup_deleted)
                .fold(0_f64, |sum, b| sum + (b as f64)),
        )
        .await;
        // only add to DATA if updated_value is true, which means that there have been two
        // additions, and thus a DELTA (difference) is calculated.
        if DELTATABLE
            .read()
            .await
            .get("pg_stat_database.xact_commit")
            .unwrap()
            .updated_value
        {
            DATA.pg_stat_database_sum.write().await.push_back((
                pg_stat_database_timestamp,
                PgStatDatabaseSum {
                    xact_commit_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.xact_commit")
                        .unwrap()
                        .per_second_value,
                    xact_rollback_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.xact_rollback")
                        .unwrap()
                        .per_second_value,
                    blks_read_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.blks_read")
                        .unwrap()
                        .per_second_value,
                    blks_hit_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.blks_hit")
                        .unwrap()
                        .per_second_value,
                    tup_returned_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_returned")
                        .unwrap()
                        .per_second_value,
                    tup_fetched_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_fetched")
                        .unwrap()
                        .per_second_value,
                    tup_inserted_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_inserted")
                        .unwrap()
                        .per_second_value,
                    tup_updated_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_updated")
                        .unwrap()
                        .per_second_value,
                    tup_deleted_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.tup_deleted")
                        .unwrap()
                        .per_second_value,
                    blk_read_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.blk_read_time")
                        .unwrap()
                        .per_second_value,
                    blk_write_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_database.blk_write_time")
                        .unwrap()
                        .per_second_value,
                },
            ));
        }
    }
}

// this pg_stat_database is consistent with postgres version 15
#[derive(Debug, FromRow, Clone)]
pub struct PgStatDatabase {
    pub timestamp: DateTime<Local>,
    pub datid: Option<Oid>,
    pub datname: Option<String>,
    pub numbackends: i32,
    pub xact_commit: i64,
    pub xact_rollback: i64,
    pub blks_read: i64,
    pub blks_hit: i64,
    pub tup_returned: i64,
    pub tup_fetched: i64,
    pub tup_inserted: i64,
    pub tup_updated: i64,
    pub tup_deleted: i64,
    pub conflicts: i64,
    pub temp_files: i64,
    pub temp_bytes: i64,
    pub deadlocks: i64,
    pub checksum_failures: Option<i64>,
    pub checksum_last_failure: Option<DateTime<Local>>,
    pub blk_read_time: f64,
    pub blk_write_time: f64,
    pub session_time: f64,
    pub active_time: f64,
    pub idle_in_transaction_time: f64,
    pub sessions: i64,
    pub sessions_abandoned: i64,
    pub sessions_fatal: i64,
    pub sessions_killed: i64,
    pub stats_reset: Option<DateTime<Local>>,
}

impl PgStatDatabase {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_stat_database = PgStatDatabase::query(pool).await;
        PgStatDatabaseSum::process_pg_stat_database(pg_stat_database).await;
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgStatDatabase> {
        let stat_database: Vec<PgStatDatabase> = query_as(
            "
            select clock_timestamp() as timestamp,
                   datid, 
                   datname, 
                   numbackends,
                   xact_commit,
                   xact_rollback,
                   blks_read, 
                   blks_hit, 
                   tup_returned,
                   tup_fetched,
                   tup_inserted,
                   tup_updated, 
                   tup_deleted, 
                   conflicts, 
                   temp_files, 
                   temp_bytes,
                   deadlocks,
                   checksum_failures, 
                   checksum_last_failure,
                   blk_read_time,
                   blk_write_time, 
                   session_time, 
                   active_time,
                   idle_in_transaction_time,
                   sessions,
                   sessions_abandoned,
                   sessions_fatal,
                   sessions_killed,
                   stats_reset
             from  pg_stat_database 
        ",
        )
        .fetch_all(pool)
        .await
        .expect("error executing query");
        //sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        //sql_rows.reverse();
        stat_database
    }
}

#[derive(Debug)]
pub struct PgStatWalSum {
    pub wal_records_ps: f64,
    pub wal_fpi_ps: f64,
    pub wal_bytes_ps: f64,
    pub wal_buffers_full_ps: f64,
    pub wal_write_ps: f64,
    pub wal_sync_ps: f64,
    pub wal_write_time_ps: f64,
    pub wal_sync_time_ps: f64,
}

impl PgStatWalSum {
    pub async fn process_pg_stat_wal(pg_stat_wal: PgStatWal) {
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_records",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_records as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_fpi",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_fpi as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_bytes",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_bytes.to_f64().unwrap(),
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_buffers_full",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_buffers_full as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_write",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_write as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_sync",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_sync as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_write_time",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_write_time,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_wal.wal_sync_time",
            pg_stat_wal.timestamp,
            pg_stat_wal.wal_sync_time,
        )
        .await;
        if DELTATABLE
            .read()
            .await
            .get("pg_stat_wal.wal_records")
            .unwrap()
            .updated_value
        {
            DATA.pg_stat_wal_sum.write().await.push_back((
                pg_stat_wal.timestamp,
                PgStatWalSum {
                    wal_records_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_records")
                        .unwrap()
                        .per_second_value,
                    wal_fpi_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_fpi")
                        .unwrap()
                        .per_second_value,
                    wal_bytes_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_bytes")
                        .unwrap()
                        .per_second_value,
                    wal_buffers_full_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_buffers_full")
                        .unwrap()
                        .per_second_value,
                    wal_write_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_write")
                        .unwrap()
                        .per_second_value,
                    wal_sync_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_sync")
                        .unwrap()
                        .per_second_value,
                    wal_write_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_write_time")
                        .unwrap()
                        .per_second_value,
                    wal_sync_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_wal.wal_sync_time")
                        .unwrap()
                        .per_second_value,
                },
            ));
        }
    }
}
// this pg_stat_wal is consistent with postgres version 15
#[derive(Debug, FromRow, Clone)]
pub struct PgStatWal {
    pub timestamp: DateTime<Local>,
    pub wal_records: i64,
    pub wal_fpi: i64,
    pub wal_bytes: BigDecimal,
    pub wal_buffers_full: i64,
    pub wal_write: i64,
    pub wal_sync: i64,
    pub wal_write_time: f64,
    pub wal_sync_time: f64,
    pub stats_reset: Option<DateTime<Local>>,
}

impl PgStatWal {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_stat_wal = PgStatWal::query(pool).await;
        PgStatWalSum::process_pg_stat_wal(pg_stat_wal).await;
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> PgStatWal {
        let stat_wal: PgStatWal = query_as(
            "
            select clock_timestamp() as timestamp,
                   wal_records, 
                   wal_fpi, 
                   wal_bytes,
                   wal_buffers_full,
                   wal_write,
                   wal_sync, 
                   wal_write_time, 
                   wal_sync_time,
                   stats_reset
             from  pg_stat_wal 
        ",
        )
        .fetch_one(pool)
        .await
        .expect("error executing query");
        //sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        //sql_rows.reverse();
        stat_wal
    }
}

#[derive(Debug)]
pub struct PgStatBgWriterSum {
    pub checkpoint_write_time_ps: f64,
    pub checkpoint_sync_time_ps: f64,
    pub buffers_checkpoint_ps: f64,
    pub buffers_clean_ps: f64,
    pub buffers_backend_ps: f64,
    pub buffers_backend_fsync_ps: f64,
    pub buffers_alloc_ps: f64,
}

impl PgStatBgWriterSum {
    pub async fn process_pg_bgwriter(pg_stat_bgwriter: PgStatBgWriter) {
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.checkpoint_write_time",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.checkpoint_write_time,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.checkpoint_sync_time",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.checkpoint_sync_time,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_checkpoint",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_checkpoint as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_clean",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_clean as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_backend",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_backend as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_backend_fsync",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_backend_fsync as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_stat_bgwriter.buffers_alloc",
            pg_stat_bgwriter.timestamp,
            pg_stat_bgwriter.buffers_alloc as f64,
        )
        .await;
        if DELTATABLE
            .read()
            .await
            .get("pg_stat_bgwriter.checkpoint_write_time")
            .unwrap()
            .updated_value
        {
            DATA.pg_stat_bgwriter_sum.write().await.push_back((
                pg_stat_bgwriter.timestamp,
                PgStatBgWriterSum {
                    checkpoint_write_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.checkpoint_write_time")
                        .unwrap()
                        .per_second_value,
                    checkpoint_sync_time_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.checkpoint_sync_time")
                        .unwrap()
                        .per_second_value,
                    buffers_checkpoint_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_checkpoint")
                        .unwrap()
                        .per_second_value,
                    buffers_clean_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_clean")
                        .unwrap()
                        .per_second_value,
                    buffers_backend_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_backend")
                        .unwrap()
                        .per_second_value,
                    buffers_backend_fsync_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_backend_fsync")
                        .unwrap()
                        .per_second_value,
                    buffers_alloc_ps: DELTATABLE
                        .read()
                        .await
                        .get("pg_stat_bgwriter.buffers_alloc")
                        .unwrap()
                        .per_second_value,
                },
            ));
        }
    }
}

// this pg_stat_bgwriter is consistent with postgres version 15
#[derive(Debug, FromRow, Clone)]
pub struct PgStatBgWriter {
    pub timestamp: DateTime<Local>,
    pub checkpoints_timed: i64,
    pub checkpoints_req: i64,
    pub checkpoint_write_time: f64,
    pub checkpoint_sync_time: f64,
    pub buffers_checkpoint: i64,
    pub buffers_clean: i64,
    pub maxwritten_clean: i64,
    pub buffers_backend: i64,
    pub buffers_backend_fsync: i64,
    pub buffers_alloc: i64,
    pub stats_reset: Option<DateTime<Local>>,
}

impl PgStatBgWriter {
    pub async fn fetch_and_add_to_data(pool: &Pool<sqlx::Postgres>) {
        let pg_stat_bgwriter = PgStatBgWriter::query(pool).await;
        PgStatBgWriterSum::process_pg_bgwriter(pg_stat_bgwriter).await;
    }
    async fn query(pool: &Pool<sqlx::Postgres>) -> PgStatBgWriter {
        let stat_bgwriter: PgStatBgWriter = query_as(
            "
            select clock_timestamp() as timestamp,
                   checkpoints_timed, 
                   checkpoints_req, 
                   checkpoint_write_time,
                   checkpoint_sync_time,
                   buffers_checkpoint,
                   buffers_clean, 
                   maxwritten_clean, 
                   buffers_backend,
                   buffers_backend_fsync,
                   buffers_alloc,
                   stats_reset
             from  pg_stat_bgwriter 
        ",
        )
        .fetch_one(pool)
        .await
        .expect("error executing query");
        //sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        //sql_rows.reverse();
        stat_bgwriter
    }
}

#[derive(Debug)]
pub struct PgDatabaseXidLimits {
    pub age_datfronzenxid: f64,
    pub age_datminmxid: f64,
}
impl PgDatabaseXidLimits {}

// this pg_database is consistent with postgres version 15
#[derive(Debug, FromRow, Clone)]
pub struct PgDatabase {
    pub timestamp: DateTime<Local>,
    pub oid: Oid,
    pub datname: String,
    pub datdba: Oid,
    pub encoding: i32,
    pub datlocprovider: String,
    pub datistemplate: bool,
    pub datallowconn: bool,
    pub datconnlimit: i32,
    pub age_datfrozenxid: i32,
    pub age_datminmxid: i32,
    pub dattablespace: Oid,
    pub datcollate: String,
    pub datctype: String,
    pub daticulocale: Option<String>,
    pub datcollversion: Option<String>,
}

impl PgDatabase {
    pub async fn query(pool: &Pool<sqlx::Postgres>) -> Vec<PgDatabase> {
        let stat_database: Vec<PgDatabase> = query_as(
            "
            select clock_timestamp() as timestamp,
                   oid, 
                   datname, 
                   datdba,
                   encoding,
                   datlocprovider::text,
                   datistemplate, 
                   datallowconn, 
                   datconnlimit,
                   age(datfrozenxid) as age_datfrozenxid,
                   mxid_age(datminmxid) as age_datminmxid,
                   dattablespace, 
                   datcollate, 
                   datctype, 
                   daticulocale, 
                   datcollversion
             from  pg_database 
        ",
        )
        .fetch_all(pool)
        .await
        .expect("error executing query");
        //sql_rows.sort_by_key(|a| a.query_time.as_ref().map_or(0_i64, |r| r.microseconds));
        //sql_rows.reverse();
        stat_database
    }
}

#[derive(Debug, Default)]
pub struct StatisticsDelta {
    pub last_timestamp: DateTime<Local>,
    pub last_value: f64,
    pub delta_value: f64,
    pub per_second_value: f64,
    pub updated_value: bool,
}

type DeltaHashTable = RwLock<HashMap<String, StatisticsDelta>>;
static DELTATABLE: Lazy<DeltaHashTable> = Lazy::new(|| RwLock::new(HashMap::new()));

pub struct DeltaTable {}

impl DeltaTable {
    pub async fn add_or_update(name: &str, last_timestamp: DateTime<Local>, last_value: f64) {
        DELTATABLE
            .write()
            .await
            .entry(name.to_string())
            .and_modify(|r| {
                // if fetched timestamp doesn't make sense alias the fetch was invalid:
                if last_timestamp == r.last_timestamp {
                    r.updated_value = false;
                } else {
                    // if the statistics are reset
                    if r.last_value > last_value {
                        r.last_timestamp = last_timestamp;
                        r.last_value = last_value;
                        r.delta_value = 0_f64;
                        r.per_second_value = 0_f64;
                        r.updated_value = false;
                    } else {
                        // this is the normal situation after the insert, where we can calculate
                        // the delta, as well as the amount per second
                        r.delta_value = last_value - r.last_value;
                        // the per secon value is caluclated by dividing it by the number of
                        // milliseconds (not seconds), and then dividing it by 1000 to make it per
                        // second.
                        r.per_second_value = r.delta_value
                            / (last_timestamp
                                .signed_duration_since(r.last_timestamp)
                                .num_milliseconds() as f64
                                / 1000_f64);
                        r.last_value = last_value;
                        r.last_timestamp = last_timestamp;
                        r.updated_value = true;
                        if r.per_second_value.is_nan() {
                            r.per_second_value = 0_f64
                        }
                    }
                };
            })
            .or_insert(StatisticsDelta {
                last_timestamp,
                last_value,
                delta_value: 0_f64,
                per_second_value: 0_f64,
                updated_value: false,
            });
    }
}

pub async fn processor_main() -> Result<()> {
    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .connect("postgres://frits.hoogland@frits.hoogland?host=/tmp/")
        .await
        .expect("Error creating connection pool");

    let mut interval = time::interval(Duration::from_secs(ARGS.interval));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        PgStatActivity::fetch_and_add_to_data(&pool).await;
        PgStatDatabase::fetch_and_add_to_data(&pool).await;
        PgStatBgWriter::fetch_and_add_to_data(&pool).await;
        PgStatWal::fetch_and_add_to_data(&pool).await;
        let pg_database = PgDatabase::query(&pool).await;
        DeltaTable::add_or_update(
            "pg_database.age_datfrozenxid",
            pg_database.first().map(|r| r.timestamp).unwrap(),
            pg_database
                .iter()
                .map(|r| r.age_datfrozenxid)
                .max()
                .unwrap() as f64,
        )
        .await;
        DeltaTable::add_or_update(
            "pg_database.age_datminmxid",
            pg_database.first().map(|r| r.timestamp).unwrap(),
            pg_database.iter().map(|r| r.age_datminmxid).max().unwrap() as f64,
        )
        .await;
        //println!(
        //    "{:#?}",
        //    pg_database.iter().map(|r| r.age_datfrozenxid).max()
        //);
        //println!("{:#?}", DELTATABLE.read().await);
        //println!("{:?}", DATA.wait_event_activity.read().await);
    }
}
