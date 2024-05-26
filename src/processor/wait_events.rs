use crate::DATA;
use serde::{Deserialize, Serialize};

pub struct WaitEvents {}
impl WaitEvents {
    pub async fn process_waits_and_add_to_data() {
        PgWaitTypes::process_last_pg_stat_activity().await;
        PgWaitTypeActivity::process_last_pg_stat_activity().await;
        PgWaitTypeBufferPin::process_last_pg_stat_activity().await;
        PgWaitTypeClient::process_last_pg_stat_activity().await;
        PgWaitTypeExtension::process_last_pg_stat_activity().await;
        PgWaitTypeIO::process_last_pg_stat_activity().await;
        PgWaitTypeIPC::process_last_pg_stat_activity().await;
        PgWaitTypeLock::process_last_pg_stat_activity().await;
        PgWaitTypeLWLock::process_last_pg_stat_activity().await;
        PgWaitTypeTimeout::process_last_pg_stat_activity().await;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PgWaitTypes {
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

impl PgWaitTypes {
    async fn new() -> Self {
        PgWaitTypes::default()
    }
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_types = PgWaitTypes::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match &element.wait_event_type {
                    None => pg_wait_types.on_cpu += 1,
                    Some(wait_event_type) => match wait_event_type.as_str() {
                        "activity" => pg_wait_types.activity += 1,
                        "buffer_pin" => pg_wait_types.buffer_pin += 1,
                        "client" => pg_wait_types.client += 1,
                        "extension" => pg_wait_types.extension += 1,
                        "io" => pg_wait_types.io += 1,
                        "ipc" => pg_wait_types.ipc += 1,
                        "lock" => pg_wait_types.lock += 1,
                        "lwlock" => pg_wait_types.lwlock += 1,
                        "timeout" => pg_wait_types.timeout += 1,
                        _ => {}
                    },
                },
                _ => { /* don't count sessions that are not active */ }
            }
        }

        DATA.wait_event_types
            .write()
            .await
            .push_back((*timestamp, pg_wait_types));
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    async fn new() -> Self {
        PgWaitTypeActivity::default()
    }
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_activity = PgWaitTypeActivity::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "activity" => match element.wait_event.as_deref().unwrap_or("") {
                        "archivermain" => pg_wait_type_activity.archivermain += 1,
                        "autovacuummain" => pg_wait_type_activity.autovacuummain += 1,
                        "bgwriterhibernate" => pg_wait_type_activity.bgwriterhibernate += 1,
                        "bgwritermain" => pg_wait_type_activity.bgwritermain += 1,
                        "checkpointermain" => pg_wait_type_activity.checkpointermain += 1,
                        "logicalapplymain" => pg_wait_type_activity.logicalapplymain += 1,
                        "logicallaunchermain" => pg_wait_type_activity.logicallaunchermain += 1,
                        "logicalparallelapplymain" => {
                            pg_wait_type_activity.logicalparallelapplymain += 1
                        }
                        "recoverywalstream" => pg_wait_type_activity.recoverywalstream += 1,
                        "sysloggermain" => pg_wait_type_activity.sysloggermain += 1,
                        "walreceivermain" => pg_wait_type_activity.walreceivermain += 1,
                        "walsendermain" => pg_wait_type_activity.walsendermain += 1,
                        "walwritermain" => pg_wait_type_activity.walwritermain += 1,
                        &_ => pg_wait_type_activity.other += 1,
                    },
                    _ => { /* wait_event_type != activity */ }
                },
                _ => { /* not active */ }
            }
        }
        DATA.wait_event_activity
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_activity));
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PgWaitTypeBufferPin {
    pub bufferpin: usize,
    pub other: usize,
}
impl PgWaitTypeBufferPin {
    pub async fn new() -> Self {
        PgWaitTypeBufferPin::default()
    }
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_bufferpin = PgWaitTypeBufferPin::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "bufferpin" => match element.wait_event.as_deref().unwrap_or("") {
                        "bufferpin" => pg_wait_type_bufferpin.bufferpin += 1,
                        _ => pg_wait_type_bufferpin.other += 1,
                    },
                    _ => { /* wait_event_type != bufferpin */ }
                },
                _ => { /* not active */ }
            }
        }
        DATA.wait_event_bufferpin
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_bufferpin));
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_client = PgWaitTypeClient::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "client" => match element.wait_event.as_deref().unwrap_or("") {
                        "clientread" => pg_wait_type_client.clientread += 1,
                        "clientwrite" => pg_wait_type_client.clientwrite += 1,
                        "gssopenserver" => pg_wait_type_client.gssopenserver += 1,
                        "libpqwalreceiverconnect" => {
                            pg_wait_type_client.libpqwalreceiverconnect += 1
                        }
                        "Libpqwalreceiverreceive" => {
                            pg_wait_type_client.libpqwalreceiverreceive += 1
                        }
                        "sslopenserver" => pg_wait_type_client.sslopenserver += 1,
                        "walsenderwaitforwal" => pg_wait_type_client.walsenderwaitforwal += 1,
                        "walsenderwritedata" => pg_wait_type_client.walsenderwritedata += 1,
                        _ => pg_wait_type_client.other += 1,
                    },
                    _ => { /* wait_event_type != client */ }
                },
                _ => { /* not active */ }
            }
        }
        DATA.wait_event_client
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_client));
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PgWaitTypeExtension {
    pub extension: usize,
    pub other: usize,
}
impl PgWaitTypeExtension {
    pub async fn new() -> Self {
        PgWaitTypeExtension::default()
    }
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_extension = PgWaitTypeExtension::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "extension" => match element.wait_event.as_deref().unwrap_or("") {
                        "extension" => pg_wait_type_extension.extension += 1,
                        _ => pg_wait_type_extension.other += 1,
                    },
                    _ => { /* wait_event_type != extension */ }
                },
                _ => { /* not active */ }
            }
        }

        DATA.wait_event_extension
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_extension));
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_io = PgWaitTypeIO::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "io" => match element.wait_event.as_deref().unwrap_or("") {
                        "basebackupread" => pg_wait_type_io.basebackupread += 1,
                        "basebackupsync" => pg_wait_type_io.basebackupsync += 1,
                        "basebackupwrite" => pg_wait_type_io.basebackupwrite += 1,
                        "buffileread" => pg_wait_type_io.buffileread += 1,
                        "buffiletruncate" => pg_wait_type_io.buffiletruncate += 1,
                        "buffilewrite" => pg_wait_type_io.buffilewrite += 1,
                        "controlfileread" => pg_wait_type_io.controlfileread += 1,
                        "controlfilesync" => pg_wait_type_io.controlfilesync += 1,
                        "controlfilesyncupdate" => pg_wait_type_io.controlfilesyncupdate += 1,
                        "controlfilewrite" => pg_wait_type_io.controlfilewrite += 1,
                        "controlfilewriteupdate" => pg_wait_type_io.controlfilewriteupdate += 1,
                        "copyfileread" => pg_wait_type_io.copyfileread += 1,
                        "copyfilewrite" => pg_wait_type_io.copyfilewrite += 1,
                        "dsmallocate" => pg_wait_type_io.dsmallocate += 1,
                        "dsmfillzerowrite" => pg_wait_type_io.dsmfillzerowrite += 1,
                        "datafileextend" => pg_wait_type_io.datafileextend += 1,
                        "datafileflush" => pg_wait_type_io.datafileflush += 1,
                        "datafileimmediatesync" => pg_wait_type_io.datafileimmediatesync += 1,
                        "datafileprefetch" => pg_wait_type_io.datafileprefetch += 1,
                        "datafileread" => pg_wait_type_io.datafileread += 1,
                        "datafilesync" => pg_wait_type_io.datafilesync += 1,
                        "datafiletruncate" => pg_wait_type_io.datafiletruncate += 1,
                        "datafilewrite" => pg_wait_type_io.datafilewrite += 1,
                        "lockfileaddtodatadirread" => pg_wait_type_io.lockfileaddtodatadirread += 1,
                        "lockfileaddtodatadirsync" => pg_wait_type_io.lockfileaddtodatadirsync += 1,
                        "lockfileaddtodatadirwrite" => {
                            pg_wait_type_io.lockfileaddtodatadirwrite += 1
                        }
                        "lockfilecreateread" => pg_wait_type_io.lockfilecreateread += 1,
                        "lockfilecreatesync" => pg_wait_type_io.lockfilecreatesync += 1,
                        "lockfilecreatewrite" => pg_wait_type_io.lockfilecreatewrite += 1,
                        "lockfilerecheckdatadirread" => {
                            pg_wait_type_io.lockfilerecheckdatadirread += 1
                        }
                        "logicalrewritecheckpointsync" => {
                            pg_wait_type_io.logicalrewritecheckpointsync += 1
                        }
                        "logicalrewritemappingsync" => {
                            pg_wait_type_io.logicalrewritemappingsync += 1
                        }
                        "logicalrewritemappingwrite" => {
                            pg_wait_type_io.logicalrewritemappingwrite += 1
                        }
                        "logicalrewritesync" => pg_wait_type_io.logicalrewritesync += 1,
                        "logicalrewritetruncate" => pg_wait_type_io.logicalrewritetruncate += 1,
                        "logicalrewritewrite" => pg_wait_type_io.logicalrewritewrite += 1,
                        "relationmapread" => pg_wait_type_io.relationmapread += 1,
                        "relationmapreplace" => pg_wait_type_io.relationmapreplace += 1,
                        "relationmapwrite" => pg_wait_type_io.relationmapwrite += 1,
                        "reorderbufferread" => pg_wait_type_io.reorderbufferread += 1,
                        "reorderbufferwrite" => pg_wait_type_io.reorderbufferwrite += 1,
                        "reorderlogicalmappingread" => {
                            pg_wait_type_io.reorderlogicalmappingread += 1
                        }
                        "replicationslotread" => pg_wait_type_io.replicationslotread += 1,
                        "replicationslotrestoresync" => {
                            pg_wait_type_io.replicationslotrestoresync += 1
                        }
                        "replicationslotsync" => pg_wait_type_io.replicationslotsync += 1,
                        "replicationslotwrite" => pg_wait_type_io.replicationslotwrite += 1,
                        "slruflushsync" => pg_wait_type_io.slruflushsync += 1,
                        "slruread" => pg_wait_type_io.slruread += 1,
                        "slrusync" => pg_wait_type_io.slrusync += 1,
                        "slruwrite" => pg_wait_type_io.slruwrite += 1,
                        "snapbuildread" => pg_wait_type_io.snapbuildread += 1,
                        "snapbuildsync" => pg_wait_type_io.snapbuildsync += 1,
                        "snapbuildwrite" => pg_wait_type_io.snapbuildwrite += 1,
                        "timelinehistoryfilesync" => pg_wait_type_io.timelinehistoryfilesync += 1,
                        "timelinehistoryfilewrite" => pg_wait_type_io.timelinehistoryfilewrite += 1,
                        "timelinehistoryread" => pg_wait_type_io.timelinehistoryread += 1,
                        "timelinehistorysync" => pg_wait_type_io.timelinehistorysync += 1,
                        "timelinehistorywrite" => pg_wait_type_io.timelinehistorywrite += 1,
                        "twophasefileread" => pg_wait_type_io.twophasefileread += 1,
                        "twophasefilesync" => pg_wait_type_io.twophasefilesync += 1,
                        "twophasefilewrite" => pg_wait_type_io.twophasefilewrite += 1,
                        "versionfilesync" => pg_wait_type_io.versionfilesync += 1,
                        "versionfilewrite" => pg_wait_type_io.versionfilewrite += 1,
                        "walbootstrapsync" => pg_wait_type_io.walbootstrapsync += 1,
                        "walbootstrapwrite" => pg_wait_type_io.walbootstrapwrite += 1,
                        "walcopyread" => pg_wait_type_io.walcopyread += 1,
                        "walcopysync" => pg_wait_type_io.walcopysync += 1,
                        "walcopywrite" => pg_wait_type_io.walcopywrite += 1,
                        "walinitsync" => pg_wait_type_io.walinitsync += 1,
                        "walinitwrite" => pg_wait_type_io.walinitwrite += 1,
                        "walread" => pg_wait_type_io.walread += 1,
                        "walsendertimelinehistoryread" => {
                            pg_wait_type_io.walsendertimelinehistoryread += 1
                        }
                        "walsync" => pg_wait_type_io.walsync += 1,
                        "walsyncmethodassign" => pg_wait_type_io.walsyncmethodassign += 1,
                        "walwrite" => pg_wait_type_io.walwrite += 1,
                        _ => pg_wait_type_io.other += 1,
                    },
                    _ => { /* wait_event_type != io */ }
                },
                _ => { /* not active */ }
            }
        }
        DATA.wait_event_io
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_io));
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_ipc = PgWaitTypeIPC::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "ipc" => match element.wait_event.as_deref().unwrap_or("") {
                        "appendready" => pg_wait_type_ipc.appendready += 1,
                        "archivecleanupcommand" => pg_wait_type_ipc.archivecleanupcommand += 1,
                        "archivecommand" => pg_wait_type_ipc.archivecommand += 1,
                        "backendtermination" => pg_wait_type_ipc.backendtermination += 1,
                        "backupwaitwalarchive" => pg_wait_type_ipc.backupwaitwalarchive += 1,
                        "bgworkershutdown" => pg_wait_type_ipc.bgworkershutdown += 1,
                        "bgworkerstartup" => pg_wait_type_ipc.bgworkerstartup += 1,
                        "btreepage" => pg_wait_type_ipc.btreepage += 1,
                        "bufferio" => pg_wait_type_ipc.bufferio += 1,
                        "checkpointdone" => pg_wait_type_ipc.checkpointdone += 1,
                        "checkpointstart" => pg_wait_type_ipc.checkpointstart += 1,
                        "executegather" => pg_wait_type_ipc.executegather += 1,
                        "hashbatchallocate" => pg_wait_type_ipc.hashbatchallocate += 1,
                        "hashbatchelect" => pg_wait_type_ipc.hashbatchelect += 1,
                        "hashbatchload" => pg_wait_type_ipc.hashbatchload += 1,
                        "hashbuildallocate" => pg_wait_type_ipc.hashbuildallocate += 1,
                        "hashbuildelect" => pg_wait_type_ipc.hashbuildelect += 1,
                        "hashbuildhashinner" => pg_wait_type_ipc.hashbuildhashinner += 1,
                        "hashbuildhashouter" => pg_wait_type_ipc.hashbuildhashouter += 1,
                        "hashgrowbatchesdecide" => pg_wait_type_ipc.hashgrowbatchesdecide += 1,
                        "hashgrowbatcheselect" => pg_wait_type_ipc.hashgrowbatcheselect += 1,
                        "hashgrowbatchesfinish" => pg_wait_type_ipc.hashgrowbatchesfinish += 1,
                        "hashgrowbatchesreallocate" => {
                            pg_wait_type_ipc.hashgrowbatchesreallocate += 1
                        }
                        "hashgrowbatchesrepartition" => {
                            pg_wait_type_ipc.hashgrowbatchesrepartition += 1
                        }
                        "hashgrowbucketselect" => pg_wait_type_ipc.hashgrowbucketselect += 1,
                        "hashgrowbucketsreallocate" => {
                            pg_wait_type_ipc.hashgrowbucketsreallocate += 1
                        }
                        "hashgrowbucketsreinsert" => pg_wait_type_ipc.hashgrowbucketsreinsert += 1,
                        "logicalapplysenddata" => pg_wait_type_ipc.logicalapplysenddata += 1,
                        "logicalparallelapplystatechange" => {
                            pg_wait_type_ipc.logicalparallelapplystatechange += 1
                        }
                        "logicalsyncdata" => pg_wait_type_ipc.logicalsyncdata += 1,
                        "logicalsyncstatechange" => pg_wait_type_ipc.logicalsyncstatechange += 1,
                        "messagequeueinternal" => pg_wait_type_ipc.messagequeueinternal += 1,
                        "messagequeueputmessage" => pg_wait_type_ipc.messagequeueputmessage += 1,
                        "messagequeuereceive" => pg_wait_type_ipc.messagequeuereceive += 1,
                        "messagequeuesend" => pg_wait_type_ipc.messagequeuesend += 1,
                        "parallelbitmapscan" => pg_wait_type_ipc.parallelbitmapscan += 1,
                        "parallelcreateindexscan" => pg_wait_type_ipc.parallelcreateindexscan += 1,
                        "parallelfinish" => pg_wait_type_ipc.parallelfinish += 1,
                        "procarraygroupupdate" => pg_wait_type_ipc.procarraygroupupdate += 1,
                        "procsignalbarrier" => pg_wait_type_ipc.procsignalbarrier += 1,
                        "promote" => pg_wait_type_ipc.promote += 1,
                        "recoveryconflictsnapshot" => {
                            pg_wait_type_ipc.recoveryconflictsnapshot += 1
                        }
                        "recoveryconflicttablespace" => {
                            pg_wait_type_ipc.recoveryconflicttablespace += 1
                        }
                        "recoveryendcommand" => pg_wait_type_ipc.recoveryendcommand += 1,
                        "recoverypause" => pg_wait_type_ipc.recoverypause += 1,
                        "replicationorigindrop" => pg_wait_type_ipc.replicationorigindrop += 1,
                        "replicationslotdrop" => pg_wait_type_ipc.replicationslotdrop += 1,
                        "restorecommand" => pg_wait_type_ipc.restorecommand += 1,
                        "safesnapshot" => pg_wait_type_ipc.safesnapshot += 1,
                        "syncrep" => pg_wait_type_ipc.syncrep += 1,
                        "walreceiverexit" => pg_wait_type_ipc.walreceiverexit += 1,
                        "walreceiverwaitstart" => pg_wait_type_ipc.walreceiverwaitstart += 1,
                        "xactgroupupdate" => pg_wait_type_ipc.xactgroupupdate += 1,
                        _ => pg_wait_type_ipc.other += 1,
                    },
                    _ => { /* wait_event_type != ipc */ }
                },
                _ => { /* not active */ }
            }
        }
        DATA.wait_event_ipc
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_ipc));
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_lock = PgWaitTypeLock::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "lock" => match element.wait_event.as_deref().unwrap_or("") {
                        "advisory" => pg_wait_type_lock.advisory += 1,
                        "applytransaction" => pg_wait_type_lock.applytransaction += 1,
                        "extend" => pg_wait_type_lock.extend += 1,
                        "frozenid" => pg_wait_type_lock.frozenid += 1,
                        "object" => pg_wait_type_lock.object += 1,
                        "page" => pg_wait_type_lock.page += 1,
                        "relation" => pg_wait_type_lock.relation += 1,
                        "spectoken" => pg_wait_type_lock.spectoken += 1,
                        "transactionid" => pg_wait_type_lock.transactionid += 1,
                        "tuple" => pg_wait_type_lock.tuple += 1,
                        "userlock" => pg_wait_type_lock.userlock += 1,
                        "virtualxid" => pg_wait_type_lock.virtualxid += 1,
                        _ => pg_wait_type_lock.other += 1,
                    },
                    _ => { /* wait_event_type != lock */ }
                },
                _ => { /* not active */ }
            }
        }
        DATA.wait_event_lock
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_lock));
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_lwlock = PgWaitTypeLWLock::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "lwlock" => match element.wait_event.as_deref().unwrap_or("") {
                        "addinshmeminit" => pg_wait_type_lwlock.addinsheminit += 1,
                        "autofile" => pg_wait_type_lwlock.autofile += 1,
                        "autovacuum" => pg_wait_type_lwlock.autovacuum += 1,
                        "autovacuumschedule" => pg_wait_type_lwlock.autovacuumschedule += 1,
                        "backgroundworker" => pg_wait_type_lwlock.backgroundworker += 1,
                        "btreevacuum" => pg_wait_type_lwlock.btreevacuum += 1,
                        "buffercontent" => pg_wait_type_lwlock.buffercontent += 1,
                        "buffermapping" => pg_wait_type_lwlock.buffermapping += 1,
                        "checkpointercomm" => pg_wait_type_lwlock.checkpointercomm += 1,
                        "committs" => pg_wait_type_lwlock.committs += 1,
                        "committsbuffer" => pg_wait_type_lwlock.committsbuffer += 1,
                        "committsslru" => pg_wait_type_lwlock.committsslru += 1,
                        "controlfile" => pg_wait_type_lwlock.controlfile += 1,
                        "dynamicsharedmemorycontrol" => {
                            pg_wait_type_lwlock.dynamicsharedmemorycontrol += 1
                        }
                        "lockfastpath" => pg_wait_type_lwlock.lockfastpath += 1,
                        "lockmanager" => pg_wait_type_lwlock.lockmanager += 1,
                        "logicalreplauncerdsa" => pg_wait_type_lwlock.logicalreplauncherdsa += 1,
                        "logicalreplauncerhash" => pg_wait_type_lwlock.logicalreplauncherhash += 1,
                        "logicalrepworker" => pg_wait_type_lwlock.logicalrepworker += 1,
                        "multixactgen" => pg_wait_type_lwlock.multixactgen += 1,
                        "multixactmemberbuffer" => pg_wait_type_lwlock.multixactmemberbuffer += 1,
                        "multixactmemberslru" => pg_wait_type_lwlock.multixactmemberslru += 1,
                        "multixactoffsetbuffer" => pg_wait_type_lwlock.multixactoffsetbuffer += 1,
                        "multixactoffsetslru" => pg_wait_type_lwlock.multixactoffsetslru += 1,
                        "multixacttruncation" => pg_wait_type_lwlock.multixacttruncation += 1,
                        "notifybuffer" => pg_wait_type_lwlock.notifybuffer += 1,
                        "notifyqueue" => pg_wait_type_lwlock.notifyqueue += 1,
                        "notifyqueuetail" => pg_wait_type_lwlock.notifyqueuetail += 1,
                        "notifyslru" => pg_wait_type_lwlock.notifyslru += 1,
                        "oidgen" => pg_wait_type_lwlock.oidgen += 1,
                        "oldsnapshottimemap" => pg_wait_type_lwlock.oldsnapshottimemap += 1,
                        "parallelappend" => pg_wait_type_lwlock.parallelappend += 1,
                        "parallelhashjoin" => pg_wait_type_lwlock.parallelhashjoin += 1,
                        "parallelquerydsa" => pg_wait_type_lwlock.parallelquerydsa += 1,
                        "persessiondsa" => pg_wait_type_lwlock.persessiondsa += 1,
                        "persessionrecordtype" => pg_wait_type_lwlock.persessionrecordtype += 1,
                        "persessionrecordtypmod" => pg_wait_type_lwlock.persessionrecordtypmod += 1,
                        "perxactpredicatelist" => pg_wait_type_lwlock.perxactpredicatelist += 1,
                        "pgstatsdata" => pg_wait_type_lwlock.pgstatsdata += 1,
                        "pgstatsdsa" => pg_wait_type_lwlock.pgstatsdsa += 1,
                        "pgstatshash" => pg_wait_type_lwlock.pgstatshash += 1,
                        "predicatelockmanager" => pg_wait_type_lwlock.predicatelockmanager += 1,
                        "procarray" => pg_wait_type_lwlock.procarray += 1,
                        "relationmapping" => pg_wait_type_lwlock.relationmapping += 1,
                        "relcacheinit" => pg_wait_type_lwlock.relcacheinit += 1,
                        "replicationorigin" => pg_wait_type_lwlock.replicationorigin += 1,
                        "replicationoriginstate" => pg_wait_type_lwlock.replicationoriginstate += 1,
                        "replicationslotallocation" => {
                            pg_wait_type_lwlock.replicationslotallocation += 1
                        }
                        "replicationslotcontrol" => pg_wait_type_lwlock.replicationslotcontrol += 1,
                        "replicationslotio" => pg_wait_type_lwlock.replicationslotio += 1,
                        "serialbuffer" => pg_wait_type_lwlock.serialbuffer += 1,
                        "serializablefinishedlist" => {
                            pg_wait_type_lwlock.serializablefinishedlist += 1
                        }
                        "serializablepredicatelist" => {
                            pg_wait_type_lwlock.serializablepredicatelist += 1
                        }
                        "serializablexacthash" => pg_wait_type_lwlock.serializablexacthash += 1,
                        "serialslru" => pg_wait_type_lwlock.serialslru += 1,
                        "sharedtidbitmap" => pg_wait_type_lwlock.sharedtidbitmap += 1,
                        "sharedtuplestore" => pg_wait_type_lwlock.sharedtuplestore += 1,
                        "shmemindex" => pg_wait_type_lwlock.shmemindex += 1,
                        "sinvalread" => pg_wait_type_lwlock.sinvalread += 1,
                        "sinvalwrite" => pg_wait_type_lwlock.sinvalwrite += 1,
                        "subtransbuffer" => pg_wait_type_lwlock.subtransbuffer += 1,
                        "subtransslru" => pg_wait_type_lwlock.subtransslru += 1,
                        "syncrep" => pg_wait_type_lwlock.syncrep += 1,
                        "syncscan" => pg_wait_type_lwlock.syncscan += 1,
                        "tablespacecreate" => pg_wait_type_lwlock.tablespacecreate += 1,
                        "twophasestate" => pg_wait_type_lwlock.twophasestate += 1,
                        "walbufmapping" => pg_wait_type_lwlock.walbufmapping += 1,
                        "walinsert" => pg_wait_type_lwlock.walinsert += 1,
                        "walwrite" => pg_wait_type_lwlock.walwrite += 1,
                        "wraplimitsvacuum" => pg_wait_type_lwlock.wraplimitsvacuum += 1,
                        "xactbuffer" => pg_wait_type_lwlock.xactbuffer += 1,
                        "xactslru" => pg_wait_type_lwlock.xactslru += 1,
                        "xacttruncation" => pg_wait_type_lwlock.xacttruncation += 1,
                        "xidgen" => pg_wait_type_lwlock.xidgen += 1,
                        _ => pg_wait_type_lwlock.other += 1,
                    },
                    _ => { /* wait_event_type != lwlock */ }
                },
                _ => { /* not active */ }
            }
        }
        DATA.wait_event_lwlock
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_lwlock));
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub async fn process_last_pg_stat_activity() {
        let mut pg_wait_type_timeout = PgWaitTypeTimeout::new().await;
        let readguard = DATA.pg_stat_activity.read().await;
        let (timestamp, pg_stat_activity) = readguard.back().unwrap();

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type.as_deref().unwrap_or("") {
                    "timeout" => match element.wait_event.as_deref().unwrap_or("") {
                        "basebackupthrottle" => pg_wait_type_timeout.basebackupthrottle += 1,
                        "checkpoinwritedelay" => pg_wait_type_timeout.checkpointerwritedelay += 1,
                        "pgsleep" => pg_wait_type_timeout.pgsleep += 1,
                        "recoveryapplydelay" => pg_wait_type_timeout.recoveryapplydelay += 1,
                        "recoveryretrieveretryinterval" => {
                            pg_wait_type_timeout.recoveryretrieveretryinterval += 1
                        }
                        "registersyncrequest" => pg_wait_type_timeout.registersyncrequest += 1,
                        "spindelay" => pg_wait_type_timeout.spindelay += 1,
                        "vacuumdelay" => pg_wait_type_timeout.vacuumdelay += 1,
                        "vacuumtruncate" => pg_wait_type_timeout.vacuumtruncate += 1,
                        _ => pg_wait_type_timeout.other += 1,
                    },
                    _ => { /* wait_event_type != timeout */ }
                },
                _ => { /* not active */ }
            }
        }
        DATA.wait_event_timeout
            .write()
            .await
            .push_back((*timestamp, pg_wait_type_timeout));
    }
}
