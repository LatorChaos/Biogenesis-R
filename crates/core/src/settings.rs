// SPDX-License-Identifier: GPL-2.0-or-later

use crate::java_numbers::{double_text, java_double, java_int};
use crate::{JavaText, WorldConfig, WorldParameters};

mod preferences;
pub use preferences::{
    HardwareStartup, MAX_PREFERENCE_BYTES, PreferenceDisposition, PreferenceError, PreferenceLoad,
    PreferenceMap, encode_preferences_xml, parse_preferences_xml, read_preferences_file,
};

pub const LANGUAGE_CODES: [&str; 6] = ["en", "es", "ca", "de", "pt", "ru"];

/// Native automatic-persistence destination; source setting keys remain unchanged
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutomaticBackupMode {
    #[default]
    Adjacent,
    Managed,
}

impl AutomaticBackupMode {
    pub fn native_value(self) -> &'static str {
        match self { Self::Adjacent => "adjacent", Self::Managed => "managed" }
    }

    pub fn from_native_value(value: &str) -> Option<Self> {
        match value { "adjacent" => Some(Self::Adjacent), "managed" => Some(Self::Managed), _ => None }
    }
}


/// Source preference values; no session seed belongs to this owner
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisSettings {
    pub width: i32,
    pub height: i32,
    pub initial_attempts: i32,
}

impl Default for GenesisSettings {
    fn default() -> Self {
        Self {
            width: 7000,
            height: 7000,
            initial_attempts: 12500,
        }
    }
}

/// Read-only values from the single settings owner
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Int(i32),
    Double(f64),
    Boolean(bool),
    Text(String),
}

