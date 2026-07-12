use std::{collections::HashMap, sync::Mutex};

pub trait CredentialStore: Send + Sync {
    fn set(&self, credential_ref: &str, secret: &str) -> Result<(), String>;
    fn get(&self, credential_ref: &str) -> Result<Option<String>, String>;
    fn delete(&self, credential_ref: &str) -> Result<(), String>;
}

#[derive(Default)]
pub struct CredentialMutationCoordinator(Mutex<()>);

impl CredentialMutationCoordinator {
    pub fn acquire(&self) -> Result<std::sync::MutexGuard<'_, ()>, String> {
        self.0.lock().map_err(|error| error.to_string())
    }
}

pub struct WindowsCredentialStore;

impl CredentialStore for WindowsCredentialStore {
    fn set(&self, credential_ref: &str, secret: &str) -> Result<(), String> {
        let entry = keyring::Entry::new("com.bananabox.app", credential_ref)
            .map_err(|error| error.to_string())?;
        entry
            .set_password(secret)
            .map_err(|error| error.to_string())
    }

    fn get(&self, credential_ref: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new("com.bananabox.app", credential_ref)
            .map_err(|error| error.to_string())?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn delete(&self, credential_ref: &str) -> Result<(), String> {
        let entry = keyring::Entry::new("com.bananabox.app", credential_ref)
            .map_err(|error| error.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[derive(Default)]
pub struct MemoryCredentialStore(Mutex<HashMap<String, String>>);

impl CredentialStore for MemoryCredentialStore {
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|error| error.to_string())?
            .insert(key.into(), value.into());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self
            .0
            .lock()
            .map_err(|error| error.to_string())?
            .get(key)
            .cloned())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|error| error.to_string())?
            .remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CredentialMutationCoordinator, CredentialStore, MemoryCredentialStore};
    use std::{
        sync::{mpsc, Arc, TryLockError},
        thread,
        time::Duration,
    };

    #[test]
    fn memory_store_returns_none_for_a_missing_credential() {
        let store = MemoryCredentialStore::default();

        assert_eq!(store.get("missing").unwrap(), None);
    }

    #[test]
    fn memory_store_sets_and_reads_a_credential() {
        let store = MemoryCredentialStore::default();

        store.set("storyboard", "secret-value").unwrap();

        assert_eq!(
            store.get("storyboard").unwrap(),
            Some("secret-value".into())
        );
    }

    #[test]
    fn memory_store_delete_is_idempotent() {
        let store = MemoryCredentialStore::default();
        store.set("storyboard", "secret-value").unwrap();

        store.delete("storyboard").unwrap();
        assert_eq!(store.get("storyboard").unwrap(), None);
        assert!(store.delete("storyboard").is_ok());
    }

    #[test]
    fn credential_mutation_coordinator_releases_when_its_guard_drops() {
        let coordinator = CredentialMutationCoordinator::default();

        {
            let _guard = coordinator.acquire().unwrap();
        }

        assert!(coordinator.acquire().is_ok());
    }

    #[test]
    fn credential_mutation_coordinator_unblocks_another_thread_after_release() {
        let coordinator = Arc::new(CredentialMutationCoordinator::default());
        let held_guard = coordinator.acquire().unwrap();
        let (waiting_tx, waiting_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let worker_coordinator = Arc::clone(&coordinator);

        let worker = thread::spawn(move || {
            waiting_tx.send(()).unwrap();
            let _guard = worker_coordinator.acquire().unwrap();
            acquired_tx.send(()).unwrap();
        });

        waiting_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(matches!(
            coordinator.0.try_lock(),
            Err(TryLockError::WouldBlock)
        ));

        drop(held_guard);

        acquired_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.join().unwrap();
    }
}
