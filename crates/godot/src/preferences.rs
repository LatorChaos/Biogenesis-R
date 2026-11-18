// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{
    HardwareStartup, PreferenceDisposition, PreferenceError, SettingKey, SettingsState,
    read_preferences_file,
};
use godot::classes::Os;
use std::{fs, io, path::{Path, PathBuf}};

const NATIVE_FILE: &str = "biogenesis-r.preferences.xml";

#[path = "preferences_renderer.rs"]
mod renderer;

#[derive(Default)]
pub(super) struct PreferenceSession {
    initialized: bool,
    path: Option<PathBuf>,
    status: String,
    notice: String,
    error: String,
    saved: bool,
    replacement_committed: bool,
    default_language: String,
    reset_backup_available: bool,
    durability_uncertain: bool,
}

fn packet(session: &PreferenceSession) -> VarDictionary {
    let mut output = VarDictionary::new();
    output.set("ok", session.error.is_empty());
    output.set("initialized", session.initialized);
    output.set("status", session.status.as_str());
    output.set("notice", session.notice.as_str());
    output.set("error", session.error.as_str());
    output.set("saved", session.saved);
    output.set("replacement_committed", session.replacement_committed);
    output.set("reset_backup_available", session.reset_backup_available);
    output.set("durability_uncertain", session.durability_uncertain);
    output
}

fn window_geometry(settings: &VarDictionary) -> PackedInt32Array {
    let fields = settings.get("values").expect("Settings snapshot has values").to::<VarDictionary>();
    let values = [SettingKey::WindowX, SettingKey::WindowY, SettingKey::WindowWidth,
        SettingKey::WindowHeight, SettingKey::WindowState].map(|key| {
            fields.get(key.source_name()).expect("Window geometry has integer owners").to::<i32>()
        });
    PackedInt32Array::from(values.as_slice())
}

#[godot_api(secondary)]
impl BgrSimulation {
    #[func]
    fn preferences_initialize(&mut self) -> VarDictionary {
        if self.preferences.initialized {
            return self.preferences_snapshot();
        }
        if self.settings_applying || self.world.is_some() || self.detached_session.is_some() {
            let mut output = packet(&self.preferences);
            output.set("ok", false);
            output.set("error", "Initialize preferences before starting a session");
            return output;
        }
        let os = Os::singleton();
        let directory = PathBuf::from(os.get_user_data_dir().to_string());
        let language = os.get_locale_language().to_string();
        let language = if language.is_empty() { "en".to_owned() } else { language };
        self.preferences.initialized = true;
        self.preferences.default_language = language.clone();
        if !directory.is_absolute() {
            self.settings = Some(SettingsState::new(language));
            self.preferences.status = "unavailable".to_owned();
            self.preferences.error = "Godot did not provide an absolute owned user-data directory".to_owned();
            return self.preferences_snapshot();
        }
        let path = directory.join(NATIVE_FILE);
        self.preferences.path = Some(path.clone());
        self.preferences.reset_backup_available = reset_backup_available(&path);
        let state = self.load_native_preferences();
        self.settings = Some(state);
        self.preferences_snapshot()
    }

    #[func]
    fn preferences_reset(&mut self) -> VarDictionary {
        self.preference_action_output(PreferenceAction::Reset)
    }

    #[func]
    fn preferences_restore(&mut self) -> VarDictionary {
        self.preference_action_output(PreferenceAction::Restore)
    }

    #[func]
    fn preferences_snapshot(&self) -> VarDictionary {
        let mut output = packet(&self.preferences);
        let settings = self.settings_snapshot();
        output.set("geometry", &window_geometry(&settings));
        output.set("settings", &settings);
        output.set("world_ready", self.world.is_some());
        output.set("session_ready", self.world.is_some() || self.detached_session.is_some());
        output
    }

    #[func]
    fn preferences_import(&mut self, path: GString) -> VarDictionary {
        match self.import_preferences_path(Path::new(&path.to_string())) {
            Ok(hardware) => {
                let mut output = packet(&self.preferences);
                output.set("applied", true);
                let mut startup = VarDictionary::new();
                startup.set("requested", hardware.requested);
                startup.set("effective", hardware.effective);
                startup.set("persisted_override", &hardware.persisted_override
                    .map_or_else(Variant::nil, |value| value.to_variant()));
                output.set("hardware", &startup);
                output
            }
            Err(error) => {
                // Rejected imports must not poison the existing native preference owner
                let mut output = packet(&self.preferences);
                output.set("ok", false);
                output.set("applied", false);
                output.set("saved", false);
                output.set("replacement_committed", false);
                output.set("durability_uncertain", false);
                output.set("error", error.as_str());
                output
            }
        }
    }

