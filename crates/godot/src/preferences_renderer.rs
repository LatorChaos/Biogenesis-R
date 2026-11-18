// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{encode_preferences_xml, replace_file_bytes, PreferenceMap};

const RENDERER_KEY: &str = "BGR_RENDERER";
const COMPATIBILITY: &str = "gl_compatibility";
const MOBILE: &str = "mobile";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RendererState {
    #[default]
    Compatibility,
    StartingTrial,
    Mobile,
    StartingConfirmed,
}

impl RendererState {
    fn value(self) -> &'static str {
        match self {
            Self::Compatibility => "compatibility",
            Self::StartingTrial => "mobile-starting-trial",
            Self::Mobile => "mobile",
            Self::StartingConfirmed => "mobile-starting-confirmed",
        }
    }

    fn read(map: &PreferenceMap) -> Result<Self, PreferenceError> {
        match map.get(RENDERER_KEY).map(String::as_str) {
            None | Some("compatibility") => Ok(Self::Compatibility),
            Some("mobile-starting-trial") => Ok(Self::StartingTrial),
            Some("mobile") => Ok(Self::Mobile),
            Some("mobile-starting-confirmed") => Ok(Self::StartingConfirmed),
            _ => Err(PreferenceError::Malformed("Unsupported native renderer state; original preferences retained".into())),
        }
    }

    fn target(self) -> &'static str {
        if self == Self::Compatibility { COMPATIBILITY } else { MOBILE }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RendererAction { Continue, Restart, Confirm }

impl RendererAction {
    fn value(self) -> &'static str {
        match self { Self::Continue => "continue", Self::Restart => "restart", Self::Confirm => "confirm" }
    }
}

fn startup_plan(state: RendererState, actual: &str, safe: bool) -> (RendererState, RendererAction, bool) {
    use RendererAction::*;
    use RendererState::*;
    let interrupted = matches!(state, StartingTrial | StartingConfirmed) && actual != MOBILE;
    if safe {
        return (Compatibility, if actual == COMPATIBILITY { Continue } else { Restart }, interrupted);
    }
    match (state, actual == MOBILE) {
        (StartingTrial, true) => (StartingTrial, Confirm, false),
        (StartingConfirmed, true) => (Mobile, Continue, false),
        (StartingTrial | StartingConfirmed, false) => (Compatibility, Continue, true),
        (Mobile, false) => (StartingConfirmed, Restart, false),
        _ => (state, Continue, false),
    }
}

fn renderer_map(path: &Path) -> Result<PreferenceMap, PreferenceError> {
    let map = read_preferences_file(path)?;
    if let Some(map) = map {
        if !matches!(SettingsState::from_preferences(&map, "en".into()).disposition, PreferenceDisposition::Loaded) {
            return Err(PreferenceError::Malformed("Renderer preferences require VERSION 700; original file retained".into()));
        }
        RendererState::read(&map)?;
        Ok(map)
    } else {
        Ok(PreferenceMap::from([("VERSION".into(), "700".into())]))
    }
}

fn write_renderer(path: &Path, state: RendererState) -> Result<(), PreferenceError> {
    preference_move_metadata(path).map_err(|error| PreferenceError::Malformed(error.message))?;
    let mut map = renderer_map(path)?;
    map.insert(RENDERER_KEY.into(), state.value().into());
    let bytes = encode_preferences_xml(&map)?;
    replace_file_bytes(path, &bytes).map_err(PreferenceError::Replace)
}

fn renderer_packet(state: RendererState, action: RendererAction, recovered: bool) -> VarDictionary {
    let mut output = VarDictionary::new();
    output.set("ok", true);
    output.set("error", "");
    output.set("state", state.value());
    output.set("target", state.target());
    output.set("action", action.value());
    output.set("recovered", recovered);
    output.set("replacement_committed", false);
    output.set("durability_uncertain", false);
    output
}

fn renderer_failure(error: impl std::fmt::Display) -> VarDictionary {
    let mut output = renderer_packet(RendererState::Compatibility, RendererAction::Continue, false);
    output.set("ok", false);
    output.set("error", error.to_string());
    output
}

