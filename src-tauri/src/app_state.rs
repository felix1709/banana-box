use crate::{db::Database, provider_http::ProviderHttpClient, providers::ProviderService};
use std::sync::{Arc, Condvar, Mutex, RwLock};

const OPERATION_GATE_UNAVAILABLE: &str = "OPERATION_GATE_UNAVAILABLE";
const RESTORE_BLOCKER_REGISTRY_UNAVAILABLE: &str = "RESTORE_BLOCKER_REGISTRY_UNAVAILABLE";
const STARTUP_STATUS_UNAVAILABLE: &str = "STARTUP_STATUS_UNAVAILABLE";

struct OperationState {
    maintenance_pending: bool,
    active_operations: usize,
}

pub struct AppOperationGate {
    state: Mutex<OperationState>,
    idle: Condvar,
}

#[must_use = "hold the permit until the protected operation has finished"]
pub struct OperationPermit {
    gate: Arc<AppOperationGate>,
    released: bool,
}

#[must_use = "hold the lease until maintenance has completed or been sealed for restart"]
pub struct MaintenanceLease {
    gate: Arc<AppOperationGate>,
    sealed_for_restart: bool,
}

impl AppOperationGate {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(OperationState {
                maintenance_pending: false,
                active_operations: 0,
            }),
            idle: Condvar::new(),
        }
    }

    pub fn enter_user(self: &Arc<Self>) -> Result<OperationPermit, String> {
        let mut state = self.lock_state()?;
        if state.maintenance_pending {
            return Err("RESTORE_PENDING".into());
        }

        state.active_operations += 1;
        Ok(OperationPermit {
            gate: self.clone(),
            released: false,
        })
    }

    pub fn try_enter_background(self: &Arc<Self>) -> Option<OperationPermit> {
        let mut state = self.state.lock().ok()?;
        if state.maintenance_pending {
            return None;
        }

        state.active_operations += 1;
        Some(OperationPermit {
            gate: self.clone(),
            released: false,
        })
    }

    pub fn begin_maintenance(self: &Arc<Self>) -> Result<MaintenanceLease, String> {
        let mut state = self.lock_state()?;
        if state.maintenance_pending {
            return Err("MAINTENANCE_ALREADY_ACTIVE".into());
        }

        state.maintenance_pending = true;
        while state.active_operations != 0 {
            state = self
                .idle
                .wait(state)
                .map_err(|_| OPERATION_GATE_UNAVAILABLE.to_string())?;
        }

        Ok(MaintenanceLease {
            gate: self.clone(),
            sealed_for_restart: false,
        })
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, OperationState>, String> {
        self.state
            .lock()
            .map_err(|_| OPERATION_GATE_UNAVAILABLE.to_string())
    }
}

impl Default for AppOperationGate {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationPermit {
    fn release(&mut self) {
        if self.released {
            return;
        }

        if let Ok(mut state) = self.gate.state.lock() {
            state.active_operations = state.active_operations.saturating_sub(1);
            self.gate.idle.notify_all();
        }
        self.released = true;
    }
}

impl Drop for OperationPermit {
    fn drop(&mut self) {
        self.release();
    }
}

impl MaintenanceLease {
    pub fn seal_for_restart(mut self) -> Result<(), String> {
        self.sealed_for_restart = true;
        if !self.gate.lock_state()?.maintenance_pending {
            return Err("MAINTENANCE_NOT_ACTIVE".into());
        }
        Ok(())
    }
}

impl Drop for MaintenanceLease {
    fn drop(&mut self) {
        if self.sealed_for_restart {
            return;
        }

        if let Ok(mut state) = self.gate.state.lock() {
            state.maintenance_pending = false;
            self.gate.idle.notify_all();
        }
    }
}

pub trait RestoreBlocker: Send + Sync {
    fn active_blocker(&self) -> Option<RestoreBlockerInfo>;
}

pub struct RestoreBlockerInfo {
    pub code: &'static str,
    pub message: String,
}

pub struct RestoreBlockerRegistry(RwLock<Vec<Arc<dyn RestoreBlocker>>>);

impl RestoreBlockerRegistry {
    pub fn new() -> Self {
        Self(RwLock::new(Vec::new()))
    }

    pub fn register(&self, participant: Arc<dyn RestoreBlocker>) -> Result<(), String> {
        self.0
            .write()
            .map_err(|_| RESTORE_BLOCKER_REGISTRY_UNAVAILABLE.to_string())?
            .push(participant);
        Ok(())
    }

