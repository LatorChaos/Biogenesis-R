// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{
    ALL_SETTING_KEYS, EditorOutcome, HardwareSelection, NUMERIC_EDITOR_KEYS, SettingValue,
    SettingsDraft, SettingsState, AutomaticBackupMode,
};
use godot::builtin::{Callable, PackedStringArray, VarArray, VariantType};

const BUSY: &str = "Settings are being applied; reentrant writes are not allowed";

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub(super) struct BgrSettingsDraft {
    draft: Option<SettingsDraft>,
    applying: bool,
    error: Option<String>,
    automatic_backup_mode: AutomaticBackupMode,
}

#[godot_api]
impl BgrSettingsDraft {
    #[func]
    fn snapshot(&self) -> VarDictionary {
        let Some(draft) = self.draft.as_ref() else {
            return outcome(Some(
                self.error
                    .as_deref()
                    .unwrap_or("This settings draft is unavailable"),
            ));
        };
        let mut result = outcome(None);
        let mut keys = PackedStringArray::new();
        let mut texts = PackedStringArray::new();
        for (key, text) in NUMERIC_EDITOR_KEYS.iter().zip(draft.editors()) {
            keys.push(key.source_name());
            texts.push(text.as_str());
        }
        result.set("keys", &keys);
        result.set("texts", &texts);
        result.set("locale", draft.locale_index() as i64);
        result.set("automatic_backup", draft.automatic_backup());
        result.set("automatic_backup_mode", mode_number(self.automatic_backup_mode));
        result.set("hardware", hardware_number(draft.hardware()));
        result.set("disable_framebuffer", draft.disable_framebuffer());
        result.set("backup_delay_enabled", draft.backup_delay_enabled());
        result.set("framebuffer_enabled", draft.framebuffer_control_enabled());
        result.set("open", draft.is_open());
        result.set("busy", self.applying);
        result
    }

    #[func]
    fn replace_controls(&mut self, controls: VarDictionary) -> VarDictionary {
        if self.applying {
            return outcome(Some(BUSY));
        }
        let parsed = parse_controls(&controls);
        let (texts, locale, backup, hardware, fbo, mode) = match parsed {
            Ok(values) => values,
            Err(error) => return outcome(Some(&error)),
        };
        let Some(draft) = self.draft.as_mut() else {
            return outcome(Some("This settings draft is unavailable"));
        };
        if !draft.is_open() {
            return outcome(Some("This settings draft has closed"));
        }
        if let Err(error) = draft.set_choices(locale, backup, hardware, fbo) {
            return outcome(Some(&format!("{error:?}")));
        }
        match draft.replace_editors(texts) {
            Ok(()) => {
                if let Some(mode) = mode { self.automatic_backup_mode = mode; }
                outcome(None)
            },
            Err(error) => outcome(Some(&format!("{error:?}"))),
        }
    }

    #[func]
    fn defaults(&mut self) -> VarDictionary {
        if self.applying {
            return outcome(Some(BUSY));
        }
        match self
            .draft
            .as_mut()
            .ok_or("This settings draft is unavailable")
            .and_then(|draft| {
                draft
                    .defaults()
                    .map_err(|_| "This settings draft has closed")
            }) {
            Ok(()) => {
                self.automatic_backup_mode = AutomaticBackupMode::Adjacent;
                self.snapshot()
            },
            Err(error) => outcome(Some(error)),
        }
    }

    #[func]
    fn cancel(&mut self) -> VarDictionary {
        if self.applying {
            return outcome(Some(BUSY));
        }
        match self
            .draft
            .as_mut()
            .ok_or("This settings draft is unavailable")
            .and_then(|draft| draft.cancel().map_err(|_| "This settings draft has closed"))
        {
            Ok(()) => outcome(None),
            Err(error) => outcome(Some(error)),
        }
    }
}

