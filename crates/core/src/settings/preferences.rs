// SPDX-License-Identifier: GPL-2.0-or-later

//! Read-only legacy imports and bounded, atomic native preference files

use super::*;
use crate::{FileReplaceError, replace_file_bytes};
use quick_xml::{Reader, XmlVersion, events::Event};
use std::{
    collections::BTreeMap,
    fmt,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

pub type PreferenceMap = BTreeMap<String, String>;
pub const MAX_PREFERENCE_BYTES: usize = 8 << 20;
const MAX_ENTRIES: usize = 16_384;
const FORMAT_VERSION: i32 = 700;
const CRISP_KEY: &str = "BGR_CRISP_LINES";
const BACKUP_MODE_KEY: &str = "BGR_AUTOMATIC_BACKUP_MODE";
// Native version 1 stores up to fifteen raw UTF-16 units as four-digit hexadecimal groups
const USER_NAME_UTF16_KEY: &str = "BGR_USER_NAME_UTF16";
const MAX_NATIVE_USER_NAME_UNITS: usize = 15;

#[derive(Debug)]
pub enum PreferenceError {
    Malformed(String),
    ResourceLimit(&'static str),
    UnsupportedNativeVersion { stored: Option<i32> },
    InvalidUserNameEncoding,
    Io { path: PathBuf, source: io::Error },
    Replace(FileReplaceError),
}

impl fmt::Display for PreferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(reason) => write!(f, "Malformed preferences: {reason}"),
            Self::ResourceLimit(resource) => write!(f, "Preference {resource} limit exceeded"),
            Self::UnsupportedNativeVersion { stored } => write!(
                f,
                "Cannot replace native preferences with unsupported version {stored:?}"
            ),
            Self::InvalidUserNameEncoding => {
                f.write_str("Unsupported native username encoding; original file retained")
            }
            Self::Io { path, source } => {
                write!(f, "Cannot read preferences {}: {source}", path.display())
            }
            Self::Replace(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PreferenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Replace(error) => Some(error),
            _ => None,
        }
    }
}

fn malformed(reason: impl fmt::Display) -> PreferenceError {
    PreferenceError::Malformed(reason.to_string())
}