    pub fn first_active(&self) -> Result<Option<RestoreBlockerInfo>, String> {
        let participants = {
            let participants = self
                .0
                .read()
                .map_err(|_| RESTORE_BLOCKER_REGISTRY_UNAVAILABLE.to_string())?;
            participants.clone()
        };

        Ok(participants
            .into_iter()
            .find_map(|participant| participant.active_blocker()))
    }
}

impl Default for RestoreBlockerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationSummary {
    pub prompts_migrated: usize,
    pub favorites_defaulted: usize,
    pub orders_rebuilt: usize,
    pub backup_path: String,
    pub warnings: Vec<String>,
}

pub struct AppServices {
    pub database: Arc<Database>,
    pub provider_http: Arc<ProviderHttpClient>,
    pub providers: Arc<ProviderService>,
    pub operations: Arc<AppOperationGate>,
}

#[derive(Clone, serde::Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StartupStatus {
    Ready {
        migration_summary: Option<MigrationSummary>,
    },
    Recovery {
        message: String,
        backup_paths: Vec<String>,
    },
}

pub struct StartupGate(RwLock<StartupStatus>);

impl StartupGate {
    pub fn new(status: StartupStatus) -> Self {
        Self(RwLock::new(status))
    }

    pub fn status(&self) -> Result<StartupStatus, String> {
        self.0
            .read()
            .map_err(|_| STARTUP_STATUS_UNAVAILABLE.to_string())
            .map(|status| status.clone())
    }

    pub fn require_ready(&self) -> Result<(), String> {
        match self.status()? {
            StartupStatus::Ready { .. } => Ok(()),
            StartupStatus::Recovery { .. } => Err("STARTUP_NOT_READY".into()),
        }
    }