    #[func]
    fn preferences_save(&mut self, geometry: PackedInt32Array) -> VarDictionary {
        if self.settings_applying {
            let mut output = packet(&self.preferences);
            output.set("ok", false);
            output.set("saved", false);
            output.set("error", "Settings are being applied; preference saving is busy");
            return output;
        }
        if !self.preferences.initialized {
            let mut output = packet(&self.preferences);
            output.set("ok", true);
            output.set("skipped", true);
            output.set("saved", false);
            return output;
        }
        self.preferences.saved = false;
        self.preferences.replacement_committed = false;
        self.preferences.durability_uncertain = false;
        let error = if self.preferences.path.is_none() {
            Some("An owned native preference path is unavailable".to_owned())
        } else if geometry.len() != 5 {
            Some("Exactly five window geometry integers are required".to_owned())
        } else {
            let state = self.settings.as_mut().expect("Initialization installs the settings owner");
            let values = std::array::from_fn(|index| geometry[index]);
            match state.record_window_geometry(values) {
                Err(error) => Some(format!("Invalid window geometry: {error:?}")),
                Ok(()) => {
                    let path = self.preferences.path.as_ref().expect("Initialization fixes the owned path");
                    match state.save_preferences(path) {
                        Ok(()) => None,
                        Err(error) => {
                            if let PreferenceError::Replace(replace) = &error {
                                self.preferences.replacement_committed = replace.replacement_committed;
                                self.preferences.durability_uncertain = replace.replacement_committed;
                            }
                            Some(error.to_string())
                        }
                    }
                }
            }
        };
        self.preferences.error = error.unwrap_or_default();
        self.preferences.saved = self.preferences.error.is_empty();
        packet(&self.preferences)
    }

    #[func]
    fn set_crisp_lines(&mut self, enabled: bool) -> VarDictionary {
        let mut output = VarDictionary::new();
        if self.settings_applying {
            output.set("ok", false);
            output.set("error", "Settings are being applied; display choice is busy");
            return output;
        }
        self.settings.get_or_insert_with(|| SettingsState::new("en".to_owned())).set_crisp_lines(enabled);
        output.set("ok", true);
        output.set("error", "");
        output.set("crisp_lines", enabled);
        output
    }
}

fn reject_native_alias(source: &Path, native: &Path) -> Result<(), String> {
    let alias = "Select a legacy prefs.xml file, not the native preference file or an alias";
    if source == native {
        return Err(alias.to_owned());
    }
    let selected = fs::canonicalize(source)
        .map_err(|error| format!("Cannot resolve legacy preferences {}: {error}", source.display()))?;
    let metadata = match fs::metadata(native) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("Cannot inspect native preferences: {error}")),
    };
    let destination = fs::canonicalize(native)
        .map_err(|error| format!("Cannot resolve native preferences: {error}"))?;
    if selected == destination {
        return Err(alias.to_owned());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let selected = fs::metadata(source)
            .map_err(|error| format!("Cannot inspect legacy preferences: {error}"))?;
        if selected.dev() == metadata.dev() && selected.ino() == metadata.ino() {
            return Err(alias.to_owned());
        }
    }
    #[cfg(not(unix))]
    let _ = metadata;
    Ok(())
}

