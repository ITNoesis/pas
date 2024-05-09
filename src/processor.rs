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
                   wait_event_type,
                   wait_event,
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
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Activity")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let buffer_pin = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "BufferPin")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let client = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Client")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let extension = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Extension")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let io = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "IO")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let ipc = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "IPC")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let lock = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Lock")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let lwlock = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "LWLock")
            .filter(|r| r.state.as_ref().unwrap_or(&"".to_string()) == "active")
            .count();
        let timeout = pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type.as_ref().unwrap_or(&"".to_string()) == "Timeout")
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
    archivermain: usize,
    autovacuummain: usize,
    bgwriterhibernate: usize,
    bgwritermain: usize,
    checkpointermain: usize,
    logicalapplymain: usize,
    logicallaunchermain: usize,
    logicalparallelapplymain: usize,
    recoverywalstream: usize,
    sysloggermain: usize,
    walreceivermain: usize,
    walsendermain: usize,
    walwritermain: usize,
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
            .filter(|r| r.wait_event_type == Some("Activity".to_string()))
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "ArchiverMain" => pgwaittypeactivity.archivermain += 1,
                    "AutoVacuumMain" => pgwaittypeactivity.autovacuummain += 1,
                    "BgWriterHibernate" => pgwaittypeactivity.bgwriterhibernate += 1,
                    "BgWriterMain" => pgwaittypeactivity.bgwritermain += 1,
                    "CheckpointerMain" => pgwaittypeactivity.checkpointermain += 1,
                    "LogicalApplyMain" => pgwaittypeactivity.logicalapplymain += 1,
                    "LogicalLauncherMain" => pgwaittypeactivity.logicallaunchermain += 1,
                    "LogicalParallelApplyMain" => pgwaittypeactivity.logicalparallelapplymain += 1,
                    "RecoveryWalStream" => pgwaittypeactivity.recoverywalstream += 1,
                    "SysLoggerMain" => pgwaittypeactivity.sysloggermain += 1,
                    "WalReceiverMain" => pgwaittypeactivity.walreceivermain += 1,
                    "WalSenderMain" => pgwaittypeactivity.walsendermain += 1,
                    "WalWriterMain" => pgwaittypeactivity.walwritermain += 1,
                    &_ => {}
                };
            });
        pgwaittypeactivity
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeBufferPin {
    bufferpin: usize,
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
            .filter(|r| r.wait_event_type == Some("BufferPin".to_string()))
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "BufferPin" => pgwaittypebufferpin.bufferpin += 1,
                    &_ => {}
                };
            });
        pgwaittypebufferpin
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeClient {
    clientread: usize,
    clientwrite: usize,
    gssopenserver: usize,
    libpqwalreceiverconnect: usize,
    libpqwalreceiverreceive: usize,
    sslopenserver: usize,
    walsenderwaitforwal: usize,
    walsenderwritedata: usize,
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
            .filter(|r| r.wait_event_type == Some("Client".to_string()))
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "ClientRead" => pgwaittypeclient.clientread += 1,
                    "ClientWrite" => pgwaittypeclient.clientwrite += 1,
                    "GSSOpenServer" => pgwaittypeclient.gssopenserver += 1,
                    "LibPQWalReceiverConnect" => pgwaittypeclient.libpqwalreceiverconnect += 1,
                    "LibPQWalReceiverReceive" => pgwaittypeclient.libpqwalreceiverreceive += 1,
                    "SSLOpenServer" => pgwaittypeclient.sslopenserver += 1,
                    "WalSenderWaitForWal" => pgwaittypeclient.walsenderwaitforwal += 1,
                    "WalSenderWriteData" => pgwaittypeclient.walsenderwritedata += 1,
                    &_ => {}
                };
            });
        pgwaittypeclient
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeExtension {
    extension: usize,
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
            .filter(|r| r.wait_event_type == Some("Extension".to_string()))
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "Extension" => pgwaittypeextension.extension += 1,
                    &_ => {}
                };
            });
        pgwaittypeextension
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeIO {
    basebackupread: usize,
    basebackupsync: usize,
    basebackupwrite: usize,
    buffileread: usize,
    buffiletruncate: usize,
    buffilewrite: usize,
    controlfileread: usize,
    controlfilesync: usize,
    controlfilesyncupdate: usize,
    controlfilewrite: usize,
    controlfilewriteupdate: usize,
    copyfileread: usize,
    copyfilewrite: usize,
    dsmallocate: usize,
    dsmfillzerowrite: usize,
    datafileextend: usize,
    datafileflush: usize,
    datafileimmediatesync: usize,
    datafileprefetch: usize,
    datafileread: usize,
    datafilesync: usize,
    datafiletruncate: usize,
    datafilewrite: usize,
    lockfileaddtodatadirread: usize,
    lockfileaddtodatadirsync: usize,
    lockfileaddtodatadirwrite: usize,
    lockfilecreateread: usize,
    lockfilecreatesync: usize,
    lockfilecreatewrite: usize,
    lockfilerecheckdatadirread: usize,
    logicalrewritecheckpointsync: usize,
    logicalrewritemappingsync: usize,
    logicalrewritemappingwrite: usize,
    logicalrewritesync: usize,
    logicalrewritetruncate: usize,
    logicalrewritewrite: usize,
    relationmapread: usize,
    relationmapreplace: usize,
    relationmapwrite: usize,
    reorderbufferread: usize,
    reorderbufferwrite: usize,
    reorderlogicalmappingread: usize,
    replicationslotread: usize,
    replicationslotrestoresync: usize,
    replicationslotsync: usize,
    replicationslotwrite: usize,
    slruflushsync: usize,
    slruread: usize,
    slrusync: usize,
    slruwrite: usize,
    snapbuildread: usize,
    snapbuildsync: usize,
    snapbuildwrite: usize,
    timelinehistoryfilesync: usize,
    timelinehistoryfilewrite: usize,
    timelinehistoryread: usize,
    timelinehistorysync: usize,
    timelinehistorywrite: usize,
    twophasefileread: usize,
    twophasefilesync: usize,
    twophasefilewrite: usize,
    versionfilesync: usize,
    versionfilewrite: usize,
    walbootstrapsync: usize,
    walbootstrapwrite: usize,
    walcopyread: usize,
    walcopysync: usize,
    walcopywrite: usize,
    walinitsync: usize,
    walinitwrite: usize,
    walread: usize,
    walsendertimelinehistoryread: usize,
    walsync: usize,
    walsyncmethodassign: usize,
    walwrite: usize,
}
impl PgWaitTypeIO {
    pub async fn new() -> Self {
        PgWaitTypeIO::default()
    }
    pub async fn process_pg_stat_activity(pg_stat_activity: Vec<PgStatActivity>) -> PgWaitTypeIO {
        let mut pgwaittypeio = PgWaitTypeIO::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type == Some("IO".to_string()))
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "BaseBackupRead" => pgwaittypeio.basebackupread += 1,
                    "BaseBackupSync" => pgwaittypeio.basebackupsync += 1,
                    "BaseBackupWrite" => pgwaittypeio.basebackupwrite += 1,
                    "BufFileRead" => pgwaittypeio.buffileread += 1,
                    "BufFileTruncate" => pgwaittypeio.buffiletruncate += 1,
                    "BufFileWrite" => pgwaittypeio.buffilewrite += 1,
                    "ControlFileRead" => pgwaittypeio.controlfileread += 1,
                    "ControlFileSync" => pgwaittypeio.controlfilesync += 1,
                    "ControlFileSyncUpdate" => pgwaittypeio.controlfilesyncupdate += 1,
                    "ControlFileWrite" => pgwaittypeio.controlfilewrite += 1,
                    "ControlFileWriteUpdate" => pgwaittypeio.controlfilewriteupdate += 1,
                    "CopyFileRead" => pgwaittypeio.copyfileread += 1,
                    "CopyFileWrite" => pgwaittypeio.copyfilewrite += 1,
                    "DSMAllocate" => pgwaittypeio.dsmallocate += 1,
                    "DSMFillZeroWrite" => pgwaittypeio.dsmfillzerowrite += 1,
                    "DataFileExtend" => pgwaittypeio.datafileextend += 1,
                    "DataFileFlush" => pgwaittypeio.datafileflush += 1,
                    "DataFileImmediateSync" => pgwaittypeio.datafileimmediatesync += 1,
                    "DataFilePrefetch" => pgwaittypeio.datafileprefetch += 1,
                    "DataFileRead" => pgwaittypeio.datafileread += 1,
                    "DataFileSync" => pgwaittypeio.datafilesync += 1,
                    "DataFileTruncate" => pgwaittypeio.datafiletruncate += 1,
                    "DataFileWrite" => pgwaittypeio.datafilewrite += 1,
                    "LockFileAddToDataDirRead" => pgwaittypeio.lockfileaddtodatadirread += 1,
                    "LockFileAddToDataDirSync" => pgwaittypeio.lockfileaddtodatadirsync += 1,
                    "LockFileAddToDataDirWrite" => pgwaittypeio.lockfileaddtodatadirwrite += 1,
                    "LockFileCreateRead" => pgwaittypeio.lockfilecreateread += 1,
                    "LockFileCreateSync" => pgwaittypeio.lockfilecreatesync += 1,
                    "LockFileCreateWrite" => pgwaittypeio.lockfilecreatewrite += 1,
                    "LockFileReCheckDataDirRead" => pgwaittypeio.lockfilerecheckdatadirread += 1,
                    "LogicalRewriteCheckpointSync" => {
                        pgwaittypeio.logicalrewritecheckpointsync += 1
                    }
                    "LogicalRewriteMappingSync" => pgwaittypeio.logicalrewritemappingsync += 1,
                    "LogicalRewriteMappingWrite" => pgwaittypeio.logicalrewritemappingwrite += 1,
                    "LogicalRewriteSync" => pgwaittypeio.logicalrewritesync += 1,
                    "LogicalRewriteTruncate" => pgwaittypeio.logicalrewritetruncate += 1,
                    "LogicalRewriteWrite" => pgwaittypeio.logicalrewritewrite += 1,
                    "RelationMapRead" => pgwaittypeio.relationmapread += 1,
                    "RelationMapReplace" => pgwaittypeio.relationmapreplace += 1,
                    "RelationMapWrite" => pgwaittypeio.relationmapwrite += 1,
                    "ReorderBufferRead" => pgwaittypeio.reorderbufferread += 1,
                    "ReorderBufferWrite" => pgwaittypeio.reorderbufferwrite += 1,
                    "ReorderLogicalMappingRead" => pgwaittypeio.reorderlogicalmappingread += 1,
                    "ReplicationSlotRead" => pgwaittypeio.replicationslotread += 1,
                    "ReplicationSlotRestoreSync" => pgwaittypeio.replicationslotrestoresync += 1,
                    "ReplicationSlotSync" => pgwaittypeio.replicationslotsync += 1,
                    "ReplicationSlotWrite" => pgwaittypeio.replicationslotwrite += 1,
                    "SLRUFlushSync" => pgwaittypeio.slruflushsync += 1,
                    "SLRURead" => pgwaittypeio.slruread += 1,
                    "SLRUSync" => pgwaittypeio.slrusync += 1,
                    "SLRUWrite" => pgwaittypeio.slruwrite += 1,
                    "SnapbuildRead" => pgwaittypeio.snapbuildread += 1,
                    "SnapbuildSync" => pgwaittypeio.snapbuildsync += 1,
                    "SnapbuildWrite" => pgwaittypeio.snapbuildwrite += 1,
                    "TimeLineHistoryFileSync" => pgwaittypeio.timelinehistoryfilesync += 1,
                    "TimeLineHistoryFileWrite" => pgwaittypeio.timelinehistoryfilewrite += 1,
                    "TimeLineHistoryRead" => pgwaittypeio.timelinehistoryread += 1,
                    "TimeLineHistorySync" => pgwaittypeio.timelinehistorysync += 1,
                    "TimeLineHistoryWrite" => pgwaittypeio.timelinehistorywrite += 1,
                    "TwophaseFileRead" => pgwaittypeio.twophasefileread += 1,
                    "TwophaseFileSync" => pgwaittypeio.twophasefilesync += 1,
                    "TwophaseFileWrite" => pgwaittypeio.twophasefilewrite += 1,
                    "VersionFileSync" => pgwaittypeio.versionfilesync += 1,
                    "VersionFileWrite" => pgwaittypeio.versionfilewrite += 1,
                    "WalBootstrapSync" => pgwaittypeio.walbootstrapsync += 1,
                    "WalBootstrapWrite" => pgwaittypeio.walbootstrapwrite += 1,
                    "WalCopyRead" => pgwaittypeio.walcopyread += 1,
                    "WalCopySync" => pgwaittypeio.walcopysync += 1,
                    "WalCopyWrite" => pgwaittypeio.walcopywrite += 1,
                    "WalInitSync" => pgwaittypeio.walinitsync += 1,
                    "WalInitWrite" => pgwaittypeio.walinitwrite += 1,
                    "WalRead" => pgwaittypeio.walread += 1,
                    "WalSenderTimelineHistoryRead" => {
                        pgwaittypeio.walsendertimelinehistoryread += 1
                    }
                    "WalSync" => pgwaittypeio.walsync += 1,
                    "WalSyncMethodAssign" => pgwaittypeio.walsyncmethodassign += 1,
                    "WalWrite" => pgwaittypeio.walwrite += 1,
                    &_ => {}
                };
            });
        pgwaittypeio
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeIPC {
    appendready: usize,
    archivecleanupcommand: usize,
    archivecommand: usize,
    backendtermination: usize,
    backupwaitwalarchive: usize,
    bgworkershutdown: usize,
    bgworkerstartup: usize,
    btreepage: usize,
    bufferio: usize,
    checkpointdone: usize,
    checkpointstart: usize,
    executegather: usize,
    hashbatchallocate: usize,
    hashbatchelect: usize,
    hashbatchload: usize,
    hashbuildallocate: usize,
    hashbuildelect: usize,
    hashbuildhashinner: usize,
    hashbuildhashouter: usize,
    hashgrowbatchesdecide: usize,
    hashgrowbatcheselect: usize,
    hashgrowbatchesfinish: usize,
    hashgrowbatchesreallocate: usize,
    hashgrowbatchesrepartition: usize,
    hashgrowbucketselect: usize,
    hashgrowbucketsreallocate: usize,
    hashgrowbucketsreinsert: usize,
    logicalapplysenddata: usize,
    logicalparallelapplystatechange: usize,
    logicalsyncdata: usize,
    logicalsyncstatechange: usize,
    messagequeueinternal: usize,
    messagequeueputmessage: usize,
    messagequeuereceive: usize,
    messagequeuesend: usize,
    parallelbitmapscan: usize,
    parallelcreateindexscan: usize,
    parallelfinish: usize,
    procarraygroupupdate: usize,
    procsignalbarrier: usize,
    promote: usize,
    recoveryconflictsnapshot: usize,
    recoveryconflicttablespace: usize,
    recoveryendcommand: usize,
    recoverypause: usize,
    replicationorigindrop: usize,
    replicationslotdrop: usize,
    restorecommand: usize,
    safesnapshot: usize,
    syncrep: usize,
    walreceiverexit: usize,
    walreceiverwaitstart: usize,
    xactgroupupdate: usize,
}
impl PgWaitTypeIPC {
    pub async fn new() -> Self {
        PgWaitTypeIPC::default()
    }
    pub async fn process_pg_stat_activity(pg_stat_activity: Vec<PgStatActivity>) -> PgWaitTypeIPC {
        let mut pgwaittypeipc = PgWaitTypeIPC::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type == Some("IPC".to_string()))
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "AppendReady" => pgwaittypeipc.appendready += 1,
                    "ArchiveCleanupCommand" => pgwaittypeipc.archivecleanupcommand += 1,
                    "ArchiveCommand" => pgwaittypeipc.archivecommand += 1,
                    "BackendTermination" => pgwaittypeipc.backendtermination += 1,
                    "BackupWaitWalArchive" => pgwaittypeipc.backupwaitwalarchive += 1,
                    "BgWorkerShutdown" => pgwaittypeipc.bgworkershutdown += 1,
                    "BgWorkerStartup" => pgwaittypeipc.bgworkerstartup += 1,
                    "BtreePage" => pgwaittypeipc.btreepage += 1,
                    "BufferIO" => pgwaittypeipc.bufferio += 1,
                    "CheckpointDone" => pgwaittypeipc.checkpointdone += 1,
                    "CheckpointStart" => pgwaittypeipc.checkpointstart += 1,
                    "ExecuteGather" => pgwaittypeipc.executegather += 1,
                    "HashBatchAllocate" => pgwaittypeipc.hashbatchallocate += 1,
                    "HashBatchElect" => pgwaittypeipc.hashbatchelect += 1,
                    "HashBatchLoad" => pgwaittypeipc.hashbatchload += 1,
                    "HashBuildAllocate" => pgwaittypeipc.hashbuildallocate += 1,
                    "HashBuildElect" => pgwaittypeipc.hashbuildelect += 1,
                    "HashBuildHashInner" => pgwaittypeipc.hashbuildhashinner += 1,
                    "HashBuildHashOuter" => pgwaittypeipc.hashbuildhashouter += 1,
                    "HashGrowBatchesDecide" => pgwaittypeipc.hashgrowbatchesdecide += 1,
                    "HashGrowBatchesElect" => pgwaittypeipc.hashgrowbatcheselect += 1,
                    "HashGrowBatchesFinish" => pgwaittypeipc.hashgrowbatchesfinish += 1,
                    "HashGrowBatchesReallocate" => pgwaittypeipc.hashgrowbatchesreallocate += 1,
                    "HashGrowBatchesRepartition" => pgwaittypeipc.hashgrowbatchesrepartition += 1,
                    "HashGrowBucketsElect" => pgwaittypeipc.hashgrowbucketselect += 1,
                    "HashGrowBucketsReallocate" => pgwaittypeipc.hashgrowbucketsreallocate += 1,
                    "HashGrowBucketsReinsert" => pgwaittypeipc.hashgrowbucketsreinsert += 1,
                    "LogicalApplySendData" => pgwaittypeipc.logicalapplysenddata += 1,
                    "LogicalParallelApplyStateChange" => {
                        pgwaittypeipc.logicalparallelapplystatechange += 1
                    }
                    "LogicalSyncData" => pgwaittypeipc.logicalsyncdata += 1,
                    "LogicalSyncStateChange" => pgwaittypeipc.logicalsyncstatechange += 1,
                    "MessageQueueInternal" => pgwaittypeipc.messagequeueinternal += 1,
                    "MessageQueuePutMessage" => pgwaittypeipc.messagequeueputmessage += 1,
                    "MessageQueueReceive" => pgwaittypeipc.messagequeuereceive += 1,
                    "MessageQueueSend" => pgwaittypeipc.messagequeuesend += 1,
                    "ParallelBitmapScan" => pgwaittypeipc.parallelbitmapscan += 1,
                    "ParallelCreateIndexScan" => pgwaittypeipc.parallelcreateindexscan += 1,
                    "ParallelFinish" => pgwaittypeipc.parallelfinish += 1,
                    "ProcArrayGroupUpdate" => pgwaittypeipc.procarraygroupupdate += 1,
                    "ProcSignalBarrier" => pgwaittypeipc.procsignalbarrier += 1,
                    "Promote" => pgwaittypeipc.promote += 1,
                    "RecoveryConflictSnapshot" => pgwaittypeipc.recoveryconflictsnapshot += 1,
                    "RecoveryConflictTablespace" => pgwaittypeipc.recoveryconflicttablespace += 1,
                    "RecoveryEndCommand" => pgwaittypeipc.recoveryendcommand += 1,
                    "RecoveryPause" => pgwaittypeipc.recoverypause += 1,
                    "ReplicationOriginDrop" => pgwaittypeipc.replicationorigindrop += 1,
                    "ReplicationSlotDrop" => pgwaittypeipc.replicationslotdrop += 1,
                    "RestoreCommand" => pgwaittypeipc.restorecommand += 1,
                    "SafeSnapshot" => pgwaittypeipc.safesnapshot += 1,
                    "SyncRep" => pgwaittypeipc.syncrep += 1,
                    "WalReceiverExit" => pgwaittypeipc.walreceiverexit += 1,
                    "WalReceiverWaitStart" => pgwaittypeipc.walreceiverwaitstart += 1,
                    "XactGroupUpdate" => pgwaittypeipc.xactgroupupdate += 1,
                    &_ => {}
                };
            });
        pgwaittypeipc
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeLock {
    advisory: usize,
    applytransaction: usize,
    extend: usize,
    frozenid: usize,
    object: usize,
    page: usize,
    relation: usize,
    spectoken: usize,
    transactionid: usize,
    tuple: usize,
    userlock: usize,
    virtualxid: usize,
}
impl PgWaitTypeLock {
    pub async fn new() -> Self {
        PgWaitTypeLock::default()
    }
    pub async fn process_pg_stat_activity(pg_stat_activity: Vec<PgStatActivity>) -> PgWaitTypeLock {
        let mut pgwaittypelock = PgWaitTypeLock::new().await;
        pg_stat_activity
            .iter()
            .filter(|r| r.wait_event_type == Some("Lock".to_string()))
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
                    &_ => {}
                };
            });
        pgwaittypelock
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeLWLock {
    addinsheminit: usize,
    autofile: usize,
    autovacuum: usize,
    autovacuumschedule: usize,
    backgroundworker: usize,
    btreevacuum: usize,
    buffercontent: usize,
    buffermapping: usize,
    checkpointercomm: usize,
    committs: usize,
    committsbuffer: usize,
    committsslru: usize,
    controlfile: usize,
    dynamicsharedmemorycontrol: usize,
    lockfastpath: usize,
    lockmanager: usize,
    logicalreplauncerdsa: usize,
    logicalreplauncerhash: usize,
    logicalrepworker: usize,
    multixactgen: usize,
    multixactmemberbuffer: usize,
    multixactmemberslru: usize,
    multixactoffsetbuffer: usize,
    multixactoffsetslru: usize,
    multixacttruncation: usize,
    notifybuffer: usize,
    notifyqueue: usize,
    notifyqueuetail: usize,
    notifyslru: usize,
    oidgen: usize,
    oldsnapshottimemap: usize,
    parallelappend: usize,
    parallelhashjoin: usize,
    parallelquerydsa: usize,
    persessiondsa: usize,
    persessionrecordtype: usize,
    persessionrecordtypmod: usize,
    perxactpredicatelist: usize,
    pgstatsdata: usize,
    pgstatsdsa: usize,
    pgstatshash: usize,
    predicatelockmanager: usize,
    procarray: usize,
    relationmapping: usize,
    relcacheinit: usize,
    replicationorigin: usize,
    replicationoriginstate: usize,
    replicationslotallocation: usize,
    replicationslotcontrol: usize,
    replicationslotio: usize,
    serialbuffer: usize,
    serializablefinishedlist: usize,
    serializablepredicatelist: usize,
    serializablexacthash: usize,
    serialslru: usize,
    sharedtidbitmap: usize,
    sharedtuplestore: usize,
    shmemindex: usize,
    sinvalread: usize,
    sinvalwrite: usize,
    subtransbuffer: usize,
    subtransslru: usize,
    syncrep: usize,
    syncscan: usize,
    tablespacecreate: usize,
    twophasestate: usize,
    walbufmapping: usize,
    walinsert: usize,
    walwrite: usize,
    wraplimitsvacuum: usize,
    xactbuffer: usize,
    xactslru: usize,
    xacttruncation: usize,
    xidgen: usize,
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
            .filter(|r| r.wait_event_type == Some("LWLock".to_string()))
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "AddinShmemInit" => pgwaittypelwlock.addinsheminit += 1,
                    "AutoFile" => pgwaittypelwlock.autofile += 1,
                    "Autovacuum" => pgwaittypelwlock.autovacuum += 1,
                    "AutovacuumSchedule" => pgwaittypelwlock.autovacuumschedule += 1,
                    "BackgroundWorker" => pgwaittypelwlock.backgroundworker += 1,
                    "BtreeVacuum" => pgwaittypelwlock.btreevacuum += 1,
                    "BufferContent" => pgwaittypelwlock.buffercontent += 1,
                    "BufferMapping" => pgwaittypelwlock.buffermapping += 1,
                    "CheckpointerComm" => pgwaittypelwlock.checkpointercomm += 1,
                    "CommitTs" => pgwaittypelwlock.committs += 1,
                    "CommitTsBuffer" => pgwaittypelwlock.committsbuffer += 1,
                    "CommitTsSLRU" => pgwaittypelwlock.committsslru += 1,
                    "ControlFile" => pgwaittypelwlock.controlfile += 1,
                    "DynamicSharedMemoryControl" => {
                        pgwaittypelwlock.dynamicsharedmemorycontrol += 1
                    }
                    "LockFastPath" => pgwaittypelwlock.lockfastpath += 1,
                    "LockManager" => pgwaittypelwlock.lockmanager += 1,
                    "LogicalRepLauncerDSA" => pgwaittypelwlock.logicalreplauncerdsa += 1,
                    "LogicalRepLauncerHash" => pgwaittypelwlock.logicalreplauncerhash += 1,
                    "LogicalRepWorker" => pgwaittypelwlock.logicalrepworker += 1,
                    "MultiXactGen" => pgwaittypelwlock.multixactgen += 1,
                    "MultiXactMemberBuffer" => pgwaittypelwlock.multixactmemberbuffer += 1,
                    "MultiXactMemberSLRU" => pgwaittypelwlock.multixactmemberslru += 1,
                    "MultiXactOffsetBuffer" => pgwaittypelwlock.multixactoffsetbuffer += 1,
                    "MultiXactOffsetSLRU" => pgwaittypelwlock.multixactoffsetslru += 1,
                    "MultiXactTruncation" => pgwaittypelwlock.multixacttruncation += 1,
                    "NotifyBuffer" => pgwaittypelwlock.notifybuffer += 1,
                    "NotifyQueue" => pgwaittypelwlock.notifyqueue += 1,
                    "NotifyQueueTail" => pgwaittypelwlock.notifyqueuetail += 1,
                    "NotifySLRU" => pgwaittypelwlock.notifyslru += 1,
                    "OidGen" => pgwaittypelwlock.oidgen += 1,
                    "OldSnapshotTimeMap" => pgwaittypelwlock.oldsnapshottimemap += 1,
                    "ParallelAppend" => pgwaittypelwlock.parallelappend += 1,
                    "ParallelHashJoin" => pgwaittypelwlock.parallelhashjoin += 1,
                    "ParallelQueryDSA" => pgwaittypelwlock.parallelquerydsa += 1,
                    "PerSessionDSA" => pgwaittypelwlock.persessiondsa += 1,
                    "PerSessionRecordType" => pgwaittypelwlock.persessionrecordtype += 1,
                    "PerSessionRecordTypMod" => pgwaittypelwlock.persessionrecordtypmod += 1,
                    "PerXactPredicateList" => pgwaittypelwlock.perxactpredicatelist += 1,
                    "PgStatsData" => pgwaittypelwlock.pgstatsdata += 1,
                    "PgStatsDSA" => pgwaittypelwlock.pgstatsdsa += 1,
                    "PgStatsHash" => pgwaittypelwlock.pgstatshash += 1,
                    "PredicateLockManager" => pgwaittypelwlock.predicatelockmanager += 1,
                    "ProcArray" => pgwaittypelwlock.procarray += 1,
                    "RelationMapping" => pgwaittypelwlock.relationmapping += 1,
                    "RelCacheInit" => pgwaittypelwlock.relcacheinit += 1,
                    "ReplicationOrigin" => pgwaittypelwlock.replicationorigin += 1,
                    "ReplicationOriginState" => pgwaittypelwlock.replicationoriginstate += 1,
                    "ReplicationSlotAllocation" => pgwaittypelwlock.replicationslotallocation += 1,
                    "ReplicationSlotControl" => pgwaittypelwlock.replicationslotcontrol += 1,
                    "ReplicationSlotIO" => pgwaittypelwlock.replicationslotio += 1,
                    "SerialBuffer" => pgwaittypelwlock.serialbuffer += 1,
                    "SerializableFinishedList" => pgwaittypelwlock.serializablefinishedlist += 1,
                    "SerializablePredicateList" => pgwaittypelwlock.serializablepredicatelist += 1,
                    "SerializableXactHash" => pgwaittypelwlock.serializablexacthash += 1,
                    "SerialSLRU" => pgwaittypelwlock.serialslru += 1,
                    "SharedTidBitmap" => pgwaittypelwlock.sharedtidbitmap += 1,
                    "SharedTupleStore" => pgwaittypelwlock.sharedtuplestore += 1,
                    "ShmemIndex" => pgwaittypelwlock.shmemindex += 1,
                    "SInvalRead" => pgwaittypelwlock.sinvalread += 1,
                    "SInvalWrite" => pgwaittypelwlock.sinvalwrite += 1,
                    "SubtransBuffer" => pgwaittypelwlock.subtransbuffer += 1,
                    "SubtransSLRU" => pgwaittypelwlock.subtransslru += 1,
                    "SyncRep" => pgwaittypelwlock.syncrep += 1,
                    "SyncScan" => pgwaittypelwlock.syncscan += 1,
                    "TablespaceCreate" => pgwaittypelwlock.tablespacecreate += 1,
                    "TwoPhaseState" => pgwaittypelwlock.twophasestate += 1,
                    "WALBufMapping" => pgwaittypelwlock.walbufmapping += 1,
                    "WALInsert" => pgwaittypelwlock.walinsert += 1,
                    "WALWrite" => pgwaittypelwlock.walwrite += 1,
                    "WrapLimitsVacuum" => pgwaittypelwlock.wraplimitsvacuum += 1,
                    "XactBuffer" => pgwaittypelwlock.xactbuffer += 1,
                    "XactSLRU" => pgwaittypelwlock.xactslru += 1,
                    "XactTruncation" => pgwaittypelwlock.xacttruncation += 1,
                    "XidGen" => pgwaittypelwlock.xidgen += 1,
                    &_ => {}
                };
            });
        pgwaittypelwlock
    }
}
#[derive(Debug, Default)]
pub struct PgWaitTypeTimeout {
    basebackupthrottle: usize,
    checkpointerwritedelay: usize,
    pgsleep: usize,
    recoveryapplydelay: usize,
    recoveryretrieveretryinterval: usize,
    registersyncrequest: usize,
    spindelay: usize,
    vacuumdelay: usize,
    vacuumtruncate: usize,
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
            .filter(|r| r.wait_event_type == Some("Timeout".to_string()))
            .map(|r| r.wait_event.clone())
            .for_each(|r| {
                match r.unwrap_or_default().as_ref() {
                    "BaseBackupThrottle" => pgwaittypetimeout.basebackupthrottle += 1,
                    "CheckpoinWriteDelay" => pgwaittypetimeout.checkpointerwritedelay += 1,
                    "PgSleep" => pgwaittypetimeout.pgsleep += 1,
                    "RecoveryApplyDelay" => pgwaittypetimeout.recoveryapplydelay += 1,
                    "RecoveryRetrieveRetryInterval" => {
                        pgwaittypetimeout.recoveryretrieveretryinterval += 1
                    }
                    "RegisterSyncRequest" => pgwaittypetimeout.registersyncrequest += 1,
                    "SpinDelay" => pgwaittypetimeout.spindelay += 1,
                    "VacuumDelay" => pgwaittypetimeout.vacuumdelay += 1,
                    "VacuumTruncate" => pgwaittypetimeout.vacuumtruncate += 1,
                    &_ => {}
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

        println!("tick");
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
        println!("{:?}", DATA.wait_event_activity.read().await);
    }
}