    pub fn clear_migration_summary(&self) -> Result<(), String> {
        let mut status = self
            .0
            .write()
            .map_err(|_| STARTUP_STATUS_UNAVAILABLE.to_string())?;
        match &mut *status {
            StartupStatus::Ready { migration_summary } => {
                *migration_summary = None;
                Ok(())
            }
            StartupStatus::Recovery { .. } => Err("STARTUP_NOT_READY".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{mpsc, Arc},
        thread,
        time::{Duration, Instant},
    };

    fn active_operations(gate: &AppOperationGate) -> usize {
        gate.state.lock().unwrap().active_operations
    }

    fn wait_until(predicate: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(1);
        while !predicate() {
            assert!(
                Instant::now() < deadline,
                "operation did not reach the expected state"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn user_permits_track_active_operations_through_raii() {
        let gate = Arc::new(AppOperationGate::new());
        assert_eq!(active_operations(&gate), 0);

        let first = gate.enter_user().unwrap();
        assert_eq!(active_operations(&gate), 1);
        {
            let second = gate.enter_user().unwrap();
            assert_eq!(active_operations(&gate), 2);
            drop(second);
        }
        assert_eq!(active_operations(&gate), 1);
        drop(first);
        assert_eq!(active_operations(&gate), 0);
    }

    #[test]
    fn maintenance_blocks_later_work_and_waits_for_existing_permits() {
        let gate = Arc::new(AppOperationGate::new());
        let active_permit = gate.enter_user().unwrap();
        let (lease_tx, lease_rx) = mpsc::channel();
        let maintenance_gate = gate.clone();
        let maintenance = thread::spawn(move || {
            lease_tx
                .send(maintenance_gate.begin_maintenance().unwrap())
                .unwrap();
        });

        wait_until(|| gate.state.lock().unwrap().maintenance_pending);
        assert!(matches!(gate.enter_user(), Err(error) if error == "RESTORE_PENDING"));
        assert!(gate.try_enter_background().is_none());
        assert!(lease_rx.try_recv().is_err());

        drop(active_permit);
        let lease = lease_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        maintenance.join().unwrap();
        drop(lease);
    }

    #[test]
    fn dropping_an_unsealed_maintenance_lease_reopens_the_gate() {
        let gate = Arc::new(AppOperationGate::new());
        let lease = gate.begin_maintenance().unwrap();
        drop(lease);

        let permit = gate.enter_user().unwrap();
        drop(permit);
    }

    #[test]
    fn sealing_maintenance_keeps_the_gate_closed_for_restart() {
        let gate = Arc::new(AppOperationGate::new());
        gate.begin_maintenance()
            .unwrap()
            .seal_for_restart()
            .unwrap();

        assert!(matches!(gate.enter_user(), Err(error) if error == "RESTORE_PENDING"));
        assert!(gate.try_enter_background().is_none());
        assert!(matches!(
            gate.begin_maintenance(),
            Err(error) if error == "MAINTENANCE_ALREADY_ACTIVE"
        ));
    }

    #[test]
    fn restore_maintenance_does_not_wait_on_a_permit_it_does_not_hold() {
        let gate = Arc::new(AppOperationGate::new());
        let lease = gate.begin_maintenance().unwrap();
        assert_eq!(active_operations(&gate), 0);
        drop(lease);
    }

    struct FakeRestoreBlocker;

    impl RestoreBlocker for FakeRestoreBlocker {
        fn active_blocker(&self) -> Option<RestoreBlockerInfo> {
            Some(RestoreBlockerInfo {
                code: "STORYBOARD_REQUEST_ACTIVE",
                message: "An active storyboard request is still running.".into(),
            })
        }
    }

    struct InactiveRestoreBlocker;

    impl RestoreBlocker for InactiveRestoreBlocker {
        fn active_blocker(&self) -> Option<RestoreBlockerInfo> {
            None
        }
    }

    struct ReentrantRestoreBlocker {
        registry: Arc<RestoreBlockerRegistry>,
    }

    impl RestoreBlocker for ReentrantRestoreBlocker {
        fn active_blocker(&self) -> Option<RestoreBlockerInfo> {
            self.registry
                .register(Arc::new(InactiveRestoreBlocker))
                .unwrap();
            None
        }
    }

    #[test]
    fn restore_blocker_registry_reads_the_first_safe_active_blocker() {
        let registry = RestoreBlockerRegistry::default();
        assert!(registry.first_active().unwrap().is_none());

        registry.register(Arc::new(FakeRestoreBlocker)).unwrap();
        let blocker = registry.first_active().unwrap().unwrap();
        assert_eq!(blocker.code, "STORYBOARD_REQUEST_ACTIVE");
        assert_eq!(
            blocker.message,
            "An active storyboard request is still running."
        );
    }

    #[test]
    fn restore_blocker_registry_releases_its_lock_before_calling_participants() {
        let registry = Arc::new(RestoreBlockerRegistry::default());
        registry
            .register(Arc::new(ReentrantRestoreBlocker {
                registry: registry.clone(),
            }))
            .unwrap();
        let registry_for_check = registry.clone();
        let (result_tx, result_rx) = mpsc::channel();

        thread::spawn(move || {
            result_tx.send(registry_for_check.first_active()).unwrap();
        });

        assert!(result_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("participant callback should not self-deadlock")
            .unwrap()
            .is_none());
        assert_eq!(registry.0.read().unwrap().len(), 2);
    }

    fn migration_summary() -> MigrationSummary {
        MigrationSummary {
            prompts_migrated: 3,
            favorites_defaulted: 1,
            orders_rebuilt: 2,
            backup_path: "C:\\temp\\library-v0.json".into(),
            warnings: vec!["Invalid record skipped".into()],
        }
    }

    #[test]
    fn startup_gate_allows_ready_and_clears_the_migration_summary() {
        let gate = StartupGate::new(StartupStatus::Ready {
            migration_summary: Some(migration_summary()),
        });

        gate.require_ready().unwrap();
        gate.clear_migration_summary().unwrap();
        assert!(matches!(
            gate.status().unwrap(),
            StartupStatus::Ready {
                migration_summary: None
            }
        ));
    }

    #[test]
    fn startup_gate_keeps_recovery_mode_closed_to_business_commands() {
        let gate = StartupGate::new(StartupStatus::Recovery {
            message: "Recovery in progress".into(),
            backup_paths: vec!["C:\\temp\\library-v0.json".into()],
        });

        assert_eq!(gate.require_ready().unwrap_err(), "STARTUP_NOT_READY");
        assert_eq!(
            gate.clear_migration_summary().unwrap_err(),
            "STARTUP_NOT_READY"
        );
        assert!(matches!(
            gate.status().unwrap(),
            StartupStatus::Recovery { .. }
        ));
    }

    #[test]
    fn recovery_status_serializes_backup_paths_for_the_frontend_contract() {
        let status = StartupStatus::Recovery {
            message: "Recovery in progress".into(),
            backup_paths: vec!["C:\\temp\\library-v0.json".into()],
        };

        assert_eq!(
            serde_json::to_value(status).unwrap(),
            serde_json::json!({
                "state": "recovery",
                "message": "Recovery in progress",
                "backupPaths": ["C:\\temp\\library-v0.json"],
            })
        );
    }
}