impl SettingValue {
    fn editor_text(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Double(value) => double_text(*value),
            _ => unreachable!("Only numeric fields have dialog editors"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsError {
    ClosedDraft,
    NotNumericEditor,
    InvalidLocale,
    WrongValueType,
    InvalidGenesis,
    InvalidWindowGeometry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorOutcome {
    Applied,
    InvalidNumber,
    OutsideRange,
}

/// Parse a remote socket port without changing source editor acceptance rules
pub fn parse_network_remote_port(text: &str) -> Option<u16> {
    java_int(text).and_then(|port| u16::try_from(port).ok()).filter(|port| *port > 0)
}

/// Source network values, including usernames measured in Java UTF-16 code units
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkSettings {
    pub accept_connections: bool,
    pub local_port: i32,
    pub max_connections: i32,
    pub user_name: JavaText,
}

/// Independent editor outcomes and values needed for ordered runtime side effects
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkSettingsApply {
    pub before: NetworkSettings,
    pub after: NetworkSettings,
    pub local_port: EditorOutcome,
    pub max_connections: EditorOutcome,
    pub user_name_applied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareSelection {
    Unselected,
    None,
    OpenGl,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsApply {
    pub editors: [EditorOutcome; 132],
    pub locale_changed: bool,
    pub legacy_hardware_request: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKey {
    AcceptConnections,
    AgeDivisor,
    AuburnEnergyConsumption,
    AuburnProb,
    AutoBackup,
    BackupDelay,
    BarkEnergyConsumption,
    BarkProb,
    BlondEnergyConsumption,
    BlondProb,
    BlueEnergyConsumption,
    BlueProb,
    C4EnergyConsumption,
    C4Prob,
    Ch4ToCo2Divisor,
    CladeComplexity,
    Co2ToCh4Divisor,
    ConnectToServer,
    CoralEnergyConsumption,
    CoralProb,
    CreamEnergyConsumption,
    CreamOrganicSubsProduced,
    CreamProb,
    CrowdedforestEnergyConsumption,
    CrowdedlimeEnergyConsumption,
    CyanEnergyConsumption,
    CyanProb,
    DarkgrayEnergyConsumption,
    DarkgrayProb,
    DarkjadeDelay,
    DarkEnergyConsumption,
    DarkProb,
    DecayEnergy,
    Delay,
    DodgeEnergyConsumption,
    DrainSubsDivisor,
    Elasticity,
    ExperienceEnergyConsumption,
    EyeEnergyConsumption,
    EyeProb,
    FallowEnergyConsumption,
    FallowProb,
    FireEnergyConsumption,
    FireProb,
    FlowerEnergyConsumption,
    FlowerProb,
    ForestEnergyConsumption,
    ForestProb,
    GoldDivisor,
    GoldProb,
    GrassEnergyConsumption,
    GrassProb,
    GrayEnergyConsumption,
    GrayProb,
    GreenEnergyConsumption,
    GreenObtainedEnergyDivisor,
    GreenProb,
    HardwareAcceleration,
    Healing,
    ImmuneSystem,
    IndigoDivisor,
    IndigoEnergyConsumption,
    IndigoProb,
    InitialCh4,
    InitialCo2,
    InitialComplexity,
    InitialEnergy,
    InitialO2,
    InitialOrganisms,
    JadeEnergyConsumption,
    JadeProb,
    LavenderEnergyConsumption,
    LavenderProb,
    LavenderShield,
    LeafEnergyConsumption,
    LeafProb,
    LilacEnergyConsumption,
    LilacProb,
    LimeEnergyConsumption,
    LimeProb,
    Locale,
    LocalPort,
    MagentaEnergyConsumption,
    MagentaProb,
    MaroonEnergyConsumption,
    MaroonProb,
    MaxAge,
    MaxConnections,
    MaxMutationRate,
    MaxRot,
    MaxVel,
    MetaMutationRate,
    MintEnergyConsumption,
    MintProb,
    MinMutationRate,
    MosquitoEnergyConsumption,
    OchreEnergyConsumption,
    OchreProb,
    OliveEnergyConsumption,
    OliveProb,
    OrangeEnergyConsumption,
    OrangeProb,
    OrganicObtainedEnergy,
    OrganicSubsProduced,
    OrganismsVectorSize,
    PinkEnergyConsumption,
    PinkProb,
    PlagueEnergyConsumption,
    PlagueProb,
    PurpleEnergyConsumption,
    PurpleProb,
    RedEnergyConsumption,
    RedProb,
    RoseEnergyConsumption,
    RoseProb,
    Rubbing,
    ScourgeEnergyConsumption,
    SegmentCostDivisor,
    ServerAddress,
    ServerPort,
    SilverEnergyConsumption,
    SilverProb,
    SkyEnergyConsumption,
    SkyProb,
    SpikeEnergyConsumption,
    SpikeProb,
    SporeEnergyConsumption,
    SporeProb,
    SpringEnergyConsumption,
    SpringProb,
    SummerEnergyConsumption,
    SummerProb,
    SymbiontEnergyConsumption,
    TealEnergyConsumption,
    TealProb,
    UserName,
    VioletEnergyConsumption,
    VioletProb,
    VirusEnergyConsumption,
    WhiteEnergyConsumption,
    WhiteProb,
    WindowHeight,
    WindowState,
    WindowWidth,
    WindowX,
    WindowY,
    WorldHeight,
    WorldWidth,
    YellowEnergyConsumption,
    YellowProb,
    LocaleIndex,
}

pub const ALL_SETTING_KEYS: [SettingKey; 151] = [
    SettingKey::AcceptConnections,
    SettingKey::AgeDivisor,
    SettingKey::AuburnEnergyConsumption,
    SettingKey::AuburnProb,
    SettingKey::AutoBackup,
    SettingKey::BackupDelay,
    SettingKey::BarkEnergyConsumption,
    SettingKey::BarkProb,
    SettingKey::BlondEnergyConsumption,
    SettingKey::BlondProb,
    SettingKey::BlueEnergyConsumption,
    SettingKey::BlueProb,
    SettingKey::C4EnergyConsumption,
    SettingKey::C4Prob,
    SettingKey::Ch4ToCo2Divisor,
    SettingKey::CladeComplexity,
    SettingKey::Co2ToCh4Divisor,
    SettingKey::ConnectToServer,
    SettingKey::CoralEnergyConsumption,
    SettingKey::CoralProb,
    SettingKey::CreamEnergyConsumption,
    SettingKey::CreamOrganicSubsProduced,
    SettingKey::CreamProb,
    SettingKey::CrowdedforestEnergyConsumption,
    SettingKey::CrowdedlimeEnergyConsumption,
    SettingKey::CyanEnergyConsumption,
    SettingKey::CyanProb,
    SettingKey::DarkgrayEnergyConsumption,
    SettingKey::DarkgrayProb,
    SettingKey::DarkjadeDelay,
    SettingKey::DarkEnergyConsumption,
    SettingKey::DarkProb,
    SettingKey::DecayEnergy,
    SettingKey::Delay,
    SettingKey::DodgeEnergyConsumption,
    SettingKey::DrainSubsDivisor,
    SettingKey::Elasticity,
    SettingKey::ExperienceEnergyConsumption,
    SettingKey::EyeEnergyConsumption,
    SettingKey::EyeProb,
    SettingKey::FallowEnergyConsumption,
    SettingKey::FallowProb,
    SettingKey::FireEnergyConsumption,
    SettingKey::FireProb,
    SettingKey::FlowerEnergyConsumption,
    SettingKey::FlowerProb,
    SettingKey::ForestEnergyConsumption,
    SettingKey::ForestProb,
    SettingKey::GoldDivisor,
    SettingKey::GoldProb,
    SettingKey::GrassEnergyConsumption,
    SettingKey::GrassProb,
    SettingKey::GrayEnergyConsumption,
    SettingKey::GrayProb,
    SettingKey::GreenEnergyConsumption,
    SettingKey::GreenObtainedEnergyDivisor,
    SettingKey::GreenProb,
    SettingKey::HardwareAcceleration,
    SettingKey::Healing,
    SettingKey::ImmuneSystem,
    SettingKey::IndigoDivisor,
    SettingKey::IndigoEnergyConsumption,
    SettingKey::IndigoProb,
    SettingKey::InitialCh4,
    SettingKey::InitialCo2,
    SettingKey::InitialComplexity,
    SettingKey::InitialEnergy,
    SettingKey::InitialO2,
    SettingKey::InitialOrganisms,
    SettingKey::JadeEnergyConsumption,
    SettingKey::JadeProb,
    SettingKey::LavenderEnergyConsumption,
    SettingKey::LavenderProb,
    SettingKey::LavenderShield,
    SettingKey::LeafEnergyConsumption,
    SettingKey::LeafProb,
    SettingKey::LilacEnergyConsumption,
    SettingKey::LilacProb,
    SettingKey::LimeEnergyConsumption,
    SettingKey::LimeProb,
    SettingKey::Locale,
    SettingKey::LocalPort,
    SettingKey::MagentaEnergyConsumption,
    SettingKey::MagentaProb,
    SettingKey::MaroonEnergyConsumption,
    SettingKey::MaroonProb,
    SettingKey::MaxAge,
    SettingKey::MaxConnections,
    SettingKey::MaxMutationRate,
    SettingKey::MaxRot,
    SettingKey::MaxVel,
    SettingKey::MetaMutationRate,
    SettingKey::MintEnergyConsumption,
    SettingKey::MintProb,
    SettingKey::MinMutationRate,
    SettingKey::MosquitoEnergyConsumption,
    SettingKey::OchreEnergyConsumption,
    SettingKey::OchreProb,
    SettingKey::OliveEnergyConsumption,
    SettingKey::OliveProb,
    SettingKey::OrangeEnergyConsumption,
    SettingKey::OrangeProb,
    SettingKey::OrganicObtainedEnergy,
    SettingKey::OrganicSubsProduced,
    SettingKey::OrganismsVectorSize,
    SettingKey::PinkEnergyConsumption,
    SettingKey::PinkProb,
    SettingKey::PlagueEnergyConsumption,
    SettingKey::PlagueProb,
    SettingKey::PurpleEnergyConsumption,
    SettingKey::PurpleProb,
    SettingKey::RedEnergyConsumption,
    SettingKey::RedProb,
    SettingKey::RoseEnergyConsumption,
    SettingKey::RoseProb,
    SettingKey::Rubbing,
    SettingKey::ScourgeEnergyConsumption,
    SettingKey::SegmentCostDivisor,
    SettingKey::ServerAddress,
    SettingKey::ServerPort,
    SettingKey::SilverEnergyConsumption,
    SettingKey::SilverProb,
    SettingKey::SkyEnergyConsumption,
    SettingKey::SkyProb,
    SettingKey::SpikeEnergyConsumption,
    SettingKey::SpikeProb,
    SettingKey::SporeEnergyConsumption,
    SettingKey::SporeProb,
    SettingKey::SpringEnergyConsumption,
    SettingKey::SpringProb,
    SettingKey::SummerEnergyConsumption,
    SettingKey::SummerProb,
    SettingKey::SymbiontEnergyConsumption,
    SettingKey::TealEnergyConsumption,
    SettingKey::TealProb,
    SettingKey::UserName,
    SettingKey::VioletEnergyConsumption,
    SettingKey::VioletProb,
    SettingKey::VirusEnergyConsumption,
    SettingKey::WhiteEnergyConsumption,
    SettingKey::WhiteProb,
    SettingKey::WindowHeight,
    SettingKey::WindowState,
    SettingKey::WindowWidth,
    SettingKey::WindowX,
    SettingKey::WindowY,
    SettingKey::WorldHeight,
    SettingKey::WorldWidth,
    SettingKey::YellowEnergyConsumption,
    SettingKey::YellowProb,
    SettingKey::LocaleIndex,
];

pub const NUMERIC_EDITOR_KEYS: [SettingKey; 132] = [
    SettingKey::WorldWidth,
    SettingKey::WorldHeight,
    SettingKey::Delay,
    SettingKey::BackupDelay,
    SettingKey::InitialO2,
    SettingKey::InitialCo2,
    SettingKey::InitialCh4,
    SettingKey::Rubbing,
    SettingKey::Elasticity,
    SettingKey::InitialOrganisms,
    SettingKey::InitialEnergy,
    SettingKey::InitialComplexity,
    SettingKey::CladeComplexity,
    SettingKey::MaxAge,
    SettingKey::AgeDivisor,
    SettingKey::Co2ToCh4Divisor,
    SettingKey::Ch4ToCo2Divisor,
    SettingKey::MetaMutationRate,
    SettingKey::MaxMutationRate,
    SettingKey::MinMutationRate,
    SettingKey::RedEnergyConsumption,
    SettingKey::GreenEnergyConsumption,
    SettingKey::BlueEnergyConsumption,
    SettingKey::CyanEnergyConsumption,
    SettingKey::WhiteEnergyConsumption,
    SettingKey::GrayEnergyConsumption,
    SettingKey::YellowEnergyConsumption,
    SettingKey::MagentaEnergyConsumption,
    SettingKey::PinkEnergyConsumption,
    SettingKey::CoralEnergyConsumption,
    SettingKey::OrangeEnergyConsumption,
    SettingKey::ForestEnergyConsumption,
    SettingKey::SpringEnergyConsumption,
    SettingKey::LeafEnergyConsumption,
    SettingKey::SummerEnergyConsumption,
    SettingKey::LimeEnergyConsumption,
    SettingKey::C4EnergyConsumption,
    SettingKey::JadeEnergyConsumption,
    SettingKey::GrassEnergyConsumption,
    SettingKey::PurpleEnergyConsumption,
    SettingKey::BarkEnergyConsumption,
    SettingKey::VioletEnergyConsumption,
    SettingKey::TealEnergyConsumption,
    SettingKey::EyeEnergyConsumption,
    SettingKey::MaroonEnergyConsumption,
    SettingKey::OliveEnergyConsumption,
    SettingKey::MintEnergyConsumption,
    SettingKey::CreamEnergyConsumption,
    SettingKey::RoseEnergyConsumption,
    SettingKey::DarkEnergyConsumption,
    SettingKey::OchreEnergyConsumption,
    SettingKey::SkyEnergyConsumption,
    SettingKey::LilacEnergyConsumption,
    SettingKey::SilverEnergyConsumption,
    SettingKey::FireEnergyConsumption,
    SettingKey::DarkgrayEnergyConsumption,
    SettingKey::GoldDivisor,
    SettingKey::BlondEnergyConsumption,
    SettingKey::FlowerEnergyConsumption,
    SettingKey::AuburnEnergyConsumption,
    SettingKey::PlagueEnergyConsumption,
    SettingKey::SpikeEnergyConsumption,
    SettingKey::IndigoEnergyConsumption,
    SettingKey::LavenderEnergyConsumption,
    SettingKey::FallowEnergyConsumption,
    SettingKey::SporeEnergyConsumption,
    SettingKey::RedProb,
    SettingKey::GreenProb,
    SettingKey::BlueProb,
    SettingKey::CyanProb,
    SettingKey::WhiteProb,
    SettingKey::GrayProb,
    SettingKey::YellowProb,
    SettingKey::MagentaProb,
    SettingKey::PinkProb,
    SettingKey::CoralProb,
    SettingKey::OrangeProb,
    SettingKey::ForestProb,
    SettingKey::SpringProb,
    SettingKey::LeafProb,
    SettingKey::SummerProb,
    SettingKey::LimeProb,
    SettingKey::C4Prob,
    SettingKey::JadeProb,
    SettingKey::GrassProb,
    SettingKey::PurpleProb,
    SettingKey::BarkProb,
    SettingKey::VioletProb,
    SettingKey::TealProb,
    SettingKey::EyeProb,
    SettingKey::MaroonProb,
    SettingKey::OliveProb,
    SettingKey::MintProb,
    SettingKey::CreamProb,
    SettingKey::RoseProb,
    SettingKey::DarkProb,
    SettingKey::OchreProb,
    SettingKey::SkyProb,
    SettingKey::LilacProb,
    SettingKey::SilverProb,
    SettingKey::FireProb,
    SettingKey::DarkgrayProb,
    SettingKey::GoldProb,
    SettingKey::BlondProb,
    SettingKey::FlowerProb,
    SettingKey::AuburnProb,
    SettingKey::PlagueProb,
    SettingKey::SpikeProb,
    SettingKey::IndigoProb,
    SettingKey::LavenderProb,
    SettingKey::FallowProb,
    SettingKey::SporeProb,
    SettingKey::SegmentCostDivisor,
    SettingKey::DrainSubsDivisor,
    SettingKey::GreenObtainedEnergyDivisor,
    SettingKey::OrganicObtainedEnergy,
    SettingKey::OrganicSubsProduced,
    SettingKey::CreamOrganicSubsProduced,
    SettingKey::Healing,
    SettingKey::ImmuneSystem,
    SettingKey::LavenderShield,
    SettingKey::IndigoDivisor,
    SettingKey::MosquitoEnergyConsumption,
    SettingKey::CrowdedlimeEnergyConsumption,
    SettingKey::CrowdedforestEnergyConsumption,
    SettingKey::SymbiontEnergyConsumption,
    SettingKey::ExperienceEnergyConsumption,
    SettingKey::DodgeEnergyConsumption,
    SettingKey::DarkjadeDelay,
    SettingKey::VirusEnergyConsumption,
    SettingKey::ScourgeEnergyConsumption,
    SettingKey::DecayEnergy,
];

impl SettingKey {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::AcceptConnections => "ACCEPT_CONNECTIONS",
            Self::AgeDivisor => "AGE_DIVISOR",
            Self::AuburnEnergyConsumption => "AUBURN_ENERGY_CONSUMPTION",
            Self::AuburnProb => "AUBURN_PROB",
            Self::AutoBackup => "AUTO_BACKUP",
            Self::BackupDelay => "BACKUP_DELAY",
            Self::BarkEnergyConsumption => "BARK_ENERGY_CONSUMPTION",
            Self::BarkProb => "BARK_PROB",
            Self::BlondEnergyConsumption => "BLOND_ENERGY_CONSUMPTION",
            Self::BlondProb => "BLOND_PROB",
            Self::BlueEnergyConsumption => "BLUE_ENERGY_CONSUMPTION",
            Self::BlueProb => "BLUE_PROB",
            Self::C4EnergyConsumption => "C4_ENERGY_CONSUMPTION",
            Self::C4Prob => "C4_PROB",
            Self::Ch4ToCo2Divisor => "CH4_TO_CO2_DIVISOR",
            Self::CladeComplexity => "CLADE_COMPLEXITY",
            Self::Co2ToCh4Divisor => "CO2_TO_CH4_DIVISOR",
            Self::ConnectToServer => "CONNECT_TO_SERVER",
            Self::CoralEnergyConsumption => "CORAL_ENERGY_CONSUMPTION",
            Self::CoralProb => "CORAL_PROB",
            Self::CreamEnergyConsumption => "CREAM_ENERGY_CONSUMPTION",
            Self::CreamOrganicSubsProduced => "CREAM_ORGANIC_SUBS_PRODUCED",
            Self::CreamProb => "CREAM_PROB",
            Self::CrowdedforestEnergyConsumption => "CROWDEDFOREST_ENERGY_CONSUMPTION",
            Self::CrowdedlimeEnergyConsumption => "CROWDEDLIME_ENERGY_CONSUMPTION",
            Self::CyanEnergyConsumption => "CYAN_ENERGY_CONSUMPTION",
            Self::CyanProb => "CYAN_PROB",
            Self::DarkgrayEnergyConsumption => "DARKGRAY_ENERGY_CONSUMPTION",
            Self::DarkgrayProb => "DARKGRAY_PROB",
            Self::DarkjadeDelay => "DARKJADE_DELAY",
            Self::DarkEnergyConsumption => "DARK_ENERGY_CONSUMPTION",
            Self::DarkProb => "DARK_PROB",
            Self::DecayEnergy => "DECAY_ENERGY",
            Self::Delay => "DELAY",
            Self::DodgeEnergyConsumption => "DODGE_ENERGY_CONSUMPTION",
            Self::DrainSubsDivisor => "DRAIN_SUBS_DIVISOR",
            Self::Elasticity => "ELASTICITY",
            Self::ExperienceEnergyConsumption => "EXPERIENCE_ENERGY_CONSUMPTION",
            Self::EyeEnergyConsumption => "EYE_ENERGY_CONSUMPTION",
            Self::EyeProb => "EYE_PROB",
            Self::FallowEnergyConsumption => "FALLOW_ENERGY_CONSUMPTION",
            Self::FallowProb => "FALLOW_PROB",
            Self::FireEnergyConsumption => "FIRE_ENERGY_CONSUMPTION",
            Self::FireProb => "FIRE_PROB",
            Self::FlowerEnergyConsumption => "FLOWER_ENERGY_CONSUMPTION",
            Self::FlowerProb => "FLOWER_PROB",
            Self::ForestEnergyConsumption => "FOREST_ENERGY_CONSUMPTION",
            Self::ForestProb => "FOREST_PROB",
            Self::GoldDivisor => "GOLD_DIVISOR",
            Self::GoldProb => "GOLD_PROB",
            Self::GrassEnergyConsumption => "GRASS_ENERGY_CONSUMPTION",
            Self::GrassProb => "GRASS_PROB",
            Self::GrayEnergyConsumption => "GRAY_ENERGY_CONSUMPTION",
            Self::GrayProb => "GRAY_PROB",
            Self::GreenEnergyConsumption => "GREEN_ENERGY_CONSUMPTION",
            Self::GreenObtainedEnergyDivisor => "GREEN_OBTAINED_ENERGY_DIVISOR",
            Self::GreenProb => "GREEN_PROB",
            Self::HardwareAcceleration => "HARDWARE_ACCELERATION",
            Self::Healing => "HEALING",
            Self::ImmuneSystem => "IMMUNE_SYSTEM",
            Self::IndigoDivisor => "INDIGO_DIVISOR",
            Self::IndigoEnergyConsumption => "INDIGO_ENERGY_CONSUMPTION",
            Self::IndigoProb => "INDIGO_PROB",
            Self::InitialCh4 => "INITIAL_CH4",
            Self::InitialCo2 => "INITIAL_CO2",
            Self::InitialComplexity => "INITIAL_COMPLEXITY",
            Self::InitialEnergy => "INITIAL_ENERGY",
            Self::InitialO2 => "INITIAL_O2",
            Self::InitialOrganisms => "INITIAL_ORGANISMS",
            Self::JadeEnergyConsumption => "JADE_ENERGY_CONSUMPTION",
            Self::JadeProb => "JADE_PROB",
            Self::LavenderEnergyConsumption => "LAVENDER_ENERGY_CONSUMPTION",
            Self::LavenderProb => "LAVENDER_PROB",
            Self::LavenderShield => "LAVENDER_SHIELD",
            Self::LeafEnergyConsumption => "LEAF_ENERGY_CONSUMPTION",
            Self::LeafProb => "LEAF_PROB",
            Self::LilacEnergyConsumption => "LILAC_ENERGY_CONSUMPTION",
            Self::LilacProb => "LILAC_PROB",
            Self::LimeEnergyConsumption => "LIME_ENERGY_CONSUMPTION",
            Self::LimeProb => "LIME_PROB",
            Self::Locale => "LOCALE",
            Self::LocalPort => "LOCAL_PORT",
            Self::MagentaEnergyConsumption => "MAGENTA_ENERGY_CONSUMPTION",
            Self::MagentaProb => "MAGENTA_PROB",
            Self::MaroonEnergyConsumption => "MAROON_ENERGY_CONSUMPTION",
            Self::MaroonProb => "MAROON_PROB",
            Self::MaxAge => "MAX_AGE",
            Self::MaxConnections => "MAX_CONNECTIONS",
            Self::MaxMutationRate => "MAX_MUTATION_RATE",
            Self::MaxRot => "MAX_ROT",
            Self::MaxVel => "MAX_VEL",
            Self::MetaMutationRate => "META_MUTATION_RATE",
            Self::MintEnergyConsumption => "MINT_ENERGY_CONSUMPTION",
            Self::MintProb => "MINT_PROB",
            Self::MinMutationRate => "MIN_MUTATION_RATE",
            Self::MosquitoEnergyConsumption => "MOSQUITO_ENERGY_CONSUMPTION",
            Self::OchreEnergyConsumption => "OCHRE_ENERGY_CONSUMPTION",
            Self::OchreProb => "OCHRE_PROB",
            Self::OliveEnergyConsumption => "OLIVE_ENERGY_CONSUMPTION",
            Self::OliveProb => "OLIVE_PROB",
            Self::OrangeEnergyConsumption => "ORANGE_ENERGY_CONSUMPTION",
            Self::OrangeProb => "ORANGE_PROB",
            Self::OrganicObtainedEnergy => "ORGANIC_OBTAINED_ENERGY",
            Self::OrganicSubsProduced => "ORGANIC_SUBS_PRODUCED",
            Self::OrganismsVectorSize => "ORGANISMS_VECTOR_SIZE",
            Self::PinkEnergyConsumption => "PINK_ENERGY_CONSUMPTION",
            Self::PinkProb => "PINK_PROB",
            Self::PlagueEnergyConsumption => "PLAGUE_ENERGY_CONSUMPTION",
            Self::PlagueProb => "PLAGUE_PROB",
            Self::PurpleEnergyConsumption => "PURPLE_ENERGY_CONSUMPTION",
            Self::PurpleProb => "PURPLE_PROB",
            Self::RedEnergyConsumption => "RED_ENERGY_CONSUMPTION",
            Self::RedProb => "RED_PROB",
            Self::RoseEnergyConsumption => "ROSE_ENERGY_CONSUMPTION",
            Self::RoseProb => "ROSE_PROB",
            Self::Rubbing => "RUBBING",
            Self::ScourgeEnergyConsumption => "SCOURGE_ENERGY_CONSUMPTION",
            Self::SegmentCostDivisor => "SEGMENT_COST_DIVISOR",
            Self::ServerAddress => "SERVER_ADDRESS",
            Self::ServerPort => "SERVER_PORT",
            Self::SilverEnergyConsumption => "SILVER_ENERGY_CONSUMPTION",
            Self::SilverProb => "SILVER_PROB",
            Self::SkyEnergyConsumption => "SKY_ENERGY_CONSUMPTION",
            Self::SkyProb => "SKY_PROB",
            Self::SpikeEnergyConsumption => "SPIKE_ENERGY_CONSUMPTION",
            Self::SpikeProb => "SPIKE_PROB",
            Self::SporeEnergyConsumption => "SPORE_ENERGY_CONSUMPTION",
            Self::SporeProb => "SPORE_PROB",
            Self::SpringEnergyConsumption => "SPRING_ENERGY_CONSUMPTION",
            Self::SpringProb => "SPRING_PROB",
            Self::SummerEnergyConsumption => "SUMMER_ENERGY_CONSUMPTION",
            Self::SummerProb => "SUMMER_PROB",
            Self::SymbiontEnergyConsumption => "SYMBIONT_ENERGY_CONSUMPTION",
            Self::TealEnergyConsumption => "TEAL_ENERGY_CONSUMPTION",
            Self::TealProb => "TEAL_PROB",
            Self::UserName => "USER_NAME",
            Self::VioletEnergyConsumption => "VIOLET_ENERGY_CONSUMPTION",
            Self::VioletProb => "VIOLET_PROB",
            Self::VirusEnergyConsumption => "VIRUS_ENERGY_CONSUMPTION",
            Self::WhiteEnergyConsumption => "WHITE_ENERGY_CONSUMPTION",
            Self::WhiteProb => "WHITE_PROB",
            Self::WindowHeight => "WINDOW_HEIGHT",
            Self::WindowState => "WINDOW_STATE",
            Self::WindowWidth => "WINDOW_WIDTH",
            Self::WindowX => "WINDOW_X",
            Self::WindowY => "WINDOW_Y",
            Self::WorldHeight => "WORLD_HEIGHT",
            Self::WorldWidth => "WORLD_WIDTH",
            Self::YellowEnergyConsumption => "YELLOW_ENERGY_CONSUMPTION",
            Self::YellowProb => "YELLOW_PROB",
            Self::LocaleIndex => "LOCALE_INDEX",
        }
    }

    pub fn from_source_name(name: &str) -> Option<Self> {
        match name {
            "ACCEPT_CONNECTIONS" => Some(Self::AcceptConnections),
            "AGE_DIVISOR" => Some(Self::AgeDivisor),
            "AUBURN_ENERGY_CONSUMPTION" => Some(Self::AuburnEnergyConsumption),
            "AUBURN_PROB" => Some(Self::AuburnProb),
            "AUTO_BACKUP" => Some(Self::AutoBackup),
            "BACKUP_DELAY" => Some(Self::BackupDelay),
            "BARK_ENERGY_CONSUMPTION" => Some(Self::BarkEnergyConsumption),
            "BARK_PROB" => Some(Self::BarkProb),
            "BLOND_ENERGY_CONSUMPTION" => Some(Self::BlondEnergyConsumption),
            "BLOND_PROB" => Some(Self::BlondProb),
            "BLUE_ENERGY_CONSUMPTION" => Some(Self::BlueEnergyConsumption),
            "BLUE_PROB" => Some(Self::BlueProb),
            "C4_ENERGY_CONSUMPTION" => Some(Self::C4EnergyConsumption),
            "C4_PROB" => Some(Self::C4Prob),
            "CH4_TO_CO2_DIVISOR" => Some(Self::Ch4ToCo2Divisor),
            "CLADE_COMPLEXITY" => Some(Self::CladeComplexity),
            "CO2_TO_CH4_DIVISOR" => Some(Self::Co2ToCh4Divisor),
            "CONNECT_TO_SERVER" => Some(Self::ConnectToServer),
            "CORAL_ENERGY_CONSUMPTION" => Some(Self::CoralEnergyConsumption),
            "CORAL_PROB" => Some(Self::CoralProb),
            "CREAM_ENERGY_CONSUMPTION" => Some(Self::CreamEnergyConsumption),
            "CREAM_ORGANIC_SUBS_PRODUCED" => Some(Self::CreamOrganicSubsProduced),
            "CREAM_PROB" => Some(Self::CreamProb),
            "CROWDEDFOREST_ENERGY_CONSUMPTION" => Some(Self::CrowdedforestEnergyConsumption),
            "CROWDEDLIME_ENERGY_CONSUMPTION" => Some(Self::CrowdedlimeEnergyConsumption),
            "CYAN_ENERGY_CONSUMPTION" => Some(Self::CyanEnergyConsumption),
            "CYAN_PROB" => Some(Self::CyanProb),
            "DARKGRAY_ENERGY_CONSUMPTION" => Some(Self::DarkgrayEnergyConsumption),
            "DARKGRAY_PROB" => Some(Self::DarkgrayProb),
            "DARKJADE_DELAY" => Some(Self::DarkjadeDelay),
            "DARK_ENERGY_CONSUMPTION" => Some(Self::DarkEnergyConsumption),
            "DARK_PROB" => Some(Self::DarkProb),
            "DECAY_ENERGY" => Some(Self::DecayEnergy),
            "DELAY" => Some(Self::Delay),
            "DODGE_ENERGY_CONSUMPTION" => Some(Self::DodgeEnergyConsumption),
            "DRAIN_SUBS_DIVISOR" => Some(Self::DrainSubsDivisor),
            "ELASTICITY" => Some(Self::Elasticity),
            "EXPERIENCE_ENERGY_CONSUMPTION" => Some(Self::ExperienceEnergyConsumption),
            "EYE_ENERGY_CONSUMPTION" => Some(Self::EyeEnergyConsumption),
            "EYE_PROB" => Some(Self::EyeProb),
            "FALLOW_ENERGY_CONSUMPTION" => Some(Self::FallowEnergyConsumption),
            "FALLOW_PROB" => Some(Self::FallowProb),
            "FIRE_ENERGY_CONSUMPTION" => Some(Self::FireEnergyConsumption),
            "FIRE_PROB" => Some(Self::FireProb),
            "FLOWER_ENERGY_CONSUMPTION" => Some(Self::FlowerEnergyConsumption),
            "FLOWER_PROB" => Some(Self::FlowerProb),
            "FOREST_ENERGY_CONSUMPTION" => Some(Self::ForestEnergyConsumption),
            "FOREST_PROB" => Some(Self::ForestProb),
            "GOLD_DIVISOR" => Some(Self::GoldDivisor),
            "GOLD_PROB" => Some(Self::GoldProb),
            "GRASS_ENERGY_CONSUMPTION" => Some(Self::GrassEnergyConsumption),
            "GRASS_PROB" => Some(Self::GrassProb),
            "GRAY_ENERGY_CONSUMPTION" => Some(Self::GrayEnergyConsumption),
            "GRAY_PROB" => Some(Self::GrayProb),
            "GREEN_ENERGY_CONSUMPTION" => Some(Self::GreenEnergyConsumption),
            "GREEN_OBTAINED_ENERGY_DIVISOR" => Some(Self::GreenObtainedEnergyDivisor),
            "GREEN_PROB" => Some(Self::GreenProb),
            "HARDWARE_ACCELERATION" => Some(Self::HardwareAcceleration),
            "HEALING" => Some(Self::Healing),
            "IMMUNE_SYSTEM" => Some(Self::ImmuneSystem),
            "INDIGO_DIVISOR" => Some(Self::IndigoDivisor),
            "INDIGO_ENERGY_CONSUMPTION" => Some(Self::IndigoEnergyConsumption),
            "INDIGO_PROB" => Some(Self::IndigoProb),
            "INITIAL_CH4" => Some(Self::InitialCh4),
            "INITIAL_CO2" => Some(Self::InitialCo2),
            "INITIAL_COMPLEXITY" => Some(Self::InitialComplexity),
            "INITIAL_ENERGY" => Some(Self::InitialEnergy),
            "INITIAL_O2" => Some(Self::InitialO2),
            "INITIAL_ORGANISMS" => Some(Self::InitialOrganisms),
            "JADE_ENERGY_CONSUMPTION" => Some(Self::JadeEnergyConsumption),
            "JADE_PROB" => Some(Self::JadeProb),
            "LAVENDER_ENERGY_CONSUMPTION" => Some(Self::LavenderEnergyConsumption),
            "LAVENDER_PROB" => Some(Self::LavenderProb),
            "LAVENDER_SHIELD" => Some(Self::LavenderShield),
            "LEAF_ENERGY_CONSUMPTION" => Some(Self::LeafEnergyConsumption),
            "LEAF_PROB" => Some(Self::LeafProb),
            "LILAC_ENERGY_CONSUMPTION" => Some(Self::LilacEnergyConsumption),
            "LILAC_PROB" => Some(Self::LilacProb),
            "LIME_ENERGY_CONSUMPTION" => Some(Self::LimeEnergyConsumption),
            "LIME_PROB" => Some(Self::LimeProb),
            "LOCALE" => Some(Self::Locale),
            "LOCAL_PORT" => Some(Self::LocalPort),
            "MAGENTA_ENERGY_CONSUMPTION" => Some(Self::MagentaEnergyConsumption),
            "MAGENTA_PROB" => Some(Self::MagentaProb),
            "MAROON_ENERGY_CONSUMPTION" => Some(Self::MaroonEnergyConsumption),
            "MAROON_PROB" => Some(Self::MaroonProb),
            "MAX_AGE" => Some(Self::MaxAge),
            "MAX_CONNECTIONS" => Some(Self::MaxConnections),
            "MAX_MUTATION_RATE" => Some(Self::MaxMutationRate),
            "MAX_ROT" => Some(Self::MaxRot),
            "MAX_VEL" => Some(Self::MaxVel),
            "META_MUTATION_RATE" => Some(Self::MetaMutationRate),
            "MINT_ENERGY_CONSUMPTION" => Some(Self::MintEnergyConsumption),
            "MINT_PROB" => Some(Self::MintProb),
            "MIN_MUTATION_RATE" => Some(Self::MinMutationRate),
            "MOSQUITO_ENERGY_CONSUMPTION" => Some(Self::MosquitoEnergyConsumption),
            "OCHRE_ENERGY_CONSUMPTION" => Some(Self::OchreEnergyConsumption),
            "OCHRE_PROB" => Some(Self::OchreProb),
            "OLIVE_ENERGY_CONSUMPTION" => Some(Self::OliveEnergyConsumption),
            "OLIVE_PROB" => Some(Self::OliveProb),
            "ORANGE_ENERGY_CONSUMPTION" => Some(Self::OrangeEnergyConsumption),
            "ORANGE_PROB" => Some(Self::OrangeProb),
            "ORGANIC_OBTAINED_ENERGY" => Some(Self::OrganicObtainedEnergy),
            "ORGANIC_SUBS_PRODUCED" => Some(Self::OrganicSubsProduced),
            "ORGANISMS_VECTOR_SIZE" => Some(Self::OrganismsVectorSize),
            "PINK_ENERGY_CONSUMPTION" => Some(Self::PinkEnergyConsumption),
            "PINK_PROB" => Some(Self::PinkProb),
            "PLAGUE_ENERGY_CONSUMPTION" => Some(Self::PlagueEnergyConsumption),
            "PLAGUE_PROB" => Some(Self::PlagueProb),
            "PURPLE_ENERGY_CONSUMPTION" => Some(Self::PurpleEnergyConsumption),
            "PURPLE_PROB" => Some(Self::PurpleProb),
            "RED_ENERGY_CONSUMPTION" => Some(Self::RedEnergyConsumption),
            "RED_PROB" => Some(Self::RedProb),
            "ROSE_ENERGY_CONSUMPTION" => Some(Self::RoseEnergyConsumption),
            "ROSE_PROB" => Some(Self::RoseProb),
            "RUBBING" => Some(Self::Rubbing),
            "SCOURGE_ENERGY_CONSUMPTION" => Some(Self::ScourgeEnergyConsumption),
            "SEGMENT_COST_DIVISOR" => Some(Self::SegmentCostDivisor),
            "SERVER_ADDRESS" => Some(Self::ServerAddress),
            "SERVER_PORT" => Some(Self::ServerPort),
            "SILVER_ENERGY_CONSUMPTION" => Some(Self::SilverEnergyConsumption),
            "SILVER_PROB" => Some(Self::SilverProb),
            "SKY_ENERGY_CONSUMPTION" => Some(Self::SkyEnergyConsumption),
            "SKY_PROB" => Some(Self::SkyProb),
            "SPIKE_ENERGY_CONSUMPTION" => Some(Self::SpikeEnergyConsumption),
            "SPIKE_PROB" => Some(Self::SpikeProb),
            "SPORE_ENERGY_CONSUMPTION" => Some(Self::SporeEnergyConsumption),
            "SPORE_PROB" => Some(Self::SporeProb),
            "SPRING_ENERGY_CONSUMPTION" => Some(Self::SpringEnergyConsumption),
            "SPRING_PROB" => Some(Self::SpringProb),
            "SUMMER_ENERGY_CONSUMPTION" => Some(Self::SummerEnergyConsumption),
            "SUMMER_PROB" => Some(Self::SummerProb),
            "SYMBIONT_ENERGY_CONSUMPTION" => Some(Self::SymbiontEnergyConsumption),
            "TEAL_ENERGY_CONSUMPTION" => Some(Self::TealEnergyConsumption),
            "TEAL_PROB" => Some(Self::TealProb),
            "USER_NAME" => Some(Self::UserName),
            "VIOLET_ENERGY_CONSUMPTION" => Some(Self::VioletEnergyConsumption),
            "VIOLET_PROB" => Some(Self::VioletProb),
            "VIRUS_ENERGY_CONSUMPTION" => Some(Self::VirusEnergyConsumption),
            "WHITE_ENERGY_CONSUMPTION" => Some(Self::WhiteEnergyConsumption),
            "WHITE_PROB" => Some(Self::WhiteProb),
            "WINDOW_HEIGHT" => Some(Self::WindowHeight),
            "WINDOW_STATE" => Some(Self::WindowState),
            "WINDOW_WIDTH" => Some(Self::WindowWidth),
            "WINDOW_X" => Some(Self::WindowX),
            "WINDOW_Y" => Some(Self::WindowY),
            "WORLD_HEIGHT" => Some(Self::WorldHeight),
            "WORLD_WIDTH" => Some(Self::WorldWidth),
            "YELLOW_ENERGY_CONSUMPTION" => Some(Self::YellowEnergyConsumption),
            "YELLOW_PROB" => Some(Self::YellowProb),
            "LOCALE_INDEX" => Some(Self::LocaleIndex),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ApplicationSettings {
    accept_connections: bool,
    auto_backup: bool,
    automatic_backup_mode: AutomaticBackupMode,
    backup_delay: i32,
    connect_to_server: bool,
    crisp_lines: bool,
    delay: i32,
    hardware_acceleration: i32,
    locale: String,
    local_port: i32,
    max_connections: i32,
    organisms_vector_size: i32,
    server_address: String,
    server_port: i32,
    user_name: JavaText,
    window_height: i32,
    window_state: i32,
    window_width: i32,
    window_x: i32,
    window_y: i32,
}

impl Default for ApplicationSettings {
    fn default() -> Self {
        Self {
            accept_connections: false,
            auto_backup: false,
            automatic_backup_mode: AutomaticBackupMode::Adjacent,
            backup_delay: 100,
            connect_to_server: false,
            crisp_lines: false,
            delay: 2,
            hardware_acceleration: 0,
            locale: "en".to_owned(),
            local_port: 8888,
            max_connections: 1,
            organisms_vector_size: 50,
            server_address: "".to_owned(),
            server_port: 0,
            user_name: JavaText::from_utf8("ID"),
            window_height: 480,
            window_state: 0,
            window_width: 640,
            window_x: 0,
            window_y: 0,
        }
    }
}

/// Editable global settings; active World parameters are a separate runtime projection
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsState {
    parameters: WorldParameters,
    genesis: GenesisSettings,
    application: ApplicationSettings,
}

impl SettingsState {
    /// Language is caller-owned startup information, never a serialized session seed
    pub fn new(language: String) -> Self {
        let mut application = ApplicationSettings::default();
        application.locale = language;
        Self {
            parameters: WorldParameters::default(),
            genesis: GenesisSettings::default(),
            application,
        }
    }

    /// Read source network values without performing socket operations
    pub fn network_settings(&self) -> NetworkSettings {
        NetworkSettings {
            accept_connections: self.application.accept_connections,
            local_port: self.application.local_port,
            max_connections: self.application.max_connections,
            user_name: self.application.user_name.clone(),
        }
    }

    /// Apply independent source dialog fields without saving or restarting sockets
    /// Use before/after values to apply acceptance, save preferences, then restart the changed port
    pub fn apply_network_editor(
        &mut self,
        accept_connections: bool,
        local_port: &str,
        max_connections: &str,
        user_name: JavaText,
    ) -> NetworkSettingsApply {
        let before = self.network_settings();
        self.application.accept_connections = accept_connections;
        let apply_integer = |target: &mut i32, text: &str| {
            let Some(value) = java_int(text) else {
                return EditorOutcome::InvalidNumber;
            };
            if value < 0 {
                return EditorOutcome::OutsideRange;
            }
            *target = value;
            EditorOutcome::Applied
        };
        let local_port = apply_integer(&mut self.application.local_port, local_port);
        let max_connections = apply_integer(&mut self.application.max_connections, max_connections);
        let user_name_applied = user_name.units().len() <= 15;
        if user_name_applied {
            self.application.user_name = user_name;
        }
        let after = self.network_settings();
        NetworkSettingsApply { before, after, local_port, max_connections, user_name_applied }
    }

    /// Native display choice, independent of the source setting-key enumeration
    pub fn crisp_lines(&self) -> bool {
        self.application.crisp_lines
    }

    pub fn set_crisp_lines(&mut self, enabled: bool) {
        self.application.crisp_lines = enabled;
    }

    pub fn automatic_backup_mode(&self) -> AutomaticBackupMode {
        self.application.automatic_backup_mode
    }

    pub fn set_automatic_backup_mode(&mut self, mode: AutomaticBackupMode) {
        self.application.automatic_backup_mode = mode;
    }

    /// Record actual x/y/width/height/source-style state after validating the full batch
    pub fn record_window_geometry(&mut self, geometry: [i32; 5]) -> Result<(), SettingsError> {
        if geometry[2] <= 0 || geometry[3] <= 0 {
            return Err(SettingsError::InvalidWindowGeometry);
        }
        self.application.window_x = geometry[0];
        self.application.window_y = geometry[1];
        self.application.window_width = geometry[2];
        self.application.window_height = geometry[3];
        self.application.window_state = geometry[4];
        Ok(())
    }

    pub fn parameters(&self) -> &WorldParameters {
        &self.parameters
    }
    pub fn genesis(&self) -> GenesisSettings {
        self.genesis
    }
    pub fn language(&self) -> &str {
        &self.application.locale
    }
    pub fn locale_index(&self) -> usize {
        LANGUAGE_CODES
            .iter()
            .position(|language| *language == self.language())
            .unwrap_or(1)
    }

    /// Project pending dimensions while retaining the caller's current seed provenance
    pub fn world_config(&self, seed: i64) -> Result<WorldConfig, SettingsError> {
        let dimension = |value: i32| (value > 0).then_some(value);
        Ok(WorldConfig {
            seed,
            width: dimension(self.genesis.width).ok_or(SettingsError::InvalidGenesis)?,
            height: dimension(self.genesis.height).ok_or(SettingsError::InvalidGenesis)?,
            initial_attempts: u32::try_from(self.genesis.initial_attempts)
                .map_err(|_| SettingsError::InvalidGenesis)?,
        })
    }

    /// Record only configuration that entered actual genesis; never record a session seed
    pub fn record_applied_genesis(
        &mut self,
        genesis: GenesisSettings,
    ) -> Result<(), SettingsError> {
        if genesis.width <= 0 || genesis.height <= 0 || genesis.initial_attempts < 0 {
            return Err(SettingsError::InvalidGenesis);
        }
        self.genesis = genesis;
        Ok(())
    }

    pub fn get(&self, key: SettingKey) -> SettingValue {
        match key {
            SettingKey::AcceptConnections => {
                SettingValue::Boolean(self.application.accept_connections)
            }
            SettingKey::AgeDivisor => SettingValue::Int(self.parameters.effects.age_divisor),
            SettingKey::AuburnEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.auburn_energy_consumption)
            }
            SettingKey::AuburnProb => SettingValue::Int(self.parameters.genetics.color_weights[34]),
            SettingKey::AutoBackup => SettingValue::Boolean(self.application.auto_backup),
            SettingKey::BackupDelay => SettingValue::Int(self.application.backup_delay),
            SettingKey::BarkEnergyConsumption => SettingValue::Double(self.parameters.effects.bark),
            SettingKey::BarkProb => SettingValue::Int(self.parameters.genetics.color_weights[15]),
            SettingKey::BlondEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.blond_energy_consumption)
            }
            SettingKey::BlondProb => SettingValue::Int(self.parameters.genetics.color_weights[32]),
            SettingKey::BlueEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.blue_energy_consumption)
            }
            SettingKey::BlueProb => SettingValue::Int(self.parameters.genetics.color_weights[2]),
            SettingKey::C4EnergyConsumption => SettingValue::Double(self.parameters.effects.c4),
            SettingKey::C4Prob => SettingValue::Int(self.parameters.genetics.color_weights[42]),
            SettingKey::Ch4ToCo2Divisor => {
                SettingValue::Int(self.parameters.atmosphere.methane_to_carbon_divisor)
            }
            SettingKey::CladeComplexity => {
                SettingValue::Int(self.parameters.genetics.clade_complexity)
            }
            SettingKey::Co2ToCh4Divisor => {
                SettingValue::Int(self.parameters.atmosphere.carbon_to_methane_divisor)
            }
            SettingKey::ConnectToServer => {
                SettingValue::Boolean(self.application.connect_to_server)
            }
            SettingKey::CoralEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.coral_energy_consumption)
            }
            SettingKey::CoralProb => SettingValue::Int(self.parameters.genetics.color_weights[8]),
            SettingKey::CreamEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.cream_energy_consumption)
            }
            SettingKey::CreamOrganicSubsProduced => {
                SettingValue::Double(self.parameters.lifecycle.cream_organic_subs_produced)
            }
            SettingKey::CreamProb => SettingValue::Int(self.parameters.genetics.color_weights[22]),
            SettingKey::CrowdedforestEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.crowded_forest_energy_consumption)
            }
            SettingKey::CrowdedlimeEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.crowded_lime_energy_consumption)
            }
            SettingKey::CyanEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.cyan_energy_consumption)
            }
            SettingKey::CyanProb => SettingValue::Int(self.parameters.genetics.color_weights[3]),
            SettingKey::DarkgrayEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.dark_gray_energy_consumption)
            }
            SettingKey::DarkgrayProb => {
                SettingValue::Int(self.parameters.genetics.color_weights[30])
            }
            SettingKey::DarkjadeDelay => SettingValue::Int(self.parameters.effects.dark_jade_delay),
            SettingKey::DarkEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.dark_energy_consumption)
            }
            SettingKey::DarkProb => SettingValue::Int(self.parameters.genetics.color_weights[24]),
            SettingKey::DecayEnergy => SettingValue::Double(self.parameters.lifecycle.decay_energy),
            SettingKey::Delay => SettingValue::Int(self.application.delay),
            SettingKey::DodgeEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.dodge_energy_consumption)
            }
            SettingKey::DrainSubsDivisor => {
                SettingValue::Int(self.parameters.atmosphere.drain_divisor)
            }
            SettingKey::Elasticity => SettingValue::Double(self.parameters.physics.elasticity),
            SettingKey::ExperienceEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.experience_energy_consumption)
            }
            SettingKey::EyeEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.eye_energy_consumption)
            }
            SettingKey::EyeProb => SettingValue::Int(self.parameters.genetics.color_weights[18]),
            SettingKey::FallowEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.fallow_energy_consumption)
            }
            SettingKey::FallowProb => SettingValue::Int(self.parameters.genetics.color_weights[39]),
            SettingKey::FireEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.fire_energy_consumption)
            }
            SettingKey::FireProb => SettingValue::Int(self.parameters.genetics.color_weights[29]),
            SettingKey::FlowerEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.flower_energy_consumption)
            }
            SettingKey::FlowerProb => SettingValue::Int(self.parameters.genetics.color_weights[33]),
            SettingKey::ForestEnergyConsumption => {
                SettingValue::Double(self.parameters.effects.forest)
            }
            SettingKey::ForestProb => SettingValue::Int(self.parameters.genetics.color_weights[10]),
            SettingKey::GoldDivisor => SettingValue::Double(self.parameters.effects.gold_divisor),
            SettingKey::GoldProb => SettingValue::Int(self.parameters.genetics.color_weights[31]),
            SettingKey::GrassEnergyConsumption => {
                SettingValue::Double(self.parameters.effects.grass)
            }
            SettingKey::GrassProb => SettingValue::Int(self.parameters.genetics.color_weights[43]),
            SettingKey::GrayEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.gray_energy_consumption)
            }
            SettingKey::GrayProb => SettingValue::Int(self.parameters.genetics.color_weights[5]),
            SettingKey::GreenEnergyConsumption => {
                SettingValue::Double(self.parameters.effects.green)
            }
            SettingKey::GreenObtainedEnergyDivisor => {
                SettingValue::Int(self.parameters.effects.green_energy_divisor)
            }
            SettingKey::GreenProb => SettingValue::Int(self.parameters.genetics.color_weights[1]),
            SettingKey::HardwareAcceleration => {
                SettingValue::Int(self.application.hardware_acceleration)
            }
            SettingKey::Healing => SettingValue::Int(self.parameters.lifecycle.healing),
            SettingKey::ImmuneSystem => SettingValue::Int(self.parameters.lifecycle.immune_system),
            SettingKey::IndigoDivisor => {
                SettingValue::Double(self.parameters.effects.indigo_divisor)
            }
            SettingKey::IndigoEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.indigo_energy_consumption)
            }
            SettingKey::IndigoProb => SettingValue::Int(self.parameters.genetics.color_weights[37]),
            SettingKey::InitialCh4 => {
                SettingValue::Double(self.parameters.atmosphere.initial_methane)
            }
            SettingKey::InitialCo2 => {
                SettingValue::Double(self.parameters.atmosphere.initial_carbon_dioxide)
            }
            SettingKey::InitialComplexity => {
                SettingValue::Int(self.parameters.genetics.initial_complexity)
            }
            SettingKey::InitialEnergy => SettingValue::Int(self.parameters.initial_energy as i32),
            SettingKey::InitialO2 => {
                SettingValue::Double(self.parameters.atmosphere.initial_oxygen)
            }
            SettingKey::InitialOrganisms => SettingValue::Int(self.genesis.initial_attempts),
            SettingKey::JadeEnergyConsumption => SettingValue::Double(self.parameters.effects.jade),
            SettingKey::JadeProb => SettingValue::Int(self.parameters.genetics.color_weights[41]),
            SettingKey::LavenderEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.lavender_energy_consumption)
            }
            SettingKey::LavenderProb => {
                SettingValue::Int(self.parameters.genetics.color_weights[38])
            }
            SettingKey::LavenderShield => {
                SettingValue::Int(self.parameters.effects.lavender_shield)
            }
            SettingKey::LeafEnergyConsumption => SettingValue::Double(self.parameters.effects.leaf),
            SettingKey::LeafProb => SettingValue::Int(self.parameters.genetics.color_weights[12]),
            SettingKey::LilacEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.lilac_energy_consumption)
            }
            SettingKey::LilacProb => SettingValue::Int(self.parameters.genetics.color_weights[27]),
            SettingKey::LimeEnergyConsumption => SettingValue::Double(self.parameters.effects.lime),
            SettingKey::LimeProb => SettingValue::Int(self.parameters.genetics.color_weights[14]),
            SettingKey::Locale => SettingValue::Text(self.application.locale.clone()),
            SettingKey::LocalPort => SettingValue::Int(self.application.local_port),
            SettingKey::MagentaEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.magenta_energy_consumption)
            }
            SettingKey::MagentaProb => SettingValue::Int(self.parameters.genetics.color_weights[6]),
            SettingKey::MaroonEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.maroon_energy_consumption)
            }
            SettingKey::MaroonProb => SettingValue::Int(self.parameters.genetics.color_weights[19]),
            SettingKey::MaxAge => SettingValue::Int(self.parameters.effects.max_age),
            SettingKey::MaxConnections => SettingValue::Int(self.application.max_connections),
            SettingKey::MaxMutationRate => {
                SettingValue::Int(self.parameters.genetics.max_mutation_rate)
            }
            SettingKey::MaxRot => SettingValue::Double(self.parameters.physics.max_rotation),
            SettingKey::MaxVel => SettingValue::Double(self.parameters.physics.max_velocity),
            SettingKey::MetaMutationRate => {
                SettingValue::Int(self.parameters.genetics.meta_mutation_rate)
            }
            SettingKey::MintEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.mint_energy_consumption)
            }
            SettingKey::MintProb => SettingValue::Int(self.parameters.genetics.color_weights[21]),
            SettingKey::MinMutationRate => {
                SettingValue::Int(self.parameters.genetics.min_mutation_rate)
            }
            SettingKey::MosquitoEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.mosquito_energy_consumption)
            }
            SettingKey::OchreEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.ochre_energy_consumption)
            }
            SettingKey::OchreProb => SettingValue::Int(self.parameters.genetics.color_weights[25]),
            SettingKey::OliveEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.olive_energy_consumption)
            }
            SettingKey::OliveProb => SettingValue::Int(self.parameters.genetics.color_weights[20]),
            SettingKey::OrangeEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.orange_energy_consumption)
            }
            SettingKey::OrangeProb => SettingValue::Int(self.parameters.genetics.color_weights[9]),
            SettingKey::OrganicObtainedEnergy => {
                SettingValue::Double(self.parameters.lifecycle.organic_obtained_energy)
            }
            SettingKey::OrganicSubsProduced => {
                SettingValue::Double(self.parameters.lifecycle.organic_subs_produced)
            }
            SettingKey::OrganismsVectorSize => {
                SettingValue::Int(self.application.organisms_vector_size)
            }
            SettingKey::PinkEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.pink_energy_consumption)
            }
            SettingKey::PinkProb => SettingValue::Int(self.parameters.genetics.color_weights[7]),
            SettingKey::PlagueEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.plague_energy_consumption)
            }
            SettingKey::PlagueProb => SettingValue::Int(self.parameters.genetics.color_weights[35]),
            SettingKey::PurpleEnergyConsumption => {
                SettingValue::Double(self.parameters.effects.purple)
            }
            SettingKey::PurpleProb => SettingValue::Int(self.parameters.genetics.color_weights[44]),
            SettingKey::RedEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.red_energy_consumption)
            }
            SettingKey::RedProb => SettingValue::Int(self.parameters.genetics.color_weights[0]),
            SettingKey::RoseEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.rose_energy_consumption)
            }
            SettingKey::RoseProb => SettingValue::Int(self.parameters.genetics.color_weights[23]),
            SettingKey::Rubbing => SettingValue::Double(self.parameters.lifecycle.rubbing),
            SettingKey::ScourgeEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.scourge_energy_consumption)
            }
            SettingKey::SegmentCostDivisor => {
                SettingValue::Int(self.parameters.lifecycle.segment_cost_divisor)
            }
            SettingKey::ServerAddress => {
                SettingValue::Text(self.application.server_address.clone())
            }
            SettingKey::ServerPort => SettingValue::Int(self.application.server_port),
            SettingKey::SilverEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.silver_energy_consumption)
            }
            SettingKey::SilverProb => SettingValue::Int(self.parameters.genetics.color_weights[28]),
            SettingKey::SkyEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.sky_energy_consumption)
            }
            SettingKey::SkyProb => SettingValue::Int(self.parameters.genetics.color_weights[26]),
            SettingKey::SpikeEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.spike_energy_consumption)
            }
            SettingKey::SpikeProb => SettingValue::Int(self.parameters.genetics.color_weights[36]),
            SettingKey::SporeEnergyConsumption => {
                SettingValue::Double(self.parameters.spore_energy_consumption)
            }
            SettingKey::SporeProb => SettingValue::Int(self.parameters.genetics.color_weights[40]),
            SettingKey::SpringEnergyConsumption => {
                SettingValue::Double(self.parameters.effects.spring)
            }
            SettingKey::SpringProb => SettingValue::Int(self.parameters.genetics.color_weights[11]),
            SettingKey::SummerEnergyConsumption => {
                SettingValue::Double(self.parameters.effects.summer)
            }
            SettingKey::SummerProb => SettingValue::Int(self.parameters.genetics.color_weights[13]),
            SettingKey::SymbiontEnergyConsumption => {
                SettingValue::Double(self.parameters.effects.symbiont)
            }
            SettingKey::TealEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.teal_energy_consumption)
            }
            SettingKey::TealProb => SettingValue::Int(self.parameters.genetics.color_weights[17]),
            // The generic String view is for display; network and persistence retain raw units
            SettingKey::UserName => SettingValue::Text(self.application.user_name.display_utf8_lossy()),
            SettingKey::VioletEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.violet_energy_consumption)
            }
            SettingKey::VioletProb => SettingValue::Int(self.parameters.genetics.color_weights[16]),
            SettingKey::VirusEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.virus_energy_consumption)
            }
            SettingKey::WhiteEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.white_energy_consumption)
            }
            SettingKey::WhiteProb => SettingValue::Int(self.parameters.genetics.color_weights[4]),
            SettingKey::WindowHeight => SettingValue::Int(self.application.window_height),
            SettingKey::WindowState => SettingValue::Int(self.application.window_state),
            SettingKey::WindowWidth => SettingValue::Int(self.application.window_width),
            SettingKey::WindowX => SettingValue::Int(self.application.window_x),
            SettingKey::WindowY => SettingValue::Int(self.application.window_y),
            SettingKey::WorldHeight => SettingValue::Int(self.genesis.height),
            SettingKey::WorldWidth => SettingValue::Int(self.genesis.width),
            SettingKey::YellowEnergyConsumption => {
                SettingValue::Double(self.parameters.lifecycle.yellow_energy_consumption)
            }
            SettingKey::YellowProb => SettingValue::Int(self.parameters.genetics.color_weights[45]),
            SettingKey::LocaleIndex => SettingValue::Int(self.locale_index() as i32),
        }
    }

    fn assign(&mut self, key: SettingKey, value: SettingValue) -> Result<(), SettingsError> {
        match (key, value) {
            (SettingKey::AcceptConnections, SettingValue::Boolean(value)) => {
                self.application.accept_connections = value
            }
            (SettingKey::AgeDivisor, SettingValue::Int(value)) => {
                self.parameters.effects.age_divisor = value
            }
            (SettingKey::AuburnEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.auburn_energy_consumption = value
            }
            (SettingKey::AuburnProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[34] = value
            }
            (SettingKey::AutoBackup, SettingValue::Boolean(value)) => {
                self.application.auto_backup = value
            }
            (SettingKey::BackupDelay, SettingValue::Int(value)) => {
                self.application.backup_delay = value
            }
            (SettingKey::BarkEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.bark = value
            }
            (SettingKey::BarkProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[15] = value
            }
            (SettingKey::BlondEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.blond_energy_consumption = value
            }
            (SettingKey::BlondProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[32] = value
            }
            (SettingKey::BlueEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.blue_energy_consumption = value
            }
            (SettingKey::BlueProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[2] = value
            }
            (SettingKey::C4EnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.c4 = value
            }
            (SettingKey::C4Prob, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[42] = value
            }
            (SettingKey::Ch4ToCo2Divisor, SettingValue::Int(value)) => {
                self.parameters.atmosphere.methane_to_carbon_divisor = value
            }
            (SettingKey::CladeComplexity, SettingValue::Int(value)) => {
                self.parameters.genetics.clade_complexity = value
            }
            (SettingKey::Co2ToCh4Divisor, SettingValue::Int(value)) => {
                self.parameters.atmosphere.carbon_to_methane_divisor = value
            }
            (SettingKey::ConnectToServer, SettingValue::Boolean(value)) => {
                self.application.connect_to_server = value
            }
            (SettingKey::CoralEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.coral_energy_consumption = value
            }
            (SettingKey::CoralProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[8] = value
            }
            (SettingKey::CreamEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.cream_energy_consumption = value
            }
            (SettingKey::CreamOrganicSubsProduced, SettingValue::Double(value)) => {
                self.parameters.lifecycle.cream_organic_subs_produced = value
            }
            (SettingKey::CreamProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[22] = value
            }
            (SettingKey::CrowdedforestEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.crowded_forest_energy_consumption = value
            }
            (SettingKey::CrowdedlimeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.crowded_lime_energy_consumption = value
            }
            (SettingKey::CyanEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.cyan_energy_consumption = value
            }
            (SettingKey::CyanProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[3] = value
            }
            (SettingKey::DarkgrayEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.dark_gray_energy_consumption = value
            }
            (SettingKey::DarkgrayProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[30] = value
            }
            (SettingKey::DarkjadeDelay, SettingValue::Int(value)) => {
                self.parameters.effects.dark_jade_delay = value
            }
            (SettingKey::DarkEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.dark_energy_consumption = value
            }
            (SettingKey::DarkProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[24] = value
            }
            (SettingKey::DecayEnergy, SettingValue::Double(value)) => {
                self.parameters.lifecycle.decay_energy = value
            }
            (SettingKey::Delay, SettingValue::Int(value)) => self.application.delay = value,
            (SettingKey::DodgeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.dodge_energy_consumption = value
            }
            (SettingKey::DrainSubsDivisor, SettingValue::Int(value)) => {
                self.parameters.atmosphere.drain_divisor = value
            }
            (SettingKey::Elasticity, SettingValue::Double(value)) => {
                self.parameters.physics.elasticity = value
            }
            (SettingKey::ExperienceEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.experience_energy_consumption = value
            }
            (SettingKey::EyeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.eye_energy_consumption = value
            }
            (SettingKey::EyeProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[18] = value
            }
            (SettingKey::FallowEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.fallow_energy_consumption = value
            }
            (SettingKey::FallowProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[39] = value
            }
            (SettingKey::FireEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.fire_energy_consumption = value
            }
            (SettingKey::FireProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[29] = value
            }
            (SettingKey::FlowerEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.flower_energy_consumption = value
            }
            (SettingKey::FlowerProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[33] = value
            }
            (SettingKey::ForestEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.forest = value
            }
            (SettingKey::ForestProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[10] = value
            }
            (SettingKey::GoldDivisor, SettingValue::Double(value)) => {
                self.parameters.effects.gold_divisor = value
            }
            (SettingKey::GoldProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[31] = value
            }
            (SettingKey::GrassEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.grass = value
            }
            (SettingKey::GrassProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[43] = value
            }
            (SettingKey::GrayEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.gray_energy_consumption = value
            }
            (SettingKey::GrayProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[5] = value
            }
            (SettingKey::GreenEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.green = value
            }
            (SettingKey::GreenObtainedEnergyDivisor, SettingValue::Int(value)) => {
                self.parameters.effects.green_energy_divisor = value
            }
            (SettingKey::GreenProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[1] = value
            }
            (SettingKey::HardwareAcceleration, SettingValue::Int(value)) => {
                self.application.hardware_acceleration = value
            }
            (SettingKey::Healing, SettingValue::Int(value)) => {
                self.parameters.lifecycle.healing = value
            }
            (SettingKey::ImmuneSystem, SettingValue::Int(value)) => {
                self.parameters.lifecycle.immune_system = value
            }
            (SettingKey::IndigoDivisor, SettingValue::Double(value)) => {
                self.parameters.effects.indigo_divisor = value
            }
            (SettingKey::IndigoEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.indigo_energy_consumption = value
            }
            (SettingKey::IndigoProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[37] = value
            }
            (SettingKey::InitialCh4, SettingValue::Double(value)) => {
                self.parameters.atmosphere.initial_methane = value
            }
            (SettingKey::InitialCo2, SettingValue::Double(value)) => {
                self.parameters.atmosphere.initial_carbon_dioxide = value
            }
            (SettingKey::InitialComplexity, SettingValue::Int(value)) => {
                self.parameters.genetics.initial_complexity = value
            }
            (SettingKey::InitialEnergy, SettingValue::Int(value)) => {
                self.parameters.initial_energy = f64::from(value)
            }
            (SettingKey::InitialO2, SettingValue::Double(value)) => {
                self.parameters.atmosphere.initial_oxygen = value
            }
            (SettingKey::InitialOrganisms, SettingValue::Int(value)) => {
                self.genesis.initial_attempts = value
            }
            (SettingKey::JadeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.jade = value
            }
            (SettingKey::JadeProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[41] = value
            }
            (SettingKey::LavenderEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.lavender_energy_consumption = value
            }
            (SettingKey::LavenderProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[38] = value
            }
            (SettingKey::LavenderShield, SettingValue::Int(value)) => {
                self.parameters.effects.lavender_shield = value
            }
            (SettingKey::LeafEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.leaf = value
            }
            (SettingKey::LeafProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[12] = value
            }
            (SettingKey::LilacEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.lilac_energy_consumption = value
            }
            (SettingKey::LilacProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[27] = value
            }
            (SettingKey::LimeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.lime = value
            }
            (SettingKey::LimeProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[14] = value
            }
            (SettingKey::Locale, SettingValue::Text(value)) => self.application.locale = value,
            (SettingKey::LocalPort, SettingValue::Int(value)) => {
                self.application.local_port = value
            }
            (SettingKey::MagentaEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.magenta_energy_consumption = value
            }
            (SettingKey::MagentaProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[6] = value
            }
            (SettingKey::MaroonEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.maroon_energy_consumption = value
            }
            (SettingKey::MaroonProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[19] = value
            }
            (SettingKey::MaxAge, SettingValue::Int(value)) => {
                self.parameters.effects.max_age = value
            }
            (SettingKey::MaxConnections, SettingValue::Int(value)) => {
                self.application.max_connections = value
            }
            (SettingKey::MaxMutationRate, SettingValue::Int(value)) => {
                self.parameters.genetics.max_mutation_rate = value
            }
            (SettingKey::MaxRot, SettingValue::Double(value)) => {
                self.parameters.physics.max_rotation = value
            }
            (SettingKey::MaxVel, SettingValue::Double(value)) => {
                self.parameters.physics.max_velocity = value
            }
            (SettingKey::MetaMutationRate, SettingValue::Int(value)) => {
                self.parameters.genetics.meta_mutation_rate = value
            }
            (SettingKey::MintEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.mint_energy_consumption = value
            }
            (SettingKey::MintProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[21] = value
            }
            (SettingKey::MinMutationRate, SettingValue::Int(value)) => {
                self.parameters.genetics.min_mutation_rate = value
            }
            (SettingKey::MosquitoEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.mosquito_energy_consumption = value
            }
            (SettingKey::OchreEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.ochre_energy_consumption = value
            }
            (SettingKey::OchreProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[25] = value
            }
            (SettingKey::OliveEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.olive_energy_consumption = value
            }
            (SettingKey::OliveProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[20] = value
            }
            (SettingKey::OrangeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.orange_energy_consumption = value
            }
            (SettingKey::OrangeProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[9] = value
            }
            (SettingKey::OrganicObtainedEnergy, SettingValue::Double(value)) => {
                self.parameters.lifecycle.organic_obtained_energy = value
            }
            (SettingKey::OrganicSubsProduced, SettingValue::Double(value)) => {
                self.parameters.lifecycle.organic_subs_produced = value
            }
            (SettingKey::OrganismsVectorSize, SettingValue::Int(value)) => {
                self.application.organisms_vector_size = value
            }
            (SettingKey::PinkEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.pink_energy_consumption = value
            }
            (SettingKey::PinkProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[7] = value
            }
            (SettingKey::PlagueEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.plague_energy_consumption = value
            }
            (SettingKey::PlagueProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[35] = value
            }
            (SettingKey::PurpleEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.purple = value
            }
            (SettingKey::PurpleProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[44] = value
            }
            (SettingKey::RedEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.red_energy_consumption = value
            }
            (SettingKey::RedProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[0] = value
            }
            (SettingKey::RoseEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.rose_energy_consumption = value
            }
            (SettingKey::RoseProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[23] = value
            }
            (SettingKey::Rubbing, SettingValue::Double(value)) => {
                self.parameters.lifecycle.rubbing = value
            }
            (SettingKey::ScourgeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.scourge_energy_consumption = value
            }
            (SettingKey::SegmentCostDivisor, SettingValue::Int(value)) => {
                self.parameters.lifecycle.segment_cost_divisor = value
            }
            (SettingKey::ServerAddress, SettingValue::Text(value)) => {
                self.application.server_address = value
            }
            (SettingKey::ServerPort, SettingValue::Int(value)) => {
                self.application.server_port = value
            }
            (SettingKey::SilverEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.silver_energy_consumption = value
            }
            (SettingKey::SilverProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[28] = value
            }
            (SettingKey::SkyEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.sky_energy_consumption = value
            }
            (SettingKey::SkyProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[26] = value
            }
            (SettingKey::SpikeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.spike_energy_consumption = value
            }
            (SettingKey::SpikeProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[36] = value
            }
            (SettingKey::SporeEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.spore_energy_consumption = value
            }
            (SettingKey::SporeProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[40] = value
            }
            (SettingKey::SpringEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.spring = value
            }
            (SettingKey::SpringProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[11] = value
            }
            (SettingKey::SummerEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.summer = value
            }
            (SettingKey::SummerProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[13] = value
            }
            (SettingKey::SymbiontEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.effects.symbiont = value
            }
            (SettingKey::TealEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.teal_energy_consumption = value
            }
            (SettingKey::TealProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[17] = value
            }
            (SettingKey::UserName, SettingValue::Text(value)) => {
                self.application.user_name = JavaText::from_utf8(&value)
            }
            (SettingKey::VioletEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.violet_energy_consumption = value
            }
            (SettingKey::VioletProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[16] = value
            }
            (SettingKey::VirusEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.virus_energy_consumption = value
            }
            (SettingKey::WhiteEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.white_energy_consumption = value
            }
            (SettingKey::WhiteProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[4] = value
            }
            (SettingKey::WindowHeight, SettingValue::Int(value)) => {
                self.application.window_height = value
            }
            (SettingKey::WindowState, SettingValue::Int(value)) => {
                self.application.window_state = value
            }
            (SettingKey::WindowWidth, SettingValue::Int(value)) => {
                self.application.window_width = value
            }
            (SettingKey::WindowX, SettingValue::Int(value)) => self.application.window_x = value,
            (SettingKey::WindowY, SettingValue::Int(value)) => self.application.window_y = value,
            (SettingKey::WorldHeight, SettingValue::Int(value)) => self.genesis.height = value,
            (SettingKey::WorldWidth, SettingValue::Int(value)) => self.genesis.width = value,
            (SettingKey::YellowEnergyConsumption, SettingValue::Double(value)) => {
                self.parameters.lifecycle.yellow_energy_consumption = value
            }
            (SettingKey::YellowProb, SettingValue::Int(value)) => {
                self.parameters.genetics.color_weights[45] = value
            }
            _ => return Err(SettingsError::WrongValueType),
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn source_input(
        &mut self,
        key: SettingKey,
        value: SettingValue,
    ) -> Result<(), SettingsError> {
        self.assign(key, value)
    }

    fn accepts(&self, key: SettingKey, value: &SettingValue) -> bool {
        match (key, value) {
            (SettingKey::WorldWidth, SettingValue::Int(number)) => *number > 0,
            (SettingKey::WorldHeight, SettingValue::Int(number)) => *number > 0,
            (SettingKey::Delay, SettingValue::Int(number)) => *number > 0,
            (SettingKey::BackupDelay, SettingValue::Int(number)) => *number > 0,
            (SettingKey::InitialO2, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::InitialCo2, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::InitialCh4, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::Rubbing, SettingValue::Double(number)) => *number >= 0.0 && *number <= 1.0,
            (SettingKey::Elasticity, SettingValue::Double(number)) => {
                *number >= 0.0 && *number <= 1.0
            }
            (SettingKey::InitialOrganisms, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::InitialEnergy, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::InitialComplexity, SettingValue::Int(number)) => *number > 0,
            (SettingKey::CladeComplexity, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::MaxAge, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::AgeDivisor, SettingValue::Int(number)) => *number > 0,
            (SettingKey::Co2ToCh4Divisor, SettingValue::Int(number)) => *number > 0,
            (SettingKey::Ch4ToCo2Divisor, SettingValue::Int(number)) => *number > 0,
            (SettingKey::MetaMutationRate, SettingValue::Int(number)) => {
                *number >= 0 && *number <= 10000
            }
            (SettingKey::MaxMutationRate, SettingValue::Int(number)) => {
                *number >= 0 && *number <= 10000
            }
            (SettingKey::MinMutationRate, SettingValue::Int(number)) => {
                *number >= 0 && *number <= self.parameters.genetics.max_mutation_rate
            }
            (SettingKey::RedEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::GreenEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::BlueEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::CyanEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::WhiteEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::GrayEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::YellowEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::MagentaEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::PinkEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::CoralEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::OrangeEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::ForestEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::SpringEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::LeafEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::SummerEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::LimeEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::C4EnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::JadeEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::GrassEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::PurpleEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::BarkEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::VioletEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::TealEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::EyeEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::MaroonEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::OliveEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::MintEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::CreamEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::RoseEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::DarkEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::OchreEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::SkyEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::LilacEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::SilverEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::FireEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::DarkgrayEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::GoldDivisor, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::BlondEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::FlowerEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::AuburnEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::PlagueEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::SpikeEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::IndigoEnergyConsumption, SettingValue::Double(number)) => *number >= 1.0,
            (SettingKey::LavenderEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::FallowEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::SporeEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::RedProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::GreenProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::BlueProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::CyanProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::WhiteProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::GrayProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::YellowProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::MagentaProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::PinkProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::CoralProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::OrangeProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::ForestProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::SpringProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::LeafProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::SummerProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::LimeProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::C4Prob, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::JadeProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::GrassProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::PurpleProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::BarkProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::VioletProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::TealProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::EyeProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::MaroonProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::OliveProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::MintProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::CreamProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::RoseProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::DarkProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::OchreProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::SkyProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::LilacProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::SilverProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::FireProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::DarkgrayProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::GoldProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::BlondProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::FlowerProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::AuburnProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::PlagueProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::SpikeProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::IndigoProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::LavenderProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::FallowProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::SporeProb, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::SegmentCostDivisor, SettingValue::Int(number)) => *number > 0,
            (SettingKey::DrainSubsDivisor, SettingValue::Int(number)) => *number > 0,
            (SettingKey::GreenObtainedEnergyDivisor, SettingValue::Int(number)) => *number > 0,
            (SettingKey::OrganicObtainedEnergy, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::OrganicSubsProduced, SettingValue::Double(number)) => {
                *number >= 0.0 && *number <= 1.0
            }
            (SettingKey::CreamOrganicSubsProduced, SettingValue::Double(number)) => {
                *number >= 0.0 && *number <= 1.0
            }
            (SettingKey::Healing, SettingValue::Int(number)) => *number >= 1,
            (SettingKey::ImmuneSystem, SettingValue::Int(number)) => *number >= 1,
            (SettingKey::LavenderShield, SettingValue::Int(number)) => *number >= 0,
            (SettingKey::IndigoDivisor, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::MosquitoEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::CrowdedlimeEnergyConsumption, SettingValue::Double(number)) => {
                *number > 0.0
            }
            (SettingKey::CrowdedforestEnergyConsumption, SettingValue::Double(number)) => {
                *number > 0.0
            }
            (SettingKey::SymbiontEnergyConsumption, SettingValue::Double(number)) => *number > 0.0,
            (SettingKey::ExperienceEnergyConsumption, SettingValue::Double(number)) => {
                *number >= 0.0
            }
            (SettingKey::DodgeEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::DarkjadeDelay, SettingValue::Int(number)) => *number >= 1,
            (SettingKey::VirusEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::ScourgeEnergyConsumption, SettingValue::Double(number)) => *number >= 0.0,
            (SettingKey::DecayEnergy, SettingValue::Double(number)) => *number >= 0.0,
            _ => false,
        }
    }
}

/// A dialog draft owns editor text independently of committed settings
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsDraft {
    editors: [String; 132],
    locale_index: usize,
    automatic_backup: bool,
    hardware: HardwareSelection,
    disable_framebuffer: bool,
    open: bool,
}

impl SettingsDraft {
    pub fn new(state: &SettingsState) -> Self {
        let acceleration = state.application.hardware_acceleration;
        let hardware = match acceleration {
            0 => HardwareSelection::None,
            1..=6 => HardwareSelection::OpenGl,
            _ => HardwareSelection::Unselected,
        };
        Self {
            editors: std::array::from_fn(|index| {
                state.get(NUMERIC_EDITOR_KEYS[index]).editor_text()
            }),
            locale_index: state.locale_index(),
            automatic_backup: state.application.auto_backup,
            hardware,
            disable_framebuffer: (4..=6).contains(&acceleration),
            open: true,
        }
    }

    pub fn editors(&self) -> &[String; 132] {
        &self.editors
    }
    pub fn locale_index(&self) -> usize {
        self.locale_index
    }
    pub fn automatic_backup(&self) -> bool {
        self.automatic_backup
    }
    pub fn backup_delay_enabled(&self) -> bool {
        self.automatic_backup
    }
    pub fn hardware(&self) -> HardwareSelection {
        self.hardware
    }
    pub fn disable_framebuffer(&self) -> bool {
        self.disable_framebuffer
    }
    pub fn framebuffer_control_enabled(&self) -> bool {
        self.hardware != HardwareSelection::None
    }
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_editor(&mut self, key: SettingKey, text: String) -> Result<(), SettingsError> {
        if !self.open {
            return Err(SettingsError::ClosedDraft);
        }
        let index = NUMERIC_EDITOR_KEYS
            .iter()
            .position(|candidate| *candidate == key)
            .ok_or(SettingsError::NotNumericEditor)?;
        self.editors[index] = text.replace('\n', " ");
        Ok(())
    }

    pub fn replace_editors(&mut self, editors: [String; 132]) -> Result<(), SettingsError> {
        if !self.open {
            return Err(SettingsError::ClosedDraft);
        }
        self.editors = editors.map(|text| text.replace('\n', " "));
        Ok(())
    }

    pub fn set_choices(
        &mut self,
        locale_index: usize,
        automatic_backup: bool,
        hardware: HardwareSelection,
        disable_framebuffer: bool,
    ) -> Result<(), SettingsError> {
        if !self.open {
            return Err(SettingsError::ClosedDraft);
        }
        if locale_index >= LANGUAGE_CODES.len() {
            return Err(SettingsError::InvalidLocale);
        }
        self.locale_index = locale_index;
        self.automatic_backup = automatic_backup;
        self.hardware = hardware;
        self.disable_framebuffer = disable_framebuffer;
        Ok(())
    }

    pub fn defaults(&mut self) -> Result<(), SettingsError> {
        if !self.open {
            return Err(SettingsError::ClosedDraft);
        }
        let defaults = SettingsState::new("en".to_owned());
        self.editors =
            std::array::from_fn(|index| defaults.get(NUMERIC_EDITOR_KEYS[index]).editor_text());
        self.automatic_backup = defaults.application.auto_backup;
        self.hardware = HardwareSelection::None;
        self.disable_framebuffer = false;
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), SettingsError> {
        if !self.open {
            return Err(SettingsError::ClosedDraft);
        }
        self.open = false;
        Ok(())
    }

    /// Invoke locale handling immediately, before assigning any numeric field
    pub fn ok(
        &mut self,
        state: &mut SettingsState,
        mut locale_changed: impl FnMut(&SettingsState),
    ) -> Result<SettingsApply, SettingsError> {
        if !self.open {
            return Err(SettingsError::ClosedDraft);
        }
        let changed = state.locale_index() != self.locale_index;
        if changed {
            state.application.locale = LANGUAGE_CODES[self.locale_index].to_owned();
            locale_changed(state);
        }
        let mut outcomes = [EditorOutcome::Applied; 132];
        for (index, key) in NUMERIC_EDITOR_KEYS.iter().copied().enumerate() {
            if key == SettingKey::BackupDelay {
                state.application.auto_backup = self.automatic_backup;
            }
            let value = match state.get(key) {
                SettingValue::Int(_) => java_int(&self.editors[index]).map(SettingValue::Int),
                SettingValue::Double(_) => {
                    java_double(&self.editors[index]).map(SettingValue::Double)
                }
                _ => unreachable!("Every numeric editor has a numeric owner"),
            };
            let Some(value) = value else {
                outcomes[index] = EditorOutcome::InvalidNumber;
                continue;
            };
            if state.accepts(key, &value) {
                state
                    .assign(key, value)
                    .expect("Numeric setting storage matches its declared type");
            } else {
                outcomes[index] = EditorOutcome::OutsideRange;
            }
        }
        let current = state.application.hardware_acceleration;
        let request = match self.hardware {
            HardwareSelection::None => Some(0),
            HardwareSelection::OpenGl if self.disable_framebuffer && current < 4 => Some(4),
            HardwareSelection::OpenGl
                if !self.disable_framebuffer && current != 0 && current > 3 =>
            {
                Some(1)
            }
            _ => None,
        };
        if let Some(value) = request {
            state.application.hardware_acceleration = value;
        }
        self.open = false;
        Ok(SettingsApply {
            editors: outcomes,
            locale_changed: changed,
            legacy_hardware_request: request,
        })
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "network_settings_tests.rs"]
mod network_tests;