fn parse_controls(
    controls: &VarDictionary,
) -> Result<([String; 132], usize, bool, HardwareSelection, bool, Option<AutomaticBackupMode>), String> {
    if controls.len() != 5 && !(controls.len() == 6 && controls.contains_key("automatic_backup_mode")) {
        return Err("Five Source control groups and an optional automatic backup mode are required".to_owned());
    }
    let required = |key: &str, kind| -> Result<Variant, String> {
        let value = controls
            .get(key)
            .ok_or_else(|| format!("Missing settings control {key}"))?;
        if value.get_type() != kind {
            return Err(format!("Wrong settings control type: {key}"));
        }
        Ok(value)
    };
    let texts = required("texts", VariantType::PACKED_STRING_ARRAY)?
        .try_to::<PackedStringArray>()
        .map_err(|_| "Invalid settings text array".to_owned())?;
    if texts.len() != 132 {
        return Err("Exactly 132 numeric editor texts are required".to_owned());
    }
    let locale = required("locale", VariantType::INT)?.to::<i64>();
    if !(0..6).contains(&locale) {
        return Err("Language index must be between zero and five".to_owned());
    }
    let hardware = match required("hardware", VariantType::INT)?.to::<i64>() {
        -1 => HardwareSelection::Unselected,
        0 => HardwareSelection::None,
        1 => HardwareSelection::OpenGl,
        _ => return Err("Unknown legacy graphics selection".to_owned()),
    };
    let backup = required("automatic_backup", VariantType::BOOL)?.to::<bool>();
    let fbo = required("disable_framebuffer", VariantType::BOOL)?.to::<bool>();
    let mode = if controls.contains_key("automatic_backup_mode") {
        Some(match required("automatic_backup_mode", VariantType::INT)?.to::<i64>() {
            0 => AutomaticBackupMode::Adjacent,
            1 => AutomaticBackupMode::Managed,
            _ => return Err("Unknown automatic backup mode".into()),
        })
    } else { None };
    Ok((
        std::array::from_fn(|index| texts[index].to_string()),
        locale as usize,
        backup,
        hardware,
        fbo,
        mode,
    ))
}

fn mode_number(mode: AutomaticBackupMode) -> i64 {
    match mode { AutomaticBackupMode::Adjacent => 0, AutomaticBackupMode::Managed => 1 }
}

fn hardware_number(value: HardwareSelection) -> i64 {
    match value {
        HardwareSelection::Unselected => -1,
        HardwareSelection::None => 0,
        HardwareSelection::OpenGl => 1,
    }
}

pub(super) fn state_fields(state: &SettingsState) -> VarDictionary {
    let mut values = VarDictionary::new();
    for key in ALL_SETTING_KEYS {
        match state.get(key) {
            SettingValue::Int(value) => values.set(key.source_name(), value),
            SettingValue::Double(value) => values.set(key.source_name(), value),
            SettingValue::Boolean(value) => values.set(key.source_name(), value),
            SettingValue::Text(value) => values.set(key.source_name(), value),
        };
    }
    let mut result = outcome(None);
    result.set("values", &values);
    result.set("language", state.language());
    result.set("locale", state.locale_index() as i64);
    let genesis = state.genesis();
    result.set(
        "genesis",
        &PackedInt64Array::from(
            &[
                i64::from(genesis.width),
                i64::from(genesis.height),
                i64::from(genesis.initial_attempts),
            ][..],
        ),
    );
    result.set("busy", false);
    result.set("crisp_lines", state.crisp_lines());
    result.set("automatic_backup_mode", mode_number(state.automatic_backup_mode()));
    result
}

#[godot_api(secondary)]
impl BgrSimulation {
    #[func]
    pub(super) fn settings_snapshot(&self) -> VarDictionary {
        if let Some(presentation) = self.settings_presentation.as_ref() {
            return presentation.duplicate_deep();
        }
        match &self.settings {
            Some(state) => state_fields(state),
            None => state_fields(&SettingsState::new("en".to_owned())),
        }
    }

    #[func]
    fn settings_open(&mut self) -> Gd<BgrSettingsDraft> {
        if self.settings_applying {
            return Gd::from_object(BgrSettingsDraft {
                draft: None,
                applying: false,
                error: Some(BUSY.to_owned()),
                automatic_backup_mode: AutomaticBackupMode::Adjacent,
            });
        }
        let state = self
            .settings
            .get_or_insert_with(|| SettingsState::new("en".to_owned()));
        Gd::from_object(BgrSettingsDraft {
            draft: Some(SettingsDraft::new(state)),
            applying: false,
            error: None,
            automatic_backup_mode: state.automatic_backup_mode(),
        })
    }