impl BgrSimulation {
    fn renderer_path(&self, changing: bool) -> Result<&Path, String> {
        if !self.preferences.initialized {
            return Err("Initialize owned preferences before choosing a renderer".into());
        }
        if changing && (self.settings_applying || self.world.is_some() || self.detached_session.is_some()) {
            return Err("Renderer changes require a fresh Launcher without an active or retained session".into());
        }
        if changing && !self.preferences.error.is_empty() {
            return Err(self.preferences.error.clone());
        }
        self.preferences.path.as_deref().ok_or_else(|| "An owned native preference path is unavailable".into())
    }

    fn renderer_state(&self, changing: bool) -> Result<RendererState, PreferenceError> {
        self.renderer_path(changing).map_err(PreferenceError::Malformed)
            .and_then(renderer_map).and_then(|map| RendererState::read(&map))
    }

    fn renderer_apply(&mut self, prior: RendererState, next: RendererState, action: RendererAction, recovered: bool) -> VarDictionary {
        let writing = prior != next || matches!(action, RendererAction::Restart | RendererAction::Confirm);
        if writing {
            let result = self.renderer_path(true).map_err(PreferenceError::Malformed)
                .and_then(|path| write_renderer(path, next));
            if let Err(error) = result {
                let committed = matches!(&error, PreferenceError::Replace(error) if error.replacement_committed);
                if committed {
                    self.preferences.status = "loaded".into();
                    self.preferences.error = error.to_string();
                    self.preferences.saved = false;
                    self.preferences.replacement_committed = true;
                    self.preferences.durability_uncertain = true;
                }
                let mut output = renderer_failure(error);
                output.set("replacement_committed", committed);
                output.set("durability_uncertain", committed);
                return output;
            }
            self.preferences.status = "loaded".into();
            self.preferences.saved = true;
            self.preferences.replacement_committed = true;
            self.preferences.durability_uncertain = false;
        }
        let mut output = renderer_packet(next, action, recovered);
        output.set("replacement_committed", writing);
        output
    }
}

#[godot_api(secondary)]
impl BgrSimulation {
    #[func]
    fn renderer_status(&self) -> VarDictionary {
        match self.renderer_state(false) {
            Ok(state) => {
                let mut output = renderer_packet(state, RendererAction::Continue, false);
                output.set("busy", self.world.is_some() || self.detached_session.is_some() || self.settings_applying);
                output
            },
            Err(error) => renderer_failure(error),
        }
    }

    #[func]
    fn renderer_startup(&mut self, actual: GString, interactive: bool, safe: bool) -> VarDictionary {
        let actual = actual.to_string();
        if !matches!(actual.as_str(), COMPATIBILITY | MOBILE | "forward_plus") {
            return renderer_failure("The engine reported an unsupported rendering method");
        }
        let state = match self.renderer_state(true) {
            Ok(state) => state,
            Err(error) => return renderer_failure(error),
        };
        if !interactive { return renderer_packet(state, RendererAction::Continue, false); }
        let (next, action, recovered) = startup_plan(state, &actual, safe);
        self.renderer_apply(state, next, action, recovered)
    }

    #[func]
    fn renderer_request(&mut self, target: GString, actual: GString) -> VarDictionary {
        let target = target.to_string();
        let actual = actual.to_string();
        if !matches!(target.as_str(), COMPATIBILITY | MOBILE) || !matches!(actual.as_str(), COMPATIBILITY | MOBILE | "forward_plus") {
            return renderer_failure("Choose a supported native rendering method");
        }
        let prior = match self.renderer_state(true) {
            Ok(state) => state,
            Err(error) => return renderer_failure(error),
        };
        let next = if target == MOBILE { RendererState::StartingTrial } else { RendererState::Compatibility };
        let action = if target == MOBILE && actual == MOBILE { RendererAction::Confirm }
            else if target != actual { RendererAction::Restart } else { RendererAction::Continue };
        self.renderer_apply(prior, next, action, false)
    }

