//! The one encrypted local database (`retinacare_persistence::SqlCipherStore`) shared by the patient
//! and audit services. The store is locked until the user supplies the database password; there is no
//! plaintext fallback and no in-memory fixture fallback. Key derivation (Argon2id) runs on a worker
//! thread so the UI never freezes.

use super::patient::PatientServiceError;
use retinacare_persistence::{SqlCipherStore, StoreError};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

pub const MIN_PASSWORD_CHARS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlockStatus {
    Locked,
    /// Deriving the key / opening the database on the worker thread.
    Working,
    Failed(String),
    Open,
}

struct Inner {
    db_path: PathBuf,
    store: Mutex<Option<SqlCipherStore>>,
    status: Mutex<UnlockStatus>,
}

/// Cheap to clone; every clone refers to the same open database.
#[derive(Clone)]
pub struct SharedStore {
    inner: Arc<Inner>,
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(std::env::temp_dir)
        .join("RetinaCare")
}

pub(crate) fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("RetinaCare")
}

/// Copy a run's original image into app data (`exam_images/<sha256>.<ext>`) so a saved exam keeps
/// its picture after the source file moves. Returns the stored path; the source path on any failure.
pub fn keep_exam_image(src: &Path) -> PathBuf {
    let Ok(bytes) = std::fs::read(src) else {
        return src.to_path_buf();
    };
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let dir = if cfg!(test) {
        std::env::temp_dir().join("rc-test-exam-images")
    } else {
        data_dir().join("exam_images")
    };
    let dest = dir.join(format!("{}.{ext}", crate::security::compute_sha256(&bytes)));
    if dest.exists()
        || (std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&dest, &bytes).is_ok())
    {
        dest
    } else {
        src.to_path_buf()
    }
}

/// Delete copied exam images no examination references. Returns how many were removed. An image
/// shared by several exams stays while any of them still points at it.
pub fn prune_exam_images(dir: &Path, referenced: &std::collections::HashSet<PathBuf>) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && !referenced.contains(p))
        .filter(|p| std::fs::remove_file(p).is_ok())
        .count()
}

/// Where `keep_exam_image` stores copies.
pub fn exam_images_dir() -> PathBuf {
    data_dir().join("exam_images")
}

const POINTER_FILE: &str = "location.json";

/// Pure choice, unit-tested. `explicit` (an environment override) always wins. Otherwise the database
/// the app used before is reused when it still exists, then the default location. Only the PATH is
/// remembered: never the password, never the key. The current working directory plays no part.
pub fn choose_db_path(explicit: Option<PathBuf>, config: &Path, data: &Path) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    let default = data.join("database/retinacare.sqlite");
    let remembered = std::fs::read_to_string(config.join(POINTER_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|v| v["database_path"].as_str().map(PathBuf::from))
        .filter(|p| p.is_absolute() && p.is_file());
    remembered.unwrap_or(default)
}

/// `$RETINACARE_DB_PATH`, else the remembered database, else
/// `<user data dir>/RetinaCare/database/retinacare.sqlite` (the same location the backend uses).
pub fn default_db_path() -> PathBuf {
    choose_db_path(
        std::env::var_os("RETINACARE_DB_PATH").map(PathBuf::from),
        &config_dir(),
        &data_dir(),
    )
}

/// Write the location pointer (mode 0600) once a database exists there. Best effort.
pub fn remember_db_path(path: &Path) {
    // Never from tests or from an explicit override: those paths are temporary by nature.
    if cfg!(test) || std::env::var_os("RETINACARE_DB_PATH").is_some() {
        return;
    }
    remember_in(&config_dir(), path);
}

fn remember_in(config: &Path, path: &Path) {
    let Ok(absolute) = std::fs::canonicalize(path) else {
        return;
    };
    let _ = std::fs::create_dir_all(config);
    let file = config.join(POINTER_FILE);
    let body = serde_json::json!({"database_path": absolute.to_string_lossy()}).to_string();
    if std::fs::read_to_string(&file).ok().as_deref() == Some(body.as_str()) {
        return;
    }
    if std::fs::write(&file, body).is_ok() {
        make_private(&file, 0o600);
        make_private(config, 0o700);
    }
}