    #[func(gd_self)]
    fn settings_apply(
        mut this: Gd<Self>,
        mut token: Gd<BgrSettingsDraft>,
        locale_callback: Callable,
    ) -> VarDictionary {
        if !locale_callback.is_valid() || locale_callback.get_argument_count() != 1 {
            return outcome(Some(
                "A one-argument locale presentation callback is required",
            ));
        }
        let (mut state, mut draft, mode) = {
            let mut owner = this.bind_mut();
            if owner.settings_applying {
                return outcome(Some(BUSY));
            }
            let mut binding = token.bind_mut();
            if binding.applying {
                return outcome(Some(BUSY));
            }
            if !binding.draft.as_ref().is_some_and(SettingsDraft::is_open) {
                return outcome(Some("This settings draft has closed"));
            }
            owner.settings_applying = true;
            binding.applying = true;
            (
                owner
                    .settings
                    .take()
                    .unwrap_or_else(|| SettingsState::new("en".to_owned())),
                binding.draft.take().expect("Checked populated draft"),
                binding.automatic_backup_mode,
            )
        };
        let result = draft.ok(&mut state, |before_numeric| {
            let mut payload = state_fields(before_numeric);
            payload.set("busy", true);
            {
                this.bind_mut().settings_presentation = Some(payload.duplicate_deep());
            }
            let mut args = VarArray::new();
            args.push(&payload.to_variant());
            locale_callback.callv(&args);
        });
        {
            let mut owner = this.bind_mut();
            if result.is_ok() {
                state.set_automatic_backup_mode(mode);
                owner.apply_runtime_settings(&state);
            }
            owner.settings = Some(state);
            owner.settings_presentation = None;
            owner.settings_applying = false;
            if result.is_ok() { owner.automatic_backup_check(); }
        }
        {
            let mut binding = token.bind_mut();
            binding.draft = Some(draft);
            binding.applying = false;
        }
        match result {
            Err(error) => outcome(Some(&format!("{error:?}"))),
            Ok(report) => {
                let owner = this.bind();
                let mut output =
                    state_fields(owner.settings.as_ref().expect("Restored settings owner"));
                output.set("automatic_backup", &owner.automatic_status());
                let mut retained = PackedStringArray::new();
                let mut outcomes = Vec::new();
                for (key, result) in NUMERIC_EDITOR_KEYS.iter().zip(report.editors) {
                    let (code, reason) = match result {
                        EditorOutcome::Applied => (0, ""),
                        EditorOutcome::InvalidNumber => (1, "invalid number"),
                        EditorOutcome::OutsideRange => (2, "outside range"),
                    };
                    outcomes.push(code);
                    if code != 0 {
                        retained.push(format!("{}: {reason}", key.source_name()).as_str());
                    }
                }
                output.set("retained", &retained);
                output.set("outcomes", &PackedInt32Array::from(outcomes.as_slice()));
                output.set("locale_changed", report.locale_changed);
                output.set(
                    "legacy_hardware_request",
                    &match report.legacy_hardware_request {
                        Some(value) => i64::from(value).to_variant(),
                        None => Variant::nil(),
                    },
                );
                output
            }
        }
    }
}

impl BgrSimulation {
    pub(super) fn apply_runtime_settings(&mut self, state: &SettingsState) {
        if let Some(world) = self.world.as_mut() {
            world.apply_settings_parameters(state);
        } else if let Some(detached) = &self.detached_session {
            detached.session.apply_settings_parameters(state);
        }
    }

    pub(super) fn record_applied_config(&mut self, config: WorldConfig) {
        record_applied(&mut self.settings, config);
    }

    pub(super) fn settings_parameters(&mut self) -> WorldParameters {
        self.settings
            .get_or_insert_with(|| SettingsState::new("en".to_owned()))
            .parameters()
            .clone()
    }
}

pub(super) fn record_applied(owner: &mut Option<SettingsState>, config: WorldConfig) {
    let genesis = biogenesis_core::GenesisSettings {
        width: config.width,
        height: config.height,
        initial_attempts: i32::try_from(config.initial_attempts)
            .expect("Validated initial attempts"),
    };
    owner
        .get_or_insert_with(|| SettingsState::new("en".to_owned()))
        .record_applied_genesis(genesis)
        .expect("Applied genesis values satisfy source bounds");
}

fn outcome(error: Option<&str>) -> VarDictionary {
    let mut output = VarDictionary::new();
    output.set("ok", error.is_none());
    output.set("error", error.unwrap_or(""));
    output
}