impl BgrSimulation {
    fn import_preferences_path(&mut self, source: &Path) -> Result<HardwareStartup, String> {
        if self.settings_applying {
            return Err("Settings are being applied; preference import is busy".to_owned());
        }
        if !self.preferences.initialized || self.preferences.path.is_none() {
            return Err("Initialize owned native preferences before importing".to_owned());
        }
        if self.world.is_some() {
            return Err("Stop the simulation before importing legacy preferences".to_owned());
        }
        if !source.is_absolute() {
            return Err("Select an absolute legacy preference file path".to_owned());
        }
        let native = self.preferences.path.as_ref().expect("Checked native path");
        reject_native_alias(source, native)?;
        let map = read_preferences_file(source).map_err(|error| error.to_string())?
            .ok_or_else(|| "The selected legacy preference file is missing".to_owned())?;
        let loaded = SettingsState::from_preferences(&map, self.preferences.default_language.clone());
        if let PreferenceDisposition::DefaultsForVersion { stored } = loaded.disposition {
            return Err(format!("Legacy preferences require VERSION 700; found {stored:?}"));
        }
        let hardware = loaded.hardware;
        let mut candidate = loaded.state;
        let current = self.settings.as_ref().expect("Initialization installs the settings owner");
        candidate.set_crisp_lines(current.crisp_lines());
        candidate.set_automatic_backup_mode(current.automatic_backup_mode());
        let error = match candidate.save_preferences(native) {
            Ok(()) => None,
            Err(error) => {
                let committed = matches!(&error, PreferenceError::Replace(error) if error.replacement_committed);
                if !committed {
                    return Err(error.to_string());
                }
                Some(error.to_string())
            }
        };
        // A committed replacement must publish the same settings even if directory sync failed
        self.apply_runtime_settings(&candidate);
        self.settings = Some(candidate);
        self.preferences.status = "imported".to_owned();
        self.preferences.notice = "Legacy settings imported. The original file is unchanged; native Crisp lines and recovery policy were retained.".to_owned();
        if hardware.persisted_override.is_some() {
            self.preferences.notice.push_str(" Legacy graphics startup requests remain metadata; the rendering backend was not changed.");
        }
        self.preferences.saved = error.is_none();
        self.preferences.durability_uncertain = error.is_some();
        self.preferences.replacement_committed = true;
        self.preferences.error = error.unwrap_or_default();
        Ok(hardware)
    }
}


#[derive(Clone, Copy)]
enum PreferenceAction {
    Reset,
    Restore,
}

struct PreferenceMoveError {
    message: String,
    committed: bool,
}

fn reset_backup_path(native: &Path) -> PathBuf {
    let mut name = native.file_name().expect("Native preferences have a fixed filename").to_os_string();
    name.push(".before-reset");
    native.with_file_name(name)
}

fn reset_backup_available(native: &Path) -> bool {
    // Availability is advisory; each action validates its files again
    fs::symlink_metadata(reset_backup_path(native)).is_ok_and(|metadata| metadata.is_file())
}

fn preference_move_error(operation: &str, path: &Path, source: io::Error, committed: bool) -> PreferenceMoveError {
    let mut message = format!("Cannot {operation} {}: {source}", path.display());
    if committed {
        message.push_str("; preference move completed, but disk durability is unconfirmed");
    }
    PreferenceMoveError { message, committed }
}

fn preference_move_metadata(path: &Path) -> Result<Option<fs::Metadata>, PreferenceMoveError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.permissions().readonly() => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() != 1 {
                    return Err(preference_move_error("move preferences at", path,
                        io::Error::new(io::ErrorKind::InvalidInput, "Preference paths must not be hard linked"), false));
                }
            }
            Ok(Some(metadata))
        },
        Ok(_) => Err(preference_move_error("move preferences at", path,
            io::Error::new(io::ErrorKind::PermissionDenied, "Preference paths must be writable regular files"), false)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(preference_move_error("inspect preferences at", path, error, false)),
    }
}

fn move_preferences(native: &Path, action: PreferenceAction) -> Result<bool, PreferenceMoveError> {
    let backup = reset_backup_path(native);
    let (source, destination) = match action {
        PreferenceAction::Reset => (native, backup.as_path()),
        PreferenceAction::Restore => (backup.as_path(), native),
    };
    let source_exists = preference_move_metadata(source)?.is_some();
    let moved = if source_exists {
        preference_move_metadata(destination)?;
        fs::rename(source, destination)
            .map_err(|error| preference_move_error("move preferences from", source, error, false))?;
        true
    } else if matches!(action, PreferenceAction::Reset) {
        false
    } else {
        return Err(preference_move_error("restore preferences from", source,
            io::Error::new(io::ErrorKind::NotFound, "No before-reset preference file exists"), false));
    };
    #[cfg(unix)]
    {
        let parent = native.parent().expect("Owned native preferences have an absolute parent");
        match fs::File::open(parent) {
            Ok(directory) => directory.sync_all()
                .map_err(|error| preference_move_error("synchronize preference directory", parent, error, moved))?,
            Err(error) if !moved && error.kind() == io::ErrorKind::NotFound => {},
            Err(error) => return Err(preference_move_error("open preference directory", parent, error, moved)),
        }
    }
    Ok(moved)
}