#[cfg(unix)]
fn make_private(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn make_private(_path: &Path, _mode: u32) {}

/// The database folder and the files in it are readable by this user only.
pub fn secure_db_files(db: &Path) {
    if let Some(dir) = db.parent() {
        make_private(dir, 0o700);
    }
    for file in [db.to_path_buf(), db.with_extension("salt")] {
        if file.is_file() {
            make_private(&file, 0o600);
        }
    }
}

pub fn map_store_error(error: StoreError) -> PatientServiceError {
    match error {
        StoreError::WrongPassword => PatientServiceError::DatabaseLocked,
        StoreError::InvalidInput => PatientServiceError::InvalidInput(
            "a value is empty, too long, or refers to a record that does not exist".to_string(),
        ),
        StoreError::Sqlite(rusqlite_error) => {
            let text = rusqlite_error.to_string();
            if text.contains("UNIQUE") {
                PatientServiceError::InvalidInput("that patient code already exists".to_string())
            } else {
                PatientServiceError::Storage(text)
            }
        }
        other => PatientServiceError::Storage(other.to_string()),
    }
}

impl SharedStore {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(Inner {
                db_path: db_path.into(),
                store: Mutex::new(None),
                status: Mutex::new(UnlockStatus::Locked),
            }),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.inner.db_path
    }

    /// First use creates the database; there is no password recovery.
    pub fn database_exists(&self) -> bool {
        self.inner.db_path.exists()
    }

    pub fn status(&self) -> UnlockStatus {
        self.inner
            .status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(UnlockStatus::Locked)
    }

    pub fn is_open(&self) -> bool {
        self.status() == UnlockStatus::Open
    }

    fn set_status(&self, status: UnlockStatus) {
        if let Ok(mut guard) = self.inner.status.lock() {
            *guard = status;
        }
    }

    /// Start opening (or creating) the database on a worker thread. Returns immediately.
    /// `confirm` must equal `password` when the database does not exist yet.
    pub fn begin_unlock(&self, password: String, confirm: Option<String>) {
        if matches!(self.status(), UnlockStatus::Working | UnlockStatus::Open) {
            return;
        }
        let creating = !self.database_exists();
        if password.chars().count() < MIN_PASSWORD_CHARS && creating {
            self.set_status(UnlockStatus::Failed(format!(
                "Choose a password of at least {MIN_PASSWORD_CHARS} characters. It cannot be recovered if lost."
            )));
            return;
        }
        if creating && confirm.as_deref() != Some(password.as_str()) {
            self.set_status(UnlockStatus::Failed(
                "The two passwords do not match.".to_string(),
            ));
            return;
        }
        self.set_status(UnlockStatus::Working);
        let this = self.clone();
        let password = zeroize::Zeroizing::new(password);
        thread::spawn(move || {
            let result = SqlCipherStore::open(&this.inner.db_path, &password);
            drop(password);
            match result {
                Ok(store) => {
                    secure_db_files(&this.inner.db_path);
                    remember_db_path(&this.inner.db_path);
                    if let Ok(mut guard) = this.inner.store.lock() {
                        *guard = Some(store);
                    }
                    this.set_status(UnlockStatus::Open);
                }
                Err(StoreError::WrongPassword) => this.set_status(UnlockStatus::Failed(
                    "Wrong password. The database was not opened.".to_string(),
                )),
                Err(other) => this.set_status(UnlockStatus::Failed(format!(
                    "The database could not be opened: {other}"
                ))),
            }
        });
    }

    /// Unlock without a prompt from `$RETINACARE_DB_PASSWORD` or the owner-only file
    /// `<config>/RetinaCare/db.key`. No source means it stays locked and the Patients page asks.
    /// shortcut: key sits unencrypted on disk (mode 0600), upgrade to the OS keyring.
    pub fn auto_unlock(&self) {
        if cfg!(test) {
            return;
        }
        let from_env = std::env::var("RETINACARE_DB_PASSWORD").ok();
        let password = from_env.or_else(|| {
            let file = config_dir().join("db.key");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&file).ok()?.permissions().mode();
                if mode & 0o077 != 0 {
                    return None;
                }
            }
            std::fs::read_to_string(file).ok()
        });
        if let Some(password) = password
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
        {
            self.begin_unlock(password.clone(), Some(password));
        }
    }

    /// Run `f` against the open store. Locked (or still opening) is an explicit error, never a fallback.
    pub fn with<R>(
        &self,
        f: impl FnOnce(&SqlCipherStore) -> Result<R, StoreError>,
    ) -> Result<R, PatientServiceError> {
        let guard = self
            .inner
            .store
            .lock()
            .map_err(|_| PatientServiceError::Storage("database lock poisoned".to_string()))?;
        match guard.as_ref() {
            Some(store) => f(store).map_err(map_store_error),
            None => Err(PatientServiceError::DatabaseLocked),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn temp_db(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rc-store-{name}-{}-{}/db.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn wait_until_settled(store: &SharedStore) -> UnlockStatus {
        let start = Instant::now();
        loop {
            let status = store.status();
            if status != UnlockStatus::Working || start.elapsed() > Duration::from_secs(60) {
                return status;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "rc-loc-{name}-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    // D01/D02: first launch uses the stable data folder; a later launch finds the same file whatever the
    // working directory is; nothing here ever reads the working directory.
    #[test]
    fn location_is_stable_remembered_and_independent_of_the_working_directory() {
        let (config, data) = (tmp("cfg"), tmp("data"));
        let default = data.join("database/retinacare.sqlite");
        assert_eq!(
            choose_db_path(None, &config, &data),
            default,
            "first run: default location"
        );
        // The database is created somewhere else (for example moved by the user) and remembered.
        let moved = tmp("moved").join("my.sqlite");
        std::fs::write(&moved, b"x").unwrap();
        remember_in(&config, &moved);
        assert_eq!(
            choose_db_path(None, &config, &data),
            std::fs::canonicalize(&moved).unwrap()
        );
        let before = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp("elsewhere")).unwrap();
        let again = choose_db_path(None, &config, &data);
        std::env::set_current_dir(before).unwrap();
        assert_eq!(
            again,
            std::fs::canonicalize(&moved).unwrap(),
            "same database from another directory"
        );
        // A stale pointer (file gone) falls back to the default instead of inventing a new location.
        std::fs::remove_file(&moved).unwrap();
        assert_eq!(choose_db_path(None, &config, &data), default);
        // An explicit override always wins.
        assert_eq!(
            choose_db_path(Some(PathBuf::from("/x/y.sqlite")), &config, &data),
            PathBuf::from("/x/y.sqlite")
        );
        // A relative pointer is ignored (it would depend on the working directory).
        std::fs::write(
            config.join(POINTER_FILE),
            r#"{"database_path":"rel/db.sqlite"}"#,
        )
        .unwrap();
        assert_eq!(choose_db_path(None, &config, &data), default);
    }

    #[cfg(unix)]
    #[test]
    fn database_files_and_the_location_pointer_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let path = temp_db("perm");
        let store = SharedStore::new(&path);
        store.begin_unlock("correct horse".into(), Some("correct horse".into()));
        assert_eq!(wait_until_settled(&store), UnlockStatus::Open);
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(&path.with_extension("salt")), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);
        let config = tmp("cfg2");
        remember_in(&config, &path);
        assert_eq!(mode(&config.join(POINTER_FILE)), 0o600);
        assert_eq!(mode(&config), 0o700);
    }

    #[test]
    fn locked_store_refuses_every_operation_without_fallback() {
        let store = SharedStore::new(temp_db("locked"));
        assert_eq!(store.status(), UnlockStatus::Locked);
        assert!(matches!(
            store.with(|s| s.list_patients()),
            Err(PatientServiceError::DatabaseLocked)
        ));
    }

    #[test]
    fn first_use_validates_the_password_before_creating_anything() {
        let path = temp_db("first");
        let store = SharedStore::new(&path);
        store.begin_unlock("short".into(), Some("short".into()));
        assert!(matches!(store.status(), UnlockStatus::Failed(m) if m.contains("at least")));
        store.begin_unlock("long enough pw".into(), Some("different pw".into()));
        assert!(matches!(store.status(), UnlockStatus::Failed(m) if m.contains("do not match")));
        assert!(!path.exists(), "nothing is created for a rejected password");
    }

    #[test]
    fn create_persist_reopen_and_reject_wrong_password() {
        let path = temp_db("reopen");
        let store = SharedStore::new(&path);
        store.begin_unlock("correct horse".into(), Some("correct horse".into()));
        assert_eq!(wait_until_settled(&store), UnlockStatus::Open);
        let id = store.with(|s| s.add_patient("P-1", "Real Name")).unwrap();
        drop(store);

        let reopened = SharedStore::new(&path);
        assert!(reopened.database_exists());
        reopened.begin_unlock("wrong password".into(), None);
        assert!(
            matches!(wait_until_settled(&reopened), UnlockStatus::Failed(m) if m.contains("Wrong password"))
        );
        assert!(matches!(
            reopened.with(|s| s.list_patients()),
            Err(PatientServiceError::DatabaseLocked)
        ));
        reopened.begin_unlock("correct horse".into(), None);
        assert_eq!(wait_until_settled(&reopened), UnlockStatus::Open);
        let patients = reopened.with(|s| s.list_patients()).unwrap();
        assert_eq!(patients.len(), 1);
        assert_eq!(patients[0].id, id);
        // The file on disk is ciphertext: it must not contain the patient name in the clear.
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(9).any(|w| w == b"Real Name"));
    }

    #[test]
    fn duplicate_patient_code_is_a_readable_error() {
        let path = temp_db("dup");
        let store = SharedStore::new(&path);
        store.begin_unlock("correct horse".into(), Some("correct horse".into()));
        assert_eq!(wait_until_settled(&store), UnlockStatus::Open);
        store.with(|s| s.add_patient("P-1", "A")).unwrap();
        let err = store.with(|s| s.add_patient("P-1", "B")).unwrap_err();
        assert!(
            matches!(err, PatientServiceError::InvalidInput(m) if m.contains("already exists"))
        );
    }
}

#[cfg(test)]
mod prune_tests {
    use super::*;

    #[test]
    fn prune_keeps_referenced_images_and_removes_orphans() {
        let dir = std::env::temp_dir().join(format!("rc-prune-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (kept, orphan) = (dir.join("a.png"), dir.join("b.png"));
        std::fs::write(&kept, b"a").unwrap();
        std::fs::write(&orphan, b"b").unwrap();
        let refs = std::collections::HashSet::from([kept.clone()]);
        assert_eq!(prune_exam_images(&dir, &refs), 1);
        assert!(kept.exists() && !orphan.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
