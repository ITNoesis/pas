use crate::processor::PgStatActivity;
use crate::DATA;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

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
    pub async fn process(
        current_timestamp: DateTime<Local>,
        pg_stat_activity: Vec<PgStatActivity>,
    ) {
        let pg_wait_types = PgWaitTypes::new().await;

        for element in pg_stat_activity {
            match element.state.as_deref().unwrap_or("") {
                "active" => match element.wait_event_type {
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
            .push_back((current_timestamp, pg_wait_types));
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
    pub async fn new() -> Self {
        PgWaitTypeActivity::default()
    }
    pub async fn process(
        current_timestamp: DateTime<Local>,
        pg_stat_activity: Vec<PgStatActivity>,
    ) {
        let mut pg_wait_type_activity = PgWaitTypeActivity::new().await;

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
            .push_back((current_timestamp, pg_wait_type_activity));
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
    pub async fn process(
        current_timestamp: DateTime<Local>,
        pg_stat_activity: Vec<PgStatActivity>,
    ) {
        let mut pg_wait_type_bufferpin = PgWaitTypeBufferPin::new().await;
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
            .push_back((current_timestamp, pg_wait_type_bufferpin));
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
    pub async fn process(
        current_timestamp: DateTime<Local>,
        pg_stat_activity: Vec<PgStatActivity>,
    ) {
        let mut pg_wait_type_client = PgWaitTypeClient::new().await;
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
            .push_back((current_timestamp, pg_wait_type_client));
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