impl BgrSimulation {
    pub(super) fn save_network_preferences(&mut self) -> Result<(), String> {
        if !self.preferences.initialized { return Err("Initialize preferences before saving network settings".to_owned()); }
        let path = self.preferences.path.as_ref().ok_or("An owned native preference path is unavailable")?;
        let state = self.settings.as_ref().expect("Initialized preferences have a settings owner");
        self.preferences.saved = false;
        self.preferences.replacement_committed = false;
        self.preferences.durability_uncertain = false;
        match state.save_preferences(path) {
            Ok(()) => {
                self.preferences.saved = true;
                self.preferences.error.clear();
                Ok(())
            }
            Err(error) => {
                if let PreferenceError::Replace(replace) = &error {
                    self.preferences.replacement_committed = replace.replacement_committed;
                    self.preferences.durability_uncertain = replace.replacement_committed;
                }
                self.preferences.error = error.to_string();
                Err(self.preferences.error.clone())
            }
        }
    }

    fn load_native_preferences(&mut self) -> SettingsState {
        self.preferences.notice.clear();
        self.preferences.error.clear();
        self.preferences.durability_uncertain = false;
        self.preferences.status = "missing".to_owned();
        let language = self.preferences.default_language.clone();
        let path = self.preferences.path.as_ref().expect("Initialization fixes the owned path");
        let mut state = SettingsState::new(language.clone());
        match read_preferences_file(&path) {
            Ok(Some(map)) => {
                let has_version = map.contains_key("VERSION");
                let loaded = SettingsState::from_native_preferences(&map, language);
                match loaded.disposition {
                    PreferenceDisposition::Loaded => {
                        self.preferences.status = "loaded".to_owned();
                    }
                    PreferenceDisposition::InvalidAutomaticBackupMode => {
                        self.preferences.status = "unreadable".to_owned();
                        self.preferences.error = "Unsupported native automatic backup mode; original preference file retained".to_owned();
                    }
                    PreferenceDisposition::InvalidUserNameEncoding => {
                        self.preferences.status = "unreadable".to_owned();
                        self.preferences.error = "Invalid native username encoding; original preference file retained".to_owned();
                    }
                    PreferenceDisposition::DefaultsForVersion { stored } => {
                        self.preferences.status = if has_version { "unsupported" } else { "missing_version" }.to_owned();
                        self.preferences.notice = "Preference version did not match. Source defaults were staged; original bytes remain unchanged.".to_owned();
                        if has_version {
                            self.preferences.error = format!("Unsupported native preference version {stored:?}; saving is blocked");
                        }
                    }
                }
                if loaded.hardware.persisted_override.is_some() {
                    self.preferences.notice.push_str(" Legacy graphics startup requests are retained as source metadata; no renderer recovery write was made.");
                }
                state = loaded.state;
            }
            Ok(None) => {}
            Err(error) => {
                self.preferences.status = "unreadable".to_owned();
                self.preferences.error = error.to_string();
                self.preferences.notice = "Source defaults were staged. The existing preference file was not changed.".to_owned();
            }
        }
        state
    }

    fn preference_action_output(&mut self, action: PreferenceAction) -> VarDictionary {
        match self.change_preferences_path(action) {
            Ok(()) => {
                let mut output = self.preferences_snapshot();
                output.set("applied", true);
                output
            }
            Err(error) => {
                let mut output = packet(&self.preferences);
                output.set("ok", false);
                output.set("applied", false);
                output.set("saved", false);
                output.set("replacement_committed", false);
                output.set("durability_uncertain", false);
                output.set("error", error.as_str());
                output
            }
        }
    }