fn xml_char(ch: char) -> bool {
    matches!(ch as u32, 0x9 | 0xa | 0xd | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}

/// Decode the UTF-8 map format written by the original Linux JDK preferences backend
/// External resources and custom entities are never resolved
pub fn parse_preferences_xml(bytes: &[u8]) -> Result<PreferenceMap, PreferenceError> {
    if bytes.len() > MAX_PREFERENCE_BYTES {
        return Err(PreferenceError::ResourceLimit("input bytes"));
    }
    let document = std::str::from_utf8(bytes).map_err(malformed)?;
    let document = document.strip_prefix('\u{feff}').unwrap_or(document);
    if !document.chars().all(xml_char) {
        return Err(malformed("Forbidden XML 1.0 character"));
    }
    let mut reader = Reader::from_str(document);
    reader.config_mut().check_comments = true;
    let mut result = PreferenceMap::new();
    let mut state = 0;
    let mut declaration = false;
    let mut doctype = false;
    let mut prior_content = false;
    loop {
        let event = reader.read_event().map_err(malformed)?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Decl(value) => {
                if prior_content
                    || declaration
                    || value.version().map_err(malformed)?.as_ref() != b"1.0"
                {
                    return Err(malformed("Misplaced or unsupported XML declaration"));
                }
                if let Some(encoding) = value.encoding().transpose().map_err(malformed)? {
                    if !encoding.eq_ignore_ascii_case(b"utf-8") {
                        return Err(malformed("Preference map requires UTF-8"));
                    }
                }
                let content = std::str::from_utf8(value.as_ref()).map_err(malformed)?;
                let attributes = quick_xml::events::BytesStart::from_content(content, 3);
                let mut rank = 0;
                for item in attributes.attributes() {
                    let item = item.map_err(malformed)?;
                    let next = match item.key.as_ref() {
                        b"version" => 1,
                        b"encoding" => 2,
                        b"standalone" if matches!(item.value.as_ref(), b"yes" | b"no") => 3,
                        _ => return Err(malformed("Invalid XML declaration attribute")),
                    };
                    if next <= rank {
                        return Err(malformed("Misordered XML declaration attributes"));
                    }
                    rank = next;
                }
                declaration = true;
            }
            Event::DocType(value) => {
                if doctype
                    || state != 0
                    || value.as_ref() != b"map SYSTEM \"http://java.sun.com/dtd/preferences.dtd\""
                {
                    return Err(malformed("Unsupported or misplaced preference DOCTYPE"));
                }
                doctype = true;
                prior_content = true;
            }
            Event::Start(element) | Event::Empty(element) => {
                let name = element.name();
                let is_map = state == 0 && name.as_ref() == b"map";
                let is_entry = state == 1 && name.as_ref() == b"entry";
                if !is_map && !is_entry {
                    return Err(malformed("Expected one map containing empty entries"));
                }
                let mut attributes = BTreeMap::new();
                for attribute in element.attributes() {
                    let attribute = attribute.map_err(malformed)?;
                    if attribute.value.contains(&b'<') {
                        return Err(malformed("Unescaped less-than in XML attribute"));
                    }
                    if attributes.len() >= 2 {
                        return Err(malformed("Too many preference attributes"));
                    }
                    let key = std::str::from_utf8(attribute.key.as_ref())
                        .map_err(malformed)?
                        .to_owned();
                    let value = attribute
                        .normalized_value(XmlVersion::Explicit1_0)
                        .map_err(malformed)?
                        .into_owned();
                    if !value.chars().all(xml_char) || attributes.insert(key, value).is_some() {
                        return Err(malformed("Invalid or duplicate preference attribute"));
                    }
                }
                if is_map {
                    if attributes.len() > 1
                        || attributes
                            .get("MAP_XML_VERSION")
                            .is_some_and(|version| version != "1.0")
                    {
                        return Err(malformed("Unsupported preference map version"));
                    }
                    if !attributes.is_empty() && !attributes.contains_key("MAP_XML_VERSION") {
                        return Err(malformed("Unknown preference map attribute"));
                    }
                    state = if empty { 3 } else { 1 };
                } else {
                    let key = attributes
                        .remove("key")
                        .ok_or_else(|| malformed("Missing preference key"))?;
                    let value = attributes
                        .remove("value")
                        .ok_or_else(|| malformed("Missing preference value"))?;
                    if !attributes.is_empty() {
                        return Err(malformed("Unknown preference entry attribute"));
                    }
                    if result.len() >= MAX_ENTRIES {
                        return Err(PreferenceError::ResourceLimit("entries"));
                    }
                    if result.insert(key, value).is_some() {
                        return Err(malformed("Duplicate preference key"));
                    }
                    if !empty {
                        state = 2;
                    }
                }
                prior_content = true;
            }
            Event::End(element) => {
                state = match (state, element.name().as_ref()) {
                    (2, b"entry") => 1,
                    (1, b"map") => 3,
                    _ => return Err(malformed("Unexpected preference closing tag")),
                };
            }
            Event::Text(value)
                if value
                    .as_ref()
                    .iter()
                    .all(|ch| matches!(ch, b' ' | b'\t' | b'\r' | b'\n')) =>
            {
                prior_content = true;
            }
            Event::Comment(_) => prior_content = true,
            Event::Eof => break,
            _ => return Err(malformed("Unexpected preference XML content")),
        }
    }
    if state != 3 {
        return Err(malformed("Incomplete preference map"));
    }
    Ok(result)
}

fn append(output: &mut String, text: &str) -> Result<(), PreferenceError> {
    if output.len().saturating_add(text.len()) > MAX_PREFERENCE_BYTES {
        return Err(PreferenceError::ResourceLimit("output bytes"));
    }
    output.push_str(text);
    Ok(())
}

fn attribute(output: &mut String, text: &str) -> Result<(), PreferenceError> {
    for ch in text.chars() {
        if !xml_char(ch) {
            return Err(malformed("Forbidden XML 1.0 character"));
        }
        match ch {
            '&' => append(output, "&amp;")?,
            '<' => append(output, "&lt;")?,
            '>' => append(output, "&gt;")?,
            '"' => append(output, "&quot;")?,
            '\t' => append(output, "&#x9;")?,
            '\n' => append(output, "&#xA;")?,
            '\r' => append(output, "&#xD;")?,
            _ => append(output, ch.encode_utf8(&mut [0; 4]))?,
        }
    }
    Ok(())
}

pub fn encode_preferences_xml(map: &PreferenceMap) -> Result<Vec<u8>, PreferenceError> {
    if map.len() > MAX_ENTRIES {
        return Err(PreferenceError::ResourceLimit("entries"));
    }
    let mut output = String::new();
    append(
        &mut output,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<map MAP_XML_VERSION=\"1.0\">\n",
    )?;
    for (key, value) in map {
        append(&mut output, "  <entry key=\"")?;
        attribute(&mut output, key)?;
        append(&mut output, "\" value=\"")?;
        attribute(&mut output, value)?;
        append(&mut output, "\"/>\n")?;
    }
    append(&mut output, "</map>\n")?;
    Ok(output.into_bytes())
}