    #[func]
    fn renderer_confirm(&mut self, keep: bool, actual: GString) -> VarDictionary {
        let actual = actual.to_string();
        let prior = match self.renderer_state(true) {
            Ok(state) => state,
            Err(error) => return renderer_failure(error),
        };
        if prior != RendererState::StartingTrial || actual != MOBILE {
            return renderer_failure("Only an active Mobile renderer trial can be confirmed");
        }
        let next = if keep { RendererState::Mobile } else { RendererState::Compatibility };
        self.renderer_apply(prior, next, if keep { RendererAction::Continue } else { RendererAction::Restart }, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_startup_recovers_interrupted_trials_and_confirmed_launches() {
        use RendererAction::*;
        use RendererState::*;
        for (state, actual, safe, expected) in [
            (Compatibility, COMPATIBILITY, false, (Compatibility, Continue, false)),
            (Compatibility, MOBILE, false, (Compatibility, Continue, false)),
            (StartingTrial, MOBILE, false, (StartingTrial, Confirm, false)),
            (StartingTrial, COMPATIBILITY, false, (Compatibility, Continue, true)),
            (StartingConfirmed, MOBILE, false, (Mobile, Continue, false)),
            (StartingConfirmed, COMPATIBILITY, false, (Compatibility, Continue, true)),
            (Mobile, MOBILE, false, (Mobile, Continue, false)),
            (Mobile, COMPATIBILITY, false, (StartingConfirmed, Restart, false)),
            (Mobile, COMPATIBILITY, true, (Compatibility, Continue, false)),
            (StartingTrial, MOBILE, true, (Compatibility, Restart, false)),
            (StartingConfirmed, COMPATIBILITY, true, (Compatibility, Continue, true)),
        ] {
            assert_eq!(startup_plan(state, actual, safe), expected);
        }
    }

    #[test]
    fn renderer_records_share_atomic_preferences_and_reset_restore() {
        let directory = std::env::temp_dir().join(format!("bgr-renderer-records-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join(NATIVE_FILE);
        let original = b"<map><entry key=\"VERSION\" value=\"700\"/><entry key=\"HARDWARE_ACCELERATION\" value=\"4\"/><entry key=\"LOCALE\" value=\"ru\"/><entry key=\"unknown\" value=\"keep\"/></map>";
        fs::write(&path, original).unwrap();
        write_renderer(&path, RendererState::StartingTrial).unwrap();
        let map = renderer_map(&path).unwrap();
        assert_eq!(map["HARDWARE_ACCELERATION"], "4");
        assert_eq!(map["LOCALE"], "ru");
        assert_eq!(map["unknown"], "keep");
        assert_eq!(RendererState::read(&map).unwrap(), RendererState::StartingTrial);
        let saved = fs::read(&path).unwrap();
        assert!(move_preferences(&path, PreferenceAction::Reset).map_err(|error| error.message).unwrap());
        assert_eq!(RendererState::read(&renderer_map(&path).unwrap()).unwrap(), RendererState::Compatibility);
        assert!(move_preferences(&path, PreferenceAction::Restore).map_err(|error| error.message).unwrap());
        assert_eq!(fs::read(&path).unwrap(), saved);
        write_renderer(&path, RendererState::Mobile).unwrap();
        let state = SettingsState::from_native_preferences(&renderer_map(&path).unwrap(), "en".into()).state;
        state.save_preferences(&path).unwrap();
        assert_eq!(RendererState::read(&renderer_map(&path).unwrap()).unwrap(), RendererState::Mobile);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn renderer_rejects_invalid_native_versions_states_and_file_aliases() {
        let directory = std::env::temp_dir().join(format!("bgr-renderer-errors-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join(NATIVE_FILE);
        for bytes in [
            b"<map><entry key=\"VERSION\" value=\"701\"/></map>".as_slice(),
            b"<map><entry key=\"VERSION\" value=\"700\"/><entry key=\"BGR_RENDERER\" value=\"future\"/></map>",
            b"<map>",
        ] {
            fs::write(&path, bytes).unwrap();
            assert!(write_renderer(&path, RendererState::StartingTrial).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        fs::remove_file(&path).unwrap();
        #[cfg(unix)]
        {
            let target = directory.join("original.xml");
            let bytes = b"<map><entry key=\"VERSION\" value=\"700\"/></map>";
            fs::write(&target, bytes).unwrap();
            std::os::unix::fs::symlink(&target, &path).unwrap();
            assert!(write_renderer(&path, RendererState::Mobile).is_err());
            assert_eq!(fs::read(&target).unwrap(), bytes);
            fs::remove_file(&path).unwrap();
            fs::hard_link(&target, &path).unwrap();
            assert!(write_renderer(&path, RendererState::Mobile).is_err());
            assert_eq!(fs::read(&target).unwrap(), bytes);
        }
        fs::remove_dir_all(directory).unwrap();
    }
}