    fn change_preferences_path(&mut self, action: PreferenceAction) -> Result<(), String> {
        if self.settings_applying {
            return Err("Settings are being applied; preference reset or restore is busy".to_owned());
        }
        if !self.preferences.initialized || self.preferences.path.is_none() {
            return Err("Initialize owned native preferences before resetting or restoring".to_owned());
        }
        if self.world.is_some() {
            return Err("Stop the simulation before resetting or restoring preferences".to_owned());
        }
        let native = self.preferences.path.as_ref().expect("Initialization fixes the owned path").clone();
        let (committed, durability_error) = match move_preferences(&native, action) {
            Ok(moved) => (moved, None),
            Err(error) if error.committed => (true, Some(error.message)),
            Err(error) => return Err(error.message),
        };
        let candidate = self.load_native_preferences();
        self.apply_runtime_settings(&candidate);
        self.settings = Some(candidate);
        self.preferences.saved = false;
        self.preferences.replacement_committed = committed;
        self.preferences.reset_backup_available = reset_backup_available(&native);
        self.preferences.durability_uncertain = durability_error.is_some();
        if let Some(error) = durability_error {
            if !self.preferences.error.is_empty() {
                self.preferences.error.push_str("; ");
            }
            self.preferences.error.push_str(&error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use biogenesis_core::{AutomaticBackupMode, SettingValue};
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    const NATIVE: &str = "<map><entry key=\"VERSION\" value=\"700\"/><entry key=\"unknown\" value=\"keep\"/></map>";
    const LEGACY: &str = "<map MAP_XML_VERSION=\"1.0\"><entry key=\"VERSION\" value=\"700\"/><entry key=\"LOCALE\" value=\"PT_BR\"/><entry key=\"WORLD_WIDTH\" value=\"1200\"/><entry key=\"WORLD_HEIGHT\" value=\"800\"/><entry key=\"INITIAL_ORGANISMS\" value=\"0\"/><entry key=\"DELAY\" value=\"-17\"/><entry key=\"MAX_VEL\" value=\"0.125\"/><entry key=\"ORGANISMS_VECTOR_SIZE\" value=\"13\"/><entry key=\"AUTO_BACKUP\" value=\"true\"/><entry key=\"HARDWARE_ACCELERATION\" value=\"1\"/><entry key=\"WINDOW_X\" value=\"22\"/><entry key=\"WINDOW_Y\" value=\"33\"/><entry key=\"WINDOW_WIDTH\" value=\"1000\"/><entry key=\"WINDOW_HEIGHT\" value=\"760\"/><entry key=\"WINDOW_STATE\" value=\"6\"/><entry key=\"BGR_CRISP_LINES\" value=\"false\"/><entry key=\"BGR_AUTOMATIC_BACKUP_MODE\" value=\"invalid-native-policy\"/></map>";

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            for _ in 0..128 {
                let path = std::env::temp_dir().join(format!(
                    "bgr-preference-import-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed),
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Cannot create private directory: {error}"),
                }
            }
            panic!("Private directory names are occupied");
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700)).unwrap();
            }
            fs::remove_dir_all(&self.0).expect("Cannot remove owned test directory");
        }
    }

    fn initialized(native: &Path) -> BgrSimulation {
        let mut owner = crate::tests::session();
        let mut settings = SettingsState::new("en".to_owned());
        settings.set_crisp_lines(true);
        settings.set_automatic_backup_mode(AutomaticBackupMode::Managed);
        owner.settings = Some(settings);
        owner.preferences.initialized = true;
        owner.preferences.path = Some(native.to_owned());
        owner.preferences.default_language = "en".to_owned();
        owner.preferences.status = "loaded".to_owned();
        owner.preferences.notice = "prior notice".to_owned();
        owner
    }

    #[test]
    fn import_retains_detached_session_random_stream_and_native_only_choices() {
        let directory = Directory::new();
        let native = directory.0.join("native.xml");
        let source = directory.0.join("prefs.xml");
        fs::write(&native, NATIVE).unwrap();
        fs::write(&source, LEGACY).unwrap();
        let mut owner = initialized(&native);
        owner.start_session_world(827364, 1200, 800, 32).unwrap();
        owner.release_world_state();
        let retained = owner.detached_session.as_ref().unwrap().session.clone();
        let random = retained.random_snapshot();
        let seed = owner.detached_session.as_ref().unwrap().seed;
        let hardware = owner.import_preferences_path(&source).unwrap();
        assert_eq!((hardware.requested, hardware.effective, hardware.persisted_override), (1, 2, Some(0)));
        assert!(retained.same_owner(&owner.detached_session.as_ref().unwrap().session));
        assert_eq!(retained.random_snapshot(), random);
        assert_eq!(owner.detached_session.as_ref().unwrap().seed, seed);
        assert_eq!(retained.parameters().physics.max_velocity, 0.125);
        let settings = owner.settings.as_ref().unwrap();
        assert_eq!(settings.get(SettingKey::Delay), SettingValue::Int(-17));
        assert_eq!(settings.get(SettingKey::OrganismsVectorSize), SettingValue::Int(13));
        assert_eq!(settings.get(SettingKey::AutoBackup), SettingValue::Boolean(true));
        assert!(settings.crisp_lines());
        assert_eq!(settings.automatic_backup_mode(), AutomaticBackupMode::Managed);
        let map = read_preferences_file(&native).unwrap().unwrap();
        assert_eq!(map.get("unknown").unwrap(), "keep");
        assert_eq!(SettingsState::from_native_preferences(&map, "en".to_owned()).state, *settings);
        assert_eq!(fs::read(&source).unwrap(), LEGACY.as_bytes());
        let before = settings.clone();
        let bytes = fs::read(&native).unwrap();
        assert!(owner.import_preferences_path(&native).is_err());
        fs::create_dir(directory.0.join("child")).unwrap();
        assert!(owner.import_preferences_path(&directory.0.join("child/../native.xml")).is_err());
        #[cfg(unix)]
        {
            let alias = directory.0.join("alias.xml");
            fs::hard_link(&native, &alias).unwrap();
            assert!(owner.import_preferences_path(&alias).is_err());
            let link = directory.0.join("link.xml");
            std::os::unix::fs::symlink(&native, &link).unwrap();
            assert!(owner.import_preferences_path(&link).is_err());
        }
        assert_eq!(owner.settings.as_ref(), Some(&before));
        assert_eq!(fs::read(&native).unwrap(), bytes);
        assert_eq!(retained.random_snapshot(), random);
        fs::write(&source, "<map><entry key=\"VERSION\" value=\"700\"/></map>").unwrap();
        owner.import_preferences_path(&source).unwrap();
        let partial = owner.settings.as_ref().unwrap();
        assert_eq!(partial.get(SettingKey::Locale), SettingValue::Text("en".to_owned()));
        assert_eq!(partial.get(SettingKey::WorldWidth), SettingsState::new("en".to_owned()).get(SettingKey::WorldWidth));
        assert_eq!(partial.get(SettingKey::Delay), SettingValue::Int(2));
        assert!(partial.crisp_lines());
        assert_eq!(partial.automatic_backup_mode(), AutomaticBackupMode::Managed);
        assert_eq!(retained.random_snapshot(), random);
    }

    #[cfg(unix)]
    #[test]
    fn import_distinguishes_precommit_failure_from_committed_durability_failure() {
        use std::os::unix::fs::PermissionsExt;
        let directory = Directory::new();
        let native = directory.0.join("native.xml");
        let source = directory.0.join("prefs.xml");
        fs::write(&native, NATIVE).unwrap();
        fs::write(&source, LEGACY).unwrap();
        let mut owner = initialized(&native);
        owner.start_session_world(827364, 1200, 800, 32).unwrap();
        owner.release_world_state();
        let retained = owner.detached_session.as_ref().unwrap().session.clone();
        let random = retained.random_snapshot();
        let before = owner.settings.clone();
        let parameters = retained.parameters();
        fs::set_permissions(&native, fs::Permissions::from_mode(0o400)).unwrap();
        assert!(owner.import_preferences_path(&source).is_err());
        assert_eq!(owner.settings, before);
        assert_eq!(owner.preferences.status, "loaded");
        assert_eq!(owner.preferences.notice, "prior notice");
        assert!(owner.preferences.error.is_empty());
        assert_eq!(retained.parameters(), parameters);
        assert_eq!(fs::read(&native).unwrap(), NATIVE.as_bytes());
        fs::set_permissions(&native, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o333)).unwrap();
        let applied = owner.import_preferences_path(&source);
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o700)).unwrap();
        applied.unwrap();
        assert!(!owner.preferences.saved);
        assert!(owner.preferences.replacement_committed);
        assert!(owner.preferences.error.contains("replacement completed"));
        let map = read_preferences_file(&native).unwrap().unwrap();
        assert_eq!(SettingsState::from_native_preferences(&map, "en".to_owned()).state, *owner.settings.as_ref().unwrap());
        assert_eq!(retained.parameters().physics.max_velocity, 0.125);
        assert!(retained.same_owner(&owner.detached_session.as_ref().unwrap().session));
        assert_eq!(retained.random_snapshot(), random);
        assert_eq!(fs::read(&source).unwrap(), LEGACY.as_bytes());
    }

    #[test]
    fn reset_and_restore_preserve_exact_bytes_and_detached_random_state() {
        let directory = Directory::new();
        let native = directory.0.join("native.xml");
        let bytes = b"<map><entry key=\"VERSION\" value=\"700\"/><entry key=\"LOCALE\" value=\"ru\"/><entry key=\"DELAY\" value=\"-17\"/><entry key=\"MAX_VEL\" value=\"0.125\"/><entry key=\"BGR_CRISP_LINES\" value=\"true\"/><entry key=\"BGR_AUTOMATIC_BACKUP_MODE\" value=\"managed\"/><entry key=\"unknown\" value=\"keep\"/></map>";
        fs::write(&native, bytes).unwrap();
        let mut owner = initialized(&native);
        owner.settings = Some(owner.load_native_preferences());
        let before = owner.settings.clone();
        owner.start_session_world(827364, 1200, 800, 32).unwrap();
        owner.release_world_state();
        let retained = owner.detached_session.as_ref().unwrap().session.clone();
        let random = retained.random_snapshot();
        let seed = owner.detached_session.as_ref().unwrap().seed;
        owner.change_preferences_path(PreferenceAction::Reset).unwrap();
        assert!(!native.exists());
        assert_eq!(fs::read(reset_backup_path(&native)).unwrap(), bytes);
        assert_eq!(owner.settings, Some(SettingsState::new("en".to_owned())));
        assert!(owner.preferences.reset_backup_available);
        assert!(retained.same_owner(&owner.detached_session.as_ref().unwrap().session));
        assert_eq!(retained.random_snapshot(), random);
        assert_eq!(owner.detached_session.as_ref().unwrap().seed, seed);
        assert_eq!(retained.parameters().physics.max_velocity, 5.0);
        owner.settings.as_ref().unwrap().save_preferences(&native).unwrap();
        owner.change_preferences_path(PreferenceAction::Restore).unwrap();
        assert_eq!(fs::read(&native).unwrap(), bytes);
        assert!(!reset_backup_path(&native).exists());
        assert_eq!(owner.settings, before);
        assert!(!owner.preferences.reset_backup_available);
        assert_eq!(retained.parameters().physics.max_velocity, 0.125);
        assert!(retained.same_owner(&owner.detached_session.as_ref().unwrap().session));
        assert_eq!(retained.random_snapshot(), random);
        assert_eq!(owner.detached_session.as_ref().unwrap().seed, seed);
    }

    #[test]
    fn reset_keeps_one_recovery_copy_and_restores_invalid_bytes_without_hiding_errors() {
        let directory = Directory::new();
        let native = directory.0.join("native.xml");
        let mut owner = initialized(&native);
        owner.change_preferences_path(PreferenceAction::Reset).unwrap();
        assert!(!native.exists() && !reset_backup_path(&native).exists());
        assert!(owner.change_preferences_path(PreferenceAction::Restore).is_err());
        for bytes in [b"<broken".as_slice(), b"<map><entry key=\"VERSION\" value=\"701\"/></map>".as_slice()] {
            fs::write(&native, bytes).unwrap();
            owner.change_preferences_path(PreferenceAction::Reset).unwrap();
            assert!(!native.exists());
            assert_eq!(fs::read(reset_backup_path(&native)).unwrap(), bytes);
            owner.change_preferences_path(PreferenceAction::Reset).unwrap();
            assert_eq!(fs::read(reset_backup_path(&native)).unwrap(), bytes);
            owner.change_preferences_path(PreferenceAction::Restore).unwrap();
            assert_eq!(fs::read(&native).unwrap(), bytes);
            assert!(!owner.preferences.error.is_empty());
            assert!(!owner.preferences.durability_uncertain);
            assert_eq!(owner.settings, Some(SettingsState::new("en".to_owned())));
        }
        owner.change_preferences_path(PreferenceAction::Reset).unwrap();
        fs::write(&native, b"latest").unwrap();
        owner.change_preferences_path(PreferenceAction::Reset).unwrap();
        assert_eq!(fs::read(reset_backup_path(&native)).unwrap(), b"latest");
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn reset_rejects_busy_world_readonly_and_alias_paths_without_mutation() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let directory = Directory::new();
        let native = directory.0.join("native.xml");
        fs::write(&native, NATIVE).unwrap();
        let mut owner = initialized(&native);
        let before = owner.settings.clone();
        owner.preferences.initialized = false;
        assert!(owner.change_preferences_path(PreferenceAction::Reset).is_err());
        owner.preferences.initialized = true;
        owner.settings_applying = true;
        assert!(owner.change_preferences_path(PreferenceAction::Reset).is_err());
        owner.settings_applying = false;
        assert_eq!(owner.settings, before);
        owner.start_session_world(827364, 1200, 800, 0).unwrap();
        let active_settings = owner.settings.clone();
        assert!(owner.change_preferences_path(PreferenceAction::Reset).is_err());
        assert_eq!(owner.settings, active_settings);
        owner.release_world_state();
        let before = owner.settings.clone();
        fs::set_permissions(&native, fs::Permissions::from_mode(0o400)).unwrap();
        assert!(owner.change_preferences_path(PreferenceAction::Reset).is_err());
        fs::set_permissions(&native, fs::Permissions::from_mode(0o600)).unwrap();
        let backup = reset_backup_path(&native);
        symlink(&native, &backup).unwrap();
        assert!(owner.change_preferences_path(PreferenceAction::Reset).is_err());
        fs::remove_file(&backup).unwrap();
        fs::hard_link(&native, &backup).unwrap();
        assert!(owner.change_preferences_path(PreferenceAction::Reset).is_err());
        fs::remove_file(&backup).unwrap();
        let other = directory.0.join("other.xml");
        fs::rename(&native, &other).unwrap();
        symlink(&other, &native).unwrap();
        assert!(owner.change_preferences_path(PreferenceAction::Reset).is_err());
        fs::remove_file(&native).unwrap();
        fs::rename(&other, &native).unwrap();
        assert_eq!(owner.settings, before);
        assert_eq!(fs::read(&native).unwrap(), NATIVE.as_bytes());
        assert!(!backup.exists());
    }

    #[cfg(unix)]
    #[test]
    fn preference_moves_publish_committed_state_and_report_directory_sync_failure() {
        use std::os::unix::fs::PermissionsExt;
        let directory = Directory::new();
        let native = directory.0.join("native.xml");
        fs::write(&native, NATIVE).unwrap();
        let mut owner = initialized(&native);
        let before = owner.settings.clone();
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o555)).unwrap();
        let rejected = owner.change_preferences_path(PreferenceAction::Reset);
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(rejected.is_err());
        assert_eq!(owner.settings, before);
        assert_eq!(fs::read(&native).unwrap(), NATIVE.as_bytes());
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o333)).unwrap();
        let moved = owner.change_preferences_path(PreferenceAction::Reset);
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o700)).unwrap();
        moved.unwrap();
        assert!(owner.preferences.replacement_committed);
        assert!(owner.preferences.error.contains("move completed"));
        assert!(owner.preferences.durability_uncertain);
        assert_eq!(owner.settings, Some(SettingsState::new("en".to_owned())));
        assert!(!native.exists());
        assert_eq!(fs::read(reset_backup_path(&native)).unwrap(), NATIVE.as_bytes());
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o333)).unwrap();
        let restored = owner.change_preferences_path(PreferenceAction::Restore);
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o700)).unwrap();
        restored.unwrap();
        assert!(owner.preferences.replacement_committed);
        assert!(owner.preferences.error.contains("move completed"));
        assert!(owner.preferences.durability_uncertain);
        assert_eq!(fs::read(&native).unwrap(), NATIVE.as_bytes());
        assert!(!reset_backup_path(&native).exists());
    }

}