/// Missing native files are distinct from invalid or unreadable existing files
pub fn read_preferences_file(path: &Path) -> Result<Option<PreferenceMap>, PreferenceError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => {
            return Err(PreferenceError::Io {
                path: path.to_owned(),
                source: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Preference file must be regular and not a symbolic link",
                ),
            });
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(PreferenceError::Io {
                path: path.to_owned(),
                source,
            });
        }
        _ => {}
    }
    let source = match File::open(path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(PreferenceError::Io {
                path: path.to_owned(),
                source,
            });
        }
    };
    let mut bytes = Vec::new();
    source
        .take((MAX_PREFERENCE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| PreferenceError::Io {
            path: path.to_owned(),
            source,
        })?;
    parse_preferences_xml(&bytes).map(Some)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferenceDisposition {
    Loaded,
    InvalidAutomaticBackupMode,
    InvalidUserNameEncoding,
    DefaultsForVersion { stored: Option<i32> },
}

/// Source startup requests do not select or validate a Godot rendering backend
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareStartup {
    pub requested: i32,
    pub effective: i32,
    pub persisted_override: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreferenceLoad {
    pub state: SettingsState,
    pub disposition: PreferenceDisposition,
    pub hardware: HardwareStartup,
}

fn language(value: &str) -> String {
    let value = value.to_ascii_lowercase();
    match value.as_str() {
        "iw" => "he".to_owned(),
        "ji" => "yi".to_owned(),
        "in" => "id".to_owned(),
        _ => value,
    }
}

fn ordinary_user_name(value: &JavaText) -> Option<String> {
    value.to_utf8().ok().filter(|text| text.chars().all(xml_char))
}

fn native_user_name(map: &PreferenceMap) -> Result<Option<JavaText>, PreferenceError> {
    let Some(encoded) = map.get(USER_NAME_UTF16_KEY) else {
        return Ok(None);
    };
    let hex = encoded.strip_prefix("1:").ok_or(PreferenceError::InvalidUserNameEncoding)?;
    if map.get("USER_NAME").map(String::as_str) != Some("")
        || hex.is_empty()
        || hex.len() % 4 != 0
        || hex.len() / 4 > MAX_NATIVE_USER_NAME_UNITS
        || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(PreferenceError::InvalidUserNameEncoding);
    }
    let units = hex.as_bytes().chunks_exact(4).map(|chunk| {
        let text = std::str::from_utf8(chunk).expect("Validated ASCII hexadecimal units");
        u16::from_str_radix(text, 16).expect("Validated four-digit hexadecimal unit")
    }).collect();
    Ok(Some(JavaText::from_units(units)))
}

fn window_key(key: SettingKey) -> bool {
    matches!(
        key,
        SettingKey::WindowX
            | SettingKey::WindowY
            | SettingKey::WindowWidth
            | SettingKey::WindowHeight
            | SettingKey::WindowState
    )
}

impl SettingsState {
    /// Read the native-only display choice without changing legacy preference parsing
    pub fn from_native_preferences(map: &PreferenceMap, default_language: String) -> PreferenceLoad {
        let mut loaded = Self::from_preferences(map, default_language.clone());
        if loaded.disposition == PreferenceDisposition::Loaded {
            match native_user_name(map) {
                Ok(Some(value)) => loaded.state.application.user_name = value,
                Ok(None) => {}
                Err(_) => {
                    loaded.state = Self::new(default_language);
                    loaded.disposition = PreferenceDisposition::InvalidUserNameEncoding;
                    loaded.hardware = HardwareStartup { requested: 0, effective: 0, persisted_override: None };
                    return loaded;
                }
            }
            loaded.state.set_crisp_lines(
                map.get(CRISP_KEY).is_some_and(|value| value.eq_ignore_ascii_case("true")),
            );
            if let Some(value) = map.get(BACKUP_MODE_KEY) {
                if let Some(mode) = AutomaticBackupMode::from_native_value(value) {
                    loaded.state.set_automatic_backup_mode(mode);
                } else {
                    loaded.state = Self::new(default_language);
                    loaded.disposition = PreferenceDisposition::InvalidAutomaticBackupMode;
                    loaded.hardware = HardwareStartup { requested: 0, effective: 0, persisted_override: None };
                }
            }
        }
        loaded
    }

    /// Stage typed source values without clearing legacy maps or applying dialog range restrictions
    pub fn from_preferences(map: &PreferenceMap, default_language: String) -> PreferenceLoad {
        let mut state = Self::new(default_language);
        let version = map.get("VERSION").and_then(|text| java_int(text));
        let disposition = if version == Some(FORMAT_VERSION) {
            for key in ALL_SETTING_KEYS {
                if key == SettingKey::LocaleIndex {
                    continue;
                }
                let Some(text) = map.get(key.source_name()) else {
                    continue;
                };
                let value = match state.get(key) {
                    SettingValue::Int(_) => java_int(text).map(SettingValue::Int),
                    SettingValue::Double(_) => java_double(text).map(SettingValue::Double),
                    SettingValue::Boolean(_) if text.eq_ignore_ascii_case("true") => {
                        Some(SettingValue::Boolean(true))
                    }
                    SettingValue::Boolean(_) if text.eq_ignore_ascii_case("false") => {
                        Some(SettingValue::Boolean(false))
                    }
                    SettingValue::Boolean(_) => None,
                    SettingValue::Text(_) => {
                        Some(SettingValue::Text(if key == SettingKey::Locale {
                            language(text)
                        } else {
                            text.clone()
                        }))
                    }
                };
                if let Some(value) = value {
                    state
                        .assign(key, value)
                        .expect("Preference value uses its setting's declared type");
                }
            }
            PreferenceDisposition::Loaded
        } else {
            PreferenceDisposition::DefaultsForVersion { stored: version }
        };
        let requested = state.application.hardware_acceleration;
        let retry = matches!(requested, 1 | 4);
        let effective = requested + i32::from(retry);
        state.application.hardware_acceleration = effective;
        PreferenceLoad {
            state,
            disposition,
            hardware: HardwareStartup {
                requested,
                effective,
                persisted_override: retry.then_some(0),
            },
        }
    }

    /// Match ordinary Utils.savePreferences, preserving unknown keys and existing window geometry
    pub fn update_preferences(&self, map: &mut PreferenceMap) {
        for key in ALL_SETTING_KEYS {
            if key == SettingKey::LocaleIndex || window_key(key) {
                continue;
            }
            let value = if key == SettingKey::UserName {
                ordinary_user_name(&self.application.user_name).unwrap_or_default()
            } else {
                match self.get(key) {
                    SettingValue::Int(value) => value.to_string(),
                    SettingValue::Double(value) => double_text(value),
                    SettingValue::Boolean(value) => value.to_string(),
                    SettingValue::Text(value) => value,
                }
            };
            map.insert(key.source_name().to_owned(), value);
        }
        map.insert("VERSION".to_owned(), FORMAT_VERSION.to_string());
    }

    /// Save the owned native file after complete decoding and encoding; never target a legacy import
    pub fn save_preferences(&self, path: &Path) -> Result<(), PreferenceError> {
        let mut map = read_preferences_file(path)?.unwrap_or_default();
        if let Some(version) = map.get("VERSION") {
            let stored = java_int(version);
            if stored != Some(FORMAT_VERSION) {
                return Err(PreferenceError::UnsupportedNativeVersion { stored });
            }
        }
        if map.get(BACKUP_MODE_KEY).is_some_and(|value| AutomaticBackupMode::from_native_value(value).is_none()) {
            return Err(malformed("Unsupported native automatic backup mode; original file retained"));
        }
        native_user_name(&map)?;
        self.update_preferences(&mut map);
        if let Some(value) = ordinary_user_name(&self.application.user_name) {
            map.insert("USER_NAME".to_owned(), value);
            map.remove(USER_NAME_UTF16_KEY);
        } else {
            if self.application.user_name.units().len() > MAX_NATIVE_USER_NAME_UNITS {
                return Err(PreferenceError::InvalidUserNameEncoding);
            }
            use std::fmt::Write;
            let mut encoded = String::from("1:");
            for unit in self.application.user_name.units() {
                write!(&mut encoded, "{unit:04x}").expect("Writing to a String cannot fail");
            }
            map.insert("USER_NAME".to_owned(), String::new());
            map.insert(USER_NAME_UTF16_KEY.to_owned(), encoded);
        }
        map.insert(BACKUP_MODE_KEY.to_owned(), self.automatic_backup_mode().native_value().to_owned());
        for key in ALL_SETTING_KEYS.into_iter().filter(|key| window_key(*key)) {
            let SettingValue::Int(value) = self.get(key) else {
                unreachable!()
            };
            map.insert(key.source_name().to_owned(), value.to_string());
        }
        map.insert(CRISP_KEY.to_owned(), self.crisp_lines().to_string());
        let bytes = encode_preferences_xml(&map)?;
        replace_file_bytes(path, &bytes).map_err(PreferenceError::Replace)
    }
}

#[cfg(test)]
#[path = "preferences_tests.rs"]
mod tests;
