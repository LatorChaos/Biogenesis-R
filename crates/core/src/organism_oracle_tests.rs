// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{EffectsState, Gene, GenomeModifiers, GenomeProperties, OrganismState, REACTION_NAMES};
use std::cell::Ref;
use std::collections::{BTreeMap, BTreeSet};

#[path = "birth_oracle_tests.rs"]
mod birth_oracle_tests;

#[path = "movement_oracle_tests.rs"]
mod movement_oracle_tests;

#[path = "death_oracle_tests.rs"]
mod death_oracle_tests;

#[path = "contact_state_oracle_tests.rs"]
mod contact_state_oracle_tests;
#[path = "contact_support_oracle_tests.rs"]
mod contact_support_oracle_tests;
#[path = "cream_lilac_oracle_tests.rs"]
mod cream_lilac_oracle_tests;
#[path = "disabling_oracle_tests.rs"]
mod disabling_oracle_tests;
#[path = "draining_oracle_tests.rs"]
mod draining_oracle_tests;
#[path = "frame_effects_oracle_tests.rs"]
mod frame_effects_oracle_tests;
#[path = "infection_oracle_tests.rs"]
mod infection_oracle_tests;
#[path = "reactions_oracle_tests.rs"]
mod reactions_oracle_tests;
#[path = "special_contacts_oracle_tests.rs"]
mod special_contacts_oracle_tests;
#[path = "white_fruit_oracle_tests.rs"]
mod white_fruit_oracle_tests;

#[path = "maintenance_oracle_tests.rs"]
mod maintenance_oracle_tests;

#[path = "reproduction_oracle_tests.rs"]
pub(in crate::world) mod reproduction_oracle_tests;

#[path = "xml_oracle_tests.rs"]
mod xml_oracle_tests;

#[path = "workload_endpoint_tests.rs"]
mod workload_endpoint_tests;

#[path = "atmosphere_frame_oracle_tests.rs"]
mod atmosphere_frame_oracle_tests;

#[path = "workload_trajectory_tests.rs"]
mod workload_trajectory_tests;

#[path = "empty_genome_source_tests.rs"]
mod empty_genome_source_tests;

#[path = "incoming_oracle_tests.rs"]
mod incoming_oracle_tests;

#[path = "outgoing_oracle_tests.rs"]
mod outgoing_oracle_tests;

#[path = "phenotype_color_oracle_tests.rs"]
mod phenotype_color_oracle_tests;

#[path = "organism_array_oracle_tests.rs"]
mod organism_array_oracle_tests;

#[path = "gene_array_oracle_tests.rs"]
mod gene_array_oracle_tests;

#[path = "rectangle_oracle_tests.rs"]
mod rectangle_oracle_tests;

pub(in crate::world) type Fields = BTreeMap<String, String>;

fn primitive_reference_schemas() -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let mut settings = BTreeMap::new();
    let mut organism = BTreeMap::new();
    let mut omitted_strings = BTreeSet::new();
    for line in include_str!("../tests/fixtures/organism_state.tsv")
        .lines()
        .take_while(|line| *line != "E")
    {
        let row: Vec<_> = line.split('\t').collect();
        match row[0] {
            "U" => {
                if ["SERVER_ADDRESS", "USER_NAME"].contains(&row[1]) {
                    assert_eq!(row[2], "java.lang.String");
                    assert!(omitted_strings.insert(row[1]));
                    continue;
                }
                assert!(settings.insert(row[1].into(), row[2].into()).is_none());
            }
            "O" => {
                let kind = row[3].rsplit('.').next().unwrap().to_owned();
                if let Some(previous) = organism.insert(row[2].to_owned(), kind.clone()) {
                    assert_eq!(previous, kind);
                }
            }
            _ => {}
        }
    }
    // Primitive runtime oracles omit the factory fixture's two String settings
    assert_eq!(
        omitted_strings,
        BTreeSet::from(["SERVER_ADDRESS", "USER_NAME"])
    );
    assert_eq!(settings.len(), 147);
    assert_eq!(organism.len(), 154);
    (settings, organism)
}

#[derive(Default)]
pub(in crate::world) struct Stage {
    organism: Fields,
    world: Fields,
    pub(in crate::world) genome: Fields,
    pub(in crate::world) genes: BTreeMap<usize, Fields>,
}
struct Case {
    name: String,
    operation: String,
    seed: i64,
    settings: Fields,
    inputs: Fields,
    stages: BTreeMap<String, Stage>,
    assertions: Fields,
    suffix: Option<(i64, i32, u64)>,
}
struct View<'a> {
    id: i64,
    state: &'a OrganismState,
    effects: &'a EffectsState,
    body: Option<&'a Body>,
    genome: Option<Ref<'a, Genome>>,
    infected: Option<Ref<'a, Genome>>,
    saved: Option<Ref<'a, Genome>>,
}
impl<'a> From<&'a Organism> for View<'a> {
    fn from(value: &'a Organism) -> Self {
        Self {
            id: value.id,
            state: &value.state,
            effects: &value.effects,
            body: Some(&value.body),
            genome: value.genome.as_ref().map(|code| code.borrow()),
            infected: value.infected_genome.as_ref().map(|code| code.borrow()),
            saved: value.saved_genome.as_ref().map(|code| code.borrow()),
        }
    }
}
fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}
fn decoded(value: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(value, 16).unwrap())
}
fn integer(fields: &Fields, key: &str) -> i32 {
    fields[key].parse().unwrap()
}
fn boolean(fields: &Fields, key: &str) -> bool {
    fields[key].parse().unwrap()
}
fn real(fields: &Fields, key: &str) -> f64 {
    decoded(&fields[key])
}
fn color(value: Option<u32>) -> String {
    value.map_or_else(|| "~".into(), |value| format!("{value:08x}"))
}
fn presence<T>(value: Option<&T>) -> String {
    if value.is_some() {
        "present".into()
    } else {
        "~".into()
    }
}
fn rectangle(value: IntRect) -> String {
    format!("{},{},{},{}", value.x, value.y, value.width, value.height)
}
fn array<T: ToString>(values: impl Iterator<Item = T>) -> String {
    values
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
fn int_array(view: &View<'_>, field: crate::IntField) -> String {
    view.body.and_then(|body| body.arrays.int(field).as_ref()).map_or_else(
        || "~".into(), |owner| array(owner.values().into_iter()))
}
fn double_array(view: &View<'_>, field: crate::DoubleField) -> String {
    view.body.and_then(|body| body.arrays.double(field).as_ref()).map_or_else(
        || "~".into(), |owner| array(owner.bits().into_iter().map(|value| format!("{value:016x}"))))
}

fn organism_fields(view: &View<'_>) -> Fields {
    let s = view.state;
    let e = view.effects;
    let mut out = Fields::new();
    out.insert("_parentID".into(), s.parent_id.to_string());
    out.insert("_nChildren".into(), s.children.to_string());
    out.insert("_nVirusChildren".into(), s.virus_children.to_string());
    out.insert("_nTotalChildren".into(), s.total_children.to_string());
    out.insert("_nTotalKills".into(), s.total_kills.to_string());
    out.insert("_nTotalInfected".into(), s.total_infected.to_string());
    out.insert("_centerX".into(), s.integer_center[0].to_string());
    out.insert("_centerY".into(), s.integer_center[1].to_string());
    out.insert("_age".into(), s.age.to_string());
    out.insert("_framesColor".into(), s.temporary_color_frames.to_string());
    out.insert(
        "_earlyReproduceEnergy".into(),
        s.reproduction.early_energy.to_string(),
    );
    out.insert("_reproduceEnergy".into(), s.reproduction.energy.to_string());
    out.insert(
        "_infectedReproduceEnergy".into(),
        s.reproduction.infected_energy.to_string(),
    );
    out.insert(
        "_timeToReproduce".into(),
        s.reproduction.countdown.to_string(),
    );
    out.insert(
        "_timeToReproduceMax".into(),
        s.reproduction.maximum_countdown.to_string(),
    );
    out.insert("_max_age".into(), s.reproduction.maximum_age.to_string());
    out.insert("hasGrown".into(), s.has_grown.to_string());
    out.insert("_theta".into(), bits(s.angle));
    out.insert("_lastTheta".into(), bits(s.last_angle));
    out.insert("_dCenterX".into(), bits(s.center[0]));
    out.insert("_dCenterY".into(), bits(s.center[1]));
    out.insert("dx".into(), bits(s.velocity[0]));
    out.insert("dy".into(), bits(s.velocity[1]));
    out.insert("dtheta".into(), bits(s.angular_velocity));
    out.insert("_energy".into(), bits(s.energy));
    out.insert("alive".into(), s.alive.to_string());
    out.insert("active".into(), s.active.to_string());
    out.insert("hasMoved".into(), s.has_moved.to_string());
    out.insert("_indigo".into(), e.indigo.to_string());
    out.insert("_antiviral".into(), e.antiviral.to_string());
    out.insert("_lavender".into(), e.lavender.to_string());
    out.insert("_createlavender".into(), e.create_lavender.to_string());
    out.insert("_healing".into(), e.healing.to_string());
    out.insert("_sporetime".into(), e.spore_time.to_string());
    out.insert("_sporeversion".into(), e.spore_version.to_string());
    out.insert("_skyversion".into(), e.sky_version.to_string());
    out.insert("_fallowversion".into(), e.fallow_version.to_string());
    out.insert("_plagueversion".into(), e.plague_version.to_string());
    out.insert("_summerinactivity".into(), e.summer_inactivity.to_string());
    out.insert("_yellowCounter".into(), e.yellow_count.to_string());
    out.insert("_reproducelate".into(), e.reproduce_late.to_string());
    out.insert("_gold".into(), e.gold.to_string());
    out.insert("_spin".into(), e.spin.to_string());
    out.insert("_jadefactor".into(), e.jade_factor.to_string());
    out.insert("_lengthfriend".into(), e.friend_length.to_string());
    out.insert("_thetafriend".into(), e.friend_theta.to_string());
    out.insert("_lowersymmetric".into(), e.lower_symmetric.to_string());
    out.insert(
        "_segmentpretoucheffects".into(),
        e.segment_pre_touch_effects.to_string(),
    );
    out.insert("_usefriendeffects".into(), e.use_friend_effects.to_string());
    out.insert("_maintenance".into(), bits(e.maintenance));
    out.insert("_methanotrophy".into(), bits(e.methanotrophy));
    out.insert("_photosynthesis".into(), bits(e.photosynthesis));
    out.insert(
        "_colonyPhotosynthesis".into(),
        bits(e.colony_photosynthesis),
    );
    out.insert("_forestphoto".into(), bits(e.forest_photo));
    out.insert("_leafphoto".into(), bits(e.leaf_photo));
    out.insert("_siblingbattle".into(), e.sibling_battle.to_string());
    out.insert("_generationbattle".into(), e.generation_battle.to_string());
    out.insert("_clockwise".into(), e.clockwise.to_string());
    out.insert("_altruist".into(), e.altruist.to_string());
    out.insert("_isaplant".into(), e.is_plant.to_string());
    out.insert("_isaconsumer".into(), e.is_consumer.to_string());
    out.insert("_isafungus".into(), e.is_fungus.to_string());
    out.insert("_isakiller".into(), e.is_killer.to_string());
    out.insert("_isgray".into(), e.is_gray.to_string());
    out.insert("_isspike".into(), e.is_spike.to_string());
    out.insert("_canmove".into(), e.can_move.to_string());
    out.insert("_canreact".into(), e.can_react.to_string());
    out.insert("_isfrozen".into(), e.is_frozen.to_string());
    out.insert("_allfrozen".into(), e.all_frozen.to_string());
    out.insert("_isinjured".into(), e.is_injured.to_string());
    out.insert("_isreproductive".into(), e.is_reproductive.to_string());
    out.insert("_isinfectious".into(), e.is_infectious.to_string());
    out.insert("_iscoral".into(), e.is_coral.to_string());
    out.insert("_isauburn".into(), e.is_auburn.to_string());
    out.insert("_isblond".into(), e.is_blond.to_string());
    out.insert("_issilver".into(), e.is_silver.to_string());
    out.insert("_isenhanced".into(), e.is_enhanced.to_string());
    out.insert("_islime".into(), e.is_lime.to_string());
    out.insert("_isonlyc4".into(), e.is_only_c4.to_string());
    out.insert("_transfersenergy".into(), e.transfers_energy.to_string());
    out.insert("_crowded".into(), e.crowded.to_string());
    out.insert("_candodge".into(), e.can_dodge.to_string());
    out.insert("_dodge".into(), e.dodge.to_string());
    out.insert("_hasdodged".into(), e.has_dodged.to_string());
    out.insert("_useframemovement".into(), e.use_frame_movement.to_string());
    out.insert("_useextraeffects".into(), e.use_extra_effects.to_string());
    out.insert(
        "_usepretoucheffects".into(),
        e.use_pre_touch_effects.to_string(),
    );
    out.insert("_ID".into(), view.id.to_string());
    out.insert("_color".into(), color(s.temporary_argb()));
    out.insert(
        "_nPosition".into(),
        s.network_position.as_ref().map(|text| text.to_utf8().expect("Fixture positions use valid UTF16")).unwrap_or_else(|| "~".into()),
    );
    out.insert("lastFrame".into(), s.last_frame.as_ref().map(|value| rectangle(value.get())).unwrap_or_else(|| "~".into()));
    out.insert("bounds".into(), rectangle(s.bounds.get()));
    out.insert(
        "_sizeRect".into(),
        view.body.and_then(|body| body.size_rect.as_ref()).map(|value| rectangle(value.get())).unwrap_or_else(|| "~".into()),
    );
    out.insert(
        "_mass".into(),
        bits(view.body.map_or(0.0, |body| body.mass)),
    );
    out.insert(
        "_I".into(),
        bits(view.body.map_or(0.0, |body| body.inertia)),
    );
    out.insert(
        "_growthRatio".into(),
        view.body.map_or(0, |body| body.growth_ratio).to_string(),
    );
    out.insert(
        "_segments".into(),
        view.body.map_or(0, |body| body.segment_count).to_string(),
    );
    out.insert(
        "_haseyes".into(),
        view.body.is_some_and(|body| body.eyes.enabled).to_string(),
    );
    out.insert(
        "_hasgoodvision".into(),
        view.body
            .is_some_and(|body| body.eyes.good_vision)
            .to_string(),
    );
    out.insert(
        "_updateEffects".into(),
        view.body.map_or(0, |body| body.update_effects).to_string(),
    );
    out.insert("_geneticCode".into(), presence(view.genome.as_deref()));
    out.insert(
        "_infectedGeneticCode".into(),
        presence(view.infected.as_deref()),
    );
    out.insert("_savedGeneticCode".into(), presence(view.saved.as_deref()));
    out.insert(
        "_world".into(),
        if s.world.is_present() { "present" } else { "~" }.into(),
    );
    // This source fixture deliberately supplies no VisibleWorld; Rust's frontend stays outside the model
    out.insert("_visibleWorld".into(), "~".into());
    out.insert(
        "_startPointX".into(),
        int_array(view, crate::IntField::StartX),
    );
    out.insert(
        "_startPointY".into(),
        int_array(view, crate::IntField::StartY),
    );
    out.insert(
        "_endPointX".into(),
        int_array(view, crate::IntField::EndX),
    );
    out.insert(
        "_endPointY".into(),
        int_array(view, crate::IntField::EndY),
    );
    out.insert(
        "_segColor".into(),
        view.body.and_then(|body| body.phenotype_colors.as_ref()).map(|colors| (0..colors.len()).map(|index| color(colors.get(index).unwrap().as_ref().map(crate::JavaColor::argb))).collect::<Vec<_>>().join(",")).unwrap_or_else(|| "~".into()),
    );
    out.insert(
        "_m".into(),
        double_array(view, crate::DoubleField::Length),
    );
    out.insert(
        "_m1".into(),
        double_array(view, crate::DoubleField::StartRadius),
    );
    out.insert(
        "_m2".into(),
        double_array(view, crate::DoubleField::EndRadius),
    );
    out.insert(
        "_mphoto".into(),
        double_array(view, crate::DoubleField::Photo),
    );
    out.insert("x1".into(), int_array(view, crate::IntField::X1));
    out.insert("y1".into(), int_array(view, crate::IntField::Y1));
    out.insert("x2".into(), int_array(view, crate::IntField::X2));
    out.insert("y2".into(), int_array(view, crate::IntField::Y2));
    for (channel, name) in REACTION_NAMES.iter().enumerate() {
        out.insert(
            format!("_seg{name}Reaction"),
            int_array(view, crate::IntField::Reaction(channel)),
        );
    }
    assert_eq!(
        out.len(),
        154,
        "Every source organism field needs an explicit mapping"
    );
    out
}

fn world_fields(world: &World) -> Fields {
    let mut out = Fields::new();
    out.insert("_width".into(), world.config.get().width.to_string());
    out.insert("_height".into(), world.config.get().height.to_string());
    out.insert("NEXT_ID".into(), world.next_id.get().to_string());
    out.insert("_population".into(), world.population().to_string());
    out.insert("organism_count".into(), world.organisms.borrow().len().to_string());
    out.insert("_O2".into(), bits(world.oxygen.get()));
    out.insert("_CO2".into(), bits(world.carbon_dioxide.get()));
    out.insert("_CH4".into(), bits(world.methane.get()));
    // The sticky flag remains observable independently of current queue membership
    out.insert("_corridorexists".into(), world.corridors_exist.get().to_string());
    out
}

pub(in crate::world) fn genome_fields(genome: &Genome) -> Fields {
    let mut out = Fields::new();
    let properties = genome.properties();
    let modifiers = genome.modifiers();
    out.insert("_symmetry".into(), properties.symmetry.to_string());
    out.insert("_mirror".into(), properties.mirror.to_string());
    out.insert("_mutationrate".into(), properties.mutation_rate.to_string());
    out.insert("_activity".into(), properties.activity.to_string());
    out.insert("_modifiescream".into(), modifiers.cream.to_string());
    out.insert("_modifiesfallow".into(), modifiers.fallow.to_string());
    out.insert("_modifiesspore".into(), modifiers.spore.to_string());
    out.insert("_adaptspore".into(), modifiers.adapt_spore.to_string());
    out.insert("_plague".into(), modifiers.plague.to_string());
    out.insert(
        "_disperseChildren".into(),
        modifiers.disperse_children.to_string(),
    );
    out.insert(
        "_generationBattle".into(),
        modifiers.generation_battle.to_string(),
    );
    out.insert(
        "_siblingBattle".into(),
        modifiers.sibling_battle.to_string(),
    );
    out.insert("_altruist".into(), modifiers.altruist.to_string());
    out.insert("_familial".into(), modifiers.familial.to_string());
    out.insert("_social".into(), modifiers.social.to_string());
    out.insert("_peaceful".into(), modifiers.peaceful.to_string());
    out.insert("_passive".into(), modifiers.passive.to_string());
    out.insert("_clockwise".into(), modifiers.clockwise.to_string());
    out.insert("_mimicall".into(), modifiers.mimic_all.to_string());
    out.insert("_modifiespink".into(), modifiers.modifies_pink.to_string());
    out.insert(
        "_modifieslilac".into(),
        modifiers.modifies_lilac.to_string(),
    );
    out.insert("_modifiessky".into(), modifiers.modifies_sky.to_string());
    out.insert("_selfish".into(), modifiers.selfish.to_string());
    out.insert("_generation".into(), genome.generation().to_string());
    out.insert("_cladeID".into(), genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref().unwrap_or("~").into());
    out.insert("_updateClade".into(), genome.update_clade().to_string());
    out
}
pub(in crate::world) fn gene_fields(gene: &Gene) -> Fields {
    let mut out = Fields::new();
    out.insert("_length".into(), bits(gene.length()));
    out.insert("_theta".into(), bits(gene.theta()));
    out.insert("_color".into(), color(gene.color()));
    out.insert("_branch".into(), gene.branch().to_string());
    for (index, name) in REACTION_NAMES.iter().enumerate() {
        out.insert(format!("_{name}reaction"), gene.reaction(index).to_string());
    }
    out
}

fn compare_fields(
    case: &str,
    stage: &str,
    group: &str,
    actual: &Fields,
    expected: &Fields,
    errors: &mut Vec<String>,
) {
    if actual.keys().ne(expected.keys()) {
        errors.push(format!(
            "{case}\t{stage}\t{group}\tschema\tactual={:?}\texpected={:?}",
            actual.keys(),
            expected.keys()
        ));
        return;
    }
    for (key, value) in actual {
        let target = &expected[key];
        if value == target {
            continue;
        }
        let geometric = group == "O" && ["_m", "_m1", "_m2", "_mass", "_I"].contains(&key.as_str());
        let close = geometric && value != "~" && target != "~" && {
            let a: Vec<_> = value.split(',').collect();
            let b: Vec<_> = target.split(',').collect();
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    let a = decoded(a);
                    let b = decoded(b);
                    if b.is_nan() {
                        a.is_nan()
                    } else if b.is_infinite() {
                        a == b
                    } else {
                        a.is_finite() && (a - b).abs() <= 1e-10 * b.abs().max(1.0)
                    }
                })
        };
        if !close {
            errors.push(format!(
                "{case}\t{stage}\t{group}\t{key}\tactual={value}\texpected={target}"
            ));
        }
    }
}
fn compare_stage(
    case: &Case,
    name: &str,
    view: &View<'_>,
    world: &World,
    errors: &mut Vec<String>,
) {
    let expected = &case.stages[name];
    compare_fields(
        &case.name,
        name,
        "O",
        &organism_fields(view),
        &expected.organism,
        errors,
    );
    compare_fields(
        &case.name,
        name,
        "W",
        &world_fields(world),
        &expected.world,
        errors,
    );
    if let Some(genome) = view.genome.as_deref() {
        compare_fields(
            &case.name,
            name,
            "G",
            &genome_fields(genome),
            &expected.genome,
            errors,
        );
        if genome.genes().unwrap().len() != expected.genes.len() {
            errors.push(format!(
                "{}\t{name}\tN\tgene count\tactual={}\texpected={}",
                case.name,
                genome.genes().unwrap().len(),
                expected.genes.len()
            ));
        }
        for (index, expected_gene) in &expected.genes {
            if *index < genome.genes().unwrap().len() {
                compare_fields(
                    &case.name,
                    name,
                    &format!("N{index}"),
                    &gene_fields(&genome.gene_ref(*index).unwrap().borrow()),
                    expected_gene,
                    errors,
                );
            }
        }
    } else if !expected.genome.is_empty() || !expected.genes.is_empty() {
        errors.push(format!(
            "{}\t{name}\tG\tunexpected absent genotype",
            case.name
        ));
    }
}

pub(in crate::world) fn explicit_genome(stage: &Stage) -> Genome {
    let fields = &stage.genome;
    let genes: Vec<_> = stage
        .genes
        .values()
        .map(|gene| {
            Gene::new(
                real(gene, "_length"),
                real(gene, "_theta"),
                if gene["_color"] == "~" {
                    None
                } else {
                    Some(u32::from_str_radix(&gene["_color"], 16).unwrap())
                },
                integer(gene, "_branch"),
                std::array::from_fn(|index| {
                    integer(gene, &format!("_{}reaction", REACTION_NAMES[index]))
                }),
            )
            .shared()
        })
        .collect();
    let mut modifiers = GenomeModifiers::default();
    modifiers.cream = integer(fields, "_modifiescream");
    modifiers.fallow = integer(fields, "_modifiesfallow");
    modifiers.spore = integer(fields, "_modifiesspore");
    modifiers.adapt_spore = integer(fields, "_adaptspore");
    modifiers.plague = boolean(fields, "_plague");
    modifiers.disperse_children = boolean(fields, "_disperseChildren");
    modifiers.generation_battle = boolean(fields, "_generationBattle");
    modifiers.sibling_battle = boolean(fields, "_siblingBattle");
    modifiers.altruist = boolean(fields, "_altruist");
    modifiers.familial = boolean(fields, "_familial");
    modifiers.social = boolean(fields, "_social");
    modifiers.peaceful = boolean(fields, "_peaceful");
    modifiers.passive = boolean(fields, "_passive");
    modifiers.clockwise = boolean(fields, "_clockwise");
    modifiers.mimic_all = boolean(fields, "_mimicall");
    modifiers.modifies_pink = boolean(fields, "_modifiespink");
    modifiers.modifies_lilac = boolean(fields, "_modifieslilac");
    modifiers.modifies_sky = boolean(fields, "_modifiessky");
    modifiers.selfish = boolean(fields, "_selfish");
    let mut genome = Genome::from_parts(
        &genes,
        GenomeProperties {
            symmetry: integer(fields, "_symmetry"),
            mirror: integer(fields, "_mirror"),
            mutation_rate: integer(fields, "_mutationrate"),
            activity: integer(fields, "_activity"),
            modifiers,
        },
    );
    genome.set_generation(integer(fields, "_generation"));
    genome.set_clade_id(if fields["_cladeID"] == "~" {
        None
    } else {
        Some(fields["_cladeID"].clone())
    });
    genome.set_update_clade(boolean(fields, "_updateClade"));
    genome
}

pub(in crate::world) fn empty_reference_world(settings: &Fields, seed: i64) -> World {
    assert_eq!(integer(settings, "INITIAL_ORGANISMS"), 0);
    let mut settings_parameters = parameters(settings);
    let source_lifecycle = settings_parameters.lifecycle;
    settings_parameters.lifecycle = crate::LifecycleParams::default();
    let world = World::with_parameters(
        WorldConfig {
            seed,
            width: integer(settings, "WORLD_WIDTH").try_into().unwrap(),
            height: integer(settings, "WORLD_HEIGHT").try_into().unwrap(),
            initial_attempts: 0,
        },
        settings_parameters,
    )
    .unwrap();
    // Install source globals before actors; the original empty World does not inspect lifecycle costs
    world.session.0.parameters.borrow_mut().lifecycle = source_lifecycle;
    world
}

fn parameters(settings: &Fields) -> WorldParameters {
    let mut result = WorldParameters::default();
    result.atmosphere.initial_oxygen = real(settings, "INITIAL_O2");
    result.atmosphere.initial_carbon_dioxide = real(settings, "INITIAL_CO2");
    result.atmosphere.initial_methane = real(settings, "INITIAL_CH4");
    result.atmosphere.carbon_to_methane_divisor = integer(settings, "CO2_TO_CH4_DIVISOR");
    result.atmosphere.methane_to_carbon_divisor = integer(settings, "CH4_TO_CO2_DIVISOR");
    result.atmosphere.drain_divisor = integer(settings, "DRAIN_SUBS_DIVISOR");
    result.physics.elasticity = real(settings, "ELASTICITY");
    result.physics.max_velocity = real(settings, "MAX_VEL");
    result.physics.max_rotation = real(settings, "MAX_ROT");
    result.lifecycle.segment_cost_divisor = integer(settings, "SEGMENT_COST_DIVISOR");
    result.lifecycle.healing = integer(settings, "HEALING");
    result.lifecycle.immune_system = integer(settings, "IMMUNE_SYSTEM");
    result.lifecycle.rubbing = real(settings, "RUBBING");
    result.lifecycle.cyan_energy_consumption = real(settings, "CYAN_ENERGY_CONSUMPTION");
    result.lifecycle.teal_energy_consumption = real(settings, "TEAL_ENERGY_CONSUMPTION");
    result.lifecycle.yellow_energy_consumption = real(settings, "YELLOW_ENERGY_CONSUMPTION");
    result.lifecycle.sky_energy_consumption = real(settings, "SKY_ENERGY_CONSUMPTION");
    result.lifecycle.fallow_energy_consumption = real(settings, "FALLOW_ENERGY_CONSUMPTION");
    result.lifecycle.magenta_energy_consumption = real(settings, "MAGENTA_ENERGY_CONSUMPTION");
    result.lifecycle.dark_gray_energy_consumption = real(settings, "DARKGRAY_ENERGY_CONSUMPTION");
    result.lifecycle.organic_obtained_energy = real(settings, "ORGANIC_OBTAINED_ENERGY");
    result.lifecycle.coral_energy_consumption = real(settings, "CORAL_ENERGY_CONSUMPTION");
    result.lifecycle.dodge_energy_consumption = real(settings, "DODGE_ENERGY_CONSUMPTION");
    result.lifecycle.indigo_energy_consumption = real(settings, "INDIGO_ENERGY_CONSUMPTION");
    result.lifecycle.lavender_energy_consumption = real(settings, "LAVENDER_ENERGY_CONSUMPTION");
    result.lifecycle.orange_energy_consumption = real(settings, "ORANGE_ENERGY_CONSUMPTION");
    result.lifecycle.fire_energy_consumption = real(settings, "FIRE_ENERGY_CONSUMPTION");
    result.lifecycle.red_energy_consumption = real(settings, "RED_ENERGY_CONSUMPTION");
    result.lifecycle.pink_energy_consumption = real(settings, "PINK_ENERGY_CONSUMPTION");
    result.lifecycle.maroon_energy_consumption = real(settings, "MAROON_ENERGY_CONSUMPTION");
    result.lifecycle.blue_energy_consumption = real(settings, "BLUE_ENERGY_CONSUMPTION");
    result.lifecycle.organic_subs_produced = real(settings, "ORGANIC_SUBS_PRODUCED");
    result.lifecycle.cream_energy_consumption = real(settings, "CREAM_ENERGY_CONSUMPTION");
    result.lifecycle.lilac_energy_consumption = real(settings, "LILAC_ENERGY_CONSUMPTION");
    result.lifecycle.silver_energy_consumption = real(settings, "SILVER_ENERGY_CONSUMPTION");
    result.lifecycle.mosquito_energy_consumption = real(settings, "MOSQUITO_ENERGY_CONSUMPTION");
    result.lifecycle.ochre_energy_consumption = real(settings, "OCHRE_ENERGY_CONSUMPTION");
    result.lifecycle.experience_energy_consumption =
        real(settings, "EXPERIENCE_ENERGY_CONSUMPTION");
    result.lifecycle.spike_energy_consumption = real(settings, "SPIKE_ENERGY_CONSUMPTION");
    result.lifecycle.white_energy_consumption = real(settings, "WHITE_ENERGY_CONSUMPTION");
    result.lifecycle.virus_energy_consumption = real(settings, "VIRUS_ENERGY_CONSUMPTION");
    result.lifecycle.plague_energy_consumption = real(settings, "PLAGUE_ENERGY_CONSUMPTION");
    result.lifecycle.scourge_energy_consumption = real(settings, "SCOURGE_ENERGY_CONSUMPTION");
    result.lifecycle.violet_energy_consumption = real(settings, "VIOLET_ENERGY_CONSUMPTION");
    result.lifecycle.olive_energy_consumption = real(settings, "OLIVE_ENERGY_CONSUMPTION");
    result.lifecycle.gray_energy_consumption = real(settings, "GRAY_ENERGY_CONSUMPTION");
    result.lifecycle.cream_organic_subs_produced = real(settings, "CREAM_ORGANIC_SUBS_PRODUCED");
    result.lifecycle.crowded_forest_energy_consumption =
        real(settings, "CROWDEDFOREST_ENERGY_CONSUMPTION");
    result.lifecycle.crowded_lime_energy_consumption =
        real(settings, "CROWDEDLIME_ENERGY_CONSUMPTION");
    result.lifecycle.mint_energy_consumption = real(settings, "MINT_ENERGY_CONSUMPTION");
    result.lifecycle.rose_energy_consumption = real(settings, "ROSE_ENERGY_CONSUMPTION");
    result.lifecycle.dark_energy_consumption = real(settings, "DARK_ENERGY_CONSUMPTION");
    result.lifecycle.blond_energy_consumption = real(settings, "BLOND_ENERGY_CONSUMPTION");
    result.lifecycle.flower_energy_consumption = real(settings, "FLOWER_ENERGY_CONSUMPTION");
    result.lifecycle.auburn_energy_consumption = real(settings, "AUBURN_ENERGY_CONSUMPTION");
    result.lifecycle.eye_energy_consumption = real(settings, "EYE_ENERGY_CONSUMPTION");
    result.lifecycle.decay_energy = real(settings, "DECAY_ENERGY");

    for (index, (name, _, _)) in crate::DEFAULT_COLORS.iter().enumerate() {
        result.genetics.color_weights[index] = integer(settings, &format!("{name}_PROB"));
    }
    result.genetics.initial_complexity = integer(settings, "INITIAL_COMPLEXITY");
    result.genetics.min_mutation_rate = integer(settings, "MIN_MUTATION_RATE");
    result.genetics.max_mutation_rate = integer(settings, "MAX_MUTATION_RATE");
    result.genetics.meta_mutation_rate = integer(settings, "META_MUTATION_RATE");
    result.genetics.clade_complexity = integer(settings, "CLADE_COMPLEXITY");
    result.initial_energy = f64::from(integer(settings, "INITIAL_ENERGY"));
    result.spore_energy_consumption = real(settings, "SPORE_ENERGY_CONSUMPTION");
    result.effects.max_age = integer(settings, "MAX_AGE");
    result.effects.age_divisor = integer(settings, "AGE_DIVISOR");
    result.effects.green_energy_divisor = integer(settings, "GREEN_OBTAINED_ENERGY_DIVISOR");
    result.effects.dark_jade_delay = integer(settings, "DARKJADE_DELAY");
    result.effects.lavender_shield = integer(settings, "LAVENDER_SHIELD");
    result.effects.gold_divisor = real(settings, "GOLD_DIVISOR");
    result.effects.indigo_divisor = real(settings, "INDIGO_DIVISOR");
    result.effects.green = real(settings, "GREEN_ENERGY_CONSUMPTION");
    result.effects.forest = real(settings, "FOREST_ENERGY_CONSUMPTION");
    result.effects.spring = real(settings, "SPRING_ENERGY_CONSUMPTION");
    result.effects.summer = real(settings, "SUMMER_ENERGY_CONSUMPTION");
    result.effects.leaf = real(settings, "LEAF_ENERGY_CONSUMPTION");
    result.effects.lime = real(settings, "LIME_ENERGY_CONSUMPTION");
    result.effects.bark = real(settings, "BARK_ENERGY_CONSUMPTION");
    result.effects.jade = real(settings, "JADE_ENERGY_CONSUMPTION");
    result.effects.grass = real(settings, "GRASS_ENERGY_CONSUMPTION");
    result.effects.purple = real(settings, "PURPLE_ENERGY_CONSUMPTION");
    result.effects.c4 = real(settings, "C4_ENERGY_CONSUMPTION");
    result.effects.symbiont = real(settings, "SYMBIONT_ENERGY_CONSUMPTION");
    result
}

fn run_case(case: Case, errors: &mut Vec<String>, stages: &mut usize) {
    let config = WorldConfig {
        seed: case.seed,
        width: integer(&case.settings, "WORLD_WIDTH"),
        height: integer(&case.settings, "WORLD_HEIGHT"),
        initial_attempts: 0,
    };
    let mut world = World::with_parameters(config, parameters(&case.settings)).unwrap();
    assert_eq!(
        world.oxygen.get().to_bits(),
        real(&case.settings, "INITIAL_O2").to_bits()
    );
    assert_eq!(
        world.carbon_dioxide.get().to_bits(),
        real(&case.settings, "INITIAL_CO2").to_bits()
    );
    assert_eq!(
        world.methane.get().to_bits(),
        real(&case.settings, "INITIAL_CH4").to_bits()
    );
    let constructor_probe = world.clone();
    let initial_state = constructor_probe.initial_organism_state();
    let supplied = (!case.stages["constructor"].genome.is_empty())
        .then(|| Rc::new(RefCell::new(explicit_genome(&case.stages["constructor"]))));
    let constructor_actor = Organism::pasted_candidate(supplied.as_ref().map(Rc::clone), initial_state);
    let constructor = View::from(&constructor_actor);
    compare_stage(
        &case,
        "constructor",
        &constructor,
        &constructor_probe,
        errors,
    );
    drop(constructor);
    drop(constructor_actor);
    *stages += 1;
    if case.operation == "constructor" {
        world = constructor_probe;
        assert!(case.assertions.is_empty());
    } else {
        let result = match case.operation.as_str() {
            "randomCreate" => world.construct_random(),
            "pasteOrganism" | "controlledCreateEffects" => world.construct_pasted(
                Rc::try_unwrap(supplied.unwrap()).unwrap().into_inner(),
                [
                    integer(&case.inputs, "paste_x"),
                    integer(&case.inputs, "paste_y"),
                ],
            ),
            operation => panic!("Unknown constructor operation {operation}"),
        };
        let succeeded = result.as_ref().is_ok_and(|value| value.is_some());
        assert_eq!(case.assertions.len(), 1);
        assert_eq!(
            succeeded,
            case.assertions["initialization_succeeded"] == "true",
            "{} initialization result",
            case.name
        );
        let mut organism = result.unwrap().unwrap();
        compare_stage(&case, "initial", &View::from(&organism), &world, errors);
        *stages += 1;
        let reapply = case
            .inputs
            .get("reapply_count")
            .map_or(0, |value| value.parse::<i32>().unwrap());
        if reapply > 0 {
            organism.state.age = integer(&case.inputs, "controlled_age");
            if let Some(phenotype) = case.inputs.get("controlled_phenotype") {
                if phenotype != "~" {
                    let argb = u32::from_str_radix(phenotype, 16).unwrap();
                    for color_index in organism.body.indices() {
                        organism.body.set_color_argb(color_index, argb).unwrap();
                    }
                }
            }
            compare_stage(
                &case,
                "controlled_input",
                &View::from(&organism),
                &world,
                errors,
            );
            *stages += 1;
            for application in 0..reapply {
                organism.effects.apply(
                    Some(&organism.genome.as_ref().unwrap().borrow()),
                    &mut organism.body,
                    organism.state.age,
                    &mut organism.state.reproduction,
                    &world.parameters().effects,
                ).unwrap();
                compare_stage(
                    &case,
                    if application == 0 {
                        "reapplied"
                    } else {
                        "reapplied_twice"
                    },
                    &View::from(&organism),
                    &world,
                    errors,
                );
                *stages += 1;
            }
        }
    }
    let suffix = case.suffix.unwrap();
    let actual = {
        let mut random = world.session.0.random.borrow_mut();
        (
            random.next_i64(),
            random.next_bounded(10000).unwrap(),
            random.next_f64().to_bits(),
        )
    };
    if actual != suffix {
        errors.push(format!(
            "{}\tR\trandom suffix\tactual={actual:?}\texpected={suffix:?}",
            case.name
        ));
    }
}

#[test]
fn real_factories_and_effects_match_every_recorded_java_field() {
    let text = include_str!("../tests/fixtures/organism_state.tsv");
    let mut current: Option<Case> = None;
    let mut errors = Vec::new();
    let mut cases = 0;
    let mut stages = 0;
    for line in text.lines() {
        let f: Vec<_> = line.split('\t').collect();
        if f[0] == "C" {
            assert!(current.is_none());
            current = Some(Case {
                name: f[1].into(),
                operation: f[2].into(),
                seed: f[3].parse().unwrap(),
                settings: Fields::new(),
                inputs: Fields::new(),
                stages: BTreeMap::new(),
                assertions: Fields::new(),
                suffix: None,
            });
            continue;
        }
        if f[0] == "E" {
            run_case(current.take().unwrap(), &mut errors, &mut stages);
            cases += 1;
            continue;
        }
        let case = current.as_mut().unwrap();
        match f[0] {
            "U" => {
                assert!(case.settings.insert(f[1].into(), f[3].into()).is_none());
            }
            "I" => {
                assert!(case.inputs.insert(f[1].into(), f[3].into()).is_none());
            }
            "O" => {
                assert!(
                    case.stages
                        .entry(f[1].into())
                        .or_default()
                        .organism
                        .insert(f[2].into(), f[4].into())
                        .is_none()
                );
            }
            "W" => {
                assert!(
                    case.stages
                        .entry(f[1].into())
                        .or_default()
                        .world
                        .insert(f[2].into(), f[4].into())
                        .is_none()
                );
            }
            "G" => {
                assert!(
                    case.stages
                        .entry(f[1].into())
                        .or_default()
                        .genome
                        .insert(f[2].into(), f[3].into())
                        .is_none()
                );
            }
            "N" => {
                assert!(
                    case.stages
                        .entry(f[1].into())
                        .or_default()
                        .genes
                        .entry(f[2].parse().unwrap())
                        .or_default()
                        .insert(f[3].into(), f[4].into())
                        .is_none()
                );
            }
            "A" => {
                assert!(case.assertions.insert(f[1].into(), f[2].into()).is_none());
            }
            "R" => {
                assert!(
                    case.suffix
                        .replace((
                            f[1].parse().unwrap(),
                            f[2].parse().unwrap(),
                            u64::from_str_radix(f[3], 16).unwrap()
                        ))
                        .is_none()
                );
            }
            record => panic!("Unknown organism record {record}"),
        }
    }
    assert!(current.is_none());
    assert_eq!(cases, 167);
    assert_eq!(stages, 436);
    if !errors.is_empty() {
        let report = std::env::temp_dir().join("organism-oracle-mismatches.tsv");
        std::fs::write(&report, errors.join("\n") + "\n").unwrap();
        panic!(
            "{} exact constructor mismatches; report={}\n{}",
            errors.len(),
            report.display(),
            errors
                .iter()
                .take(40)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn rejection_genome(first_branch: i32) -> Genome {
    let gene = Gene::new(12.0, 0.0, Some(0xff00ff00), first_branch, [0; 36]).shared();
    Genome::from_parts(
        &[gene],
        GenomeProperties {
            symmetry: 1,
            mirror: 0,
            mutation_rate: 0,
            activity: 0,
            modifiers: GenomeModifiers {
                cream: 1,
                fallow: 1,
                spore: 1,
                adapt_spore: 1,
                ..GenomeModifiers::default()
            },
        },
    )
}

#[test]
fn paste_rejects_a_forward_first_branch_after_consuming_only_constructor_randomness() {
    let world = World::new(WorldConfig {
        initial_attempts: 0,
        seed: 827364,
        ..WorldConfig::default()
    })
    .unwrap();
    let before = (
        world.oxygen.get().to_bits(),
        world.carbon_dioxide.get().to_bits(),
        world.methane.get().to_bits(),
        world.next_id.get(),
    );
    let mut expected_random = world.random_snapshot();
    expected_random.next_f64();
    let result = world
        .construct_pasted(rejection_genome(999), [3500, 3500])
        .unwrap();
    assert!(
        result.is_none(),
        "Java pasteOrganism rejects a first branch above its gene index"
    );
    assert_eq!(
        world.random_snapshot(), expected_random,
        "Only the real constructor orientation consumes randomness"
    );
    assert_eq!(
        (
            world.oxygen.get().to_bits(),
            world.carbon_dioxide.get().to_bits(),
            world.methane.get().to_bits(),
            world.next_id.get()
        ),
        before
    );
    assert!(world.organisms.borrow().is_empty());
}

fn placement_resources(world: &World) -> Vec<String> {
    vec![
        bits(world.carbon_dioxide.get()),
        bits(world.oxygen.get()),
        bits(world.methane.get()),
        world.next_id.get().to_string(),
    ]
}

#[test]
fn paste_success_rejection_overlap_and_rng_match_real_java_observations() {
    let mut world: Option<World> = None;
    let mut candidate: Option<Genome> = None;
    let mut name = String::new();
    let mut center = [0; 2];
    let mut cases = 0;
    let mut rejected_object_fields_unexposed = 0;
    for line in include_str!("../tests/fixtures/paste_rejection.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        match fields[0] {
            "C" => {
                assert!(world.is_none());
                name = fields[1].into();
                world = Some(
                    World::new(WorldConfig {
                        seed: fields[2].parse().unwrap(),
                        width: fields[3].parse().unwrap(),
                        height: fields[4].parse().unwrap(),
                        initial_attempts: 0,
                    })
                    .unwrap(),
                );
                center = [fields[5].parse().unwrap(), fields[6].parse().unwrap()];
                candidate = Some(rejection_genome(fields[7].parse().unwrap()));
                assert_eq!(fields[8] == "true", name == "overlapping-placement");
            }
            "K" => {
                let world = world.as_mut().unwrap();
                let blocker = world
                    .construct_pasted(rejection_genome(0), [3500, 3500])
                    .unwrap()
                    .unwrap();
                let bounds = blocker.state.bounds.get();
                assert_eq!(
                    [bounds.x, bounds.y, bounds.width, bounds.height],
                    std::array::from_fn(|index| fields[index + 1].parse::<i32>().unwrap())
                );
                assert_eq!(placement_resources(world), fields[5..9]);
                world.organisms.borrow_mut().push(Rc::new(RefCell::new(blocker)));
            }
            "B" => {
                assert_eq!(placement_resources(world.as_ref().unwrap()), fields[1..5]);
                let genome = candidate.as_ref().unwrap();
                assert_eq!(genome.generation(), fields[5].parse::<i32>().unwrap());
                assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref().unwrap_or("~"), fields[6]);
            }
            "A" => {
                let world = world.as_mut().unwrap();
                let before_count = world.organisms.borrow().len();
                let result = world
                    .construct_pasted(candidate.take().unwrap(), center)
                    .unwrap();
                assert_eq!(
                    result.is_some(),
                    fields[1] == "true",
                    "{name} initialization outcome"
                );
                assert_eq!(
                    placement_resources(world),
                    fields[2..6],
                    "{name} placement resources"
                );
                assert_eq!(
                    world.organisms.borrow().len(),
                    before_count,
                    "The shared factory must not insert an organism"
                );
                if let Some(organism) = result {
                    assert_eq!(
                        organism.genome.as_ref().unwrap().borrow().generation(),
                        fields[6].parse::<i32>().unwrap()
                    );
                    assert_eq!(
                        organism.genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref().unwrap_or("~"),
                        fields[7]
                    );
                    let bounds = organism.state.bounds.get();
                    assert_eq!(
                        [bounds.x, bounds.y, bounds.width, bounds.height],
                        std::array::from_fn(|index| fields[index + 8].parse::<i32>().unwrap())
                    );
                } else {
                    // Rust discards rejected candidates; their source generation, clade, and rectangle stay unexposed
                    // These six recorded source fields are not passing Rust parity observations
                    assert_eq!(fields.len(), 12);
                    rejected_object_fields_unexposed += fields[6..12].len();
                }
            }
            "R" => {
                let mut random = world.as_ref().unwrap().session.0.random.borrow_mut();
                assert_eq!(
                    random.next_i64(),
                    fields[1].parse::<i64>().unwrap(),
                    "{name} nextLong"
                );
                assert_eq!(
                    random.next_bounded(10000).unwrap(),
                    fields[2].parse::<i32>().unwrap(),
                    "{name} nextInt"
                );
                assert_eq!(
                    random.next_f64().to_bits(),
                    decoded(fields[3]).to_bits(),
                    "{name} nextDouble"
                );
            }
            "E" => {
                assert!(candidate.is_none());
                world = None;
                cases += 1;
            }
            record => panic!("Unknown paste observation record {record}"),
        }
    }
    assert!(world.is_none());
    assert_eq!(cases, 4);
    assert_eq!(rejected_object_fields_unexposed, 18);
}

fn identity_holder<'a>(organisms: &'a [Organism], name: &str) -> Option<&'a GenomeRef> {
    match name {
        "a.genotype" => organisms[0].genome.as_ref(),
        "b.genotype" => organisms[1].genome.as_ref(),
        "c.genotype" => organisms[2].genome.as_ref(),
        "b.infected" => organisms[1].infected_genome.as_ref(),
        "c.saved" => organisms[2].saved_genome.as_ref(),
        name => panic!("Unknown genotype holder {name}"),
    }
}

fn identity_value(code: Option<&GenomeRef>, field: &str) -> String {
    if field == "present" {
        return code.is_some().to_string();
    }
    let code = code.unwrap().borrow();
    match field {
        "_generation" => code.generation().to_string(),
        "_cladeID" => code.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref().unwrap_or("~").into(),
        "_updateClade" => code.update_clade().to_string(),
        "gene0._length" => bits(code.gene_ref(0).unwrap().borrow().length()),
        "gene0._redreaction" => code.gene_ref(0).unwrap().borrow().reactions()[0].to_string(),
        field => panic!("Unknown genotype value {field}"),
    }
}

#[test]
fn shared_constructor_identity_and_controlled_aliases_match_java_observations() {
    let mut world: Option<World> = None;
    let mut organisms = Vec::new();
    let mut stage = String::new();
    let mut stages = 0;
    let mut compared_identities = 0;
    let mut compared_values = 0;
    let mut deferred_statistics = 0;
    let mut deferred_side_effects = 0;
    let mut inputs = 0;
    for line in include_str!("../tests/fixtures/genotype_identity.tsv").lines() {
        let fields: Vec<_> = line.split('\t').collect();
        match fields[0] {
            "C" => {
                assert_eq!(fields[1], "shared-genotype");
                assert!(world.is_none());
                let actual = World::new(WorldConfig {
                    seed: fields[2].parse().unwrap(),
                    initial_attempts: 0,
                    ..WorldConfig::default()
                })
                .unwrap();
                let mut code = rejection_genome(0);
                code.set_generation(7);
                code.set_clade_id(Some("root".into()));
                code.set_update_clade(true);
                let shared = Rc::new(RefCell::new(code));
                let copied = Rc::new(RefCell::new(shared.borrow().java_clone().unwrap()));
                for code in [Rc::clone(&shared), shared, copied] {
                    organisms.push(
                        actual
                            .construct_pasted_shared(code, [3500, 3500])
                            .unwrap()
                            .unwrap(),
                    );
                }
                world = Some(actual);
            }
            "I" => {
                inputs += 1;
                match fields[2] {
                    // Frontend selection only avoids a Java callback; Rust stores no frontend reference
                    "visible._selectedOrganism" => assert_eq!(fields[3], "b"),
                    "a._reproduceEnergy" => {
                        organisms[0].state.reproduction.energy = fields[3].parse().unwrap();
                    }
                    "c._reproduceEnergy" => {
                        organisms[2].state.reproduction.energy = fields[3].parse().unwrap();
                    }
                    "b._infectedReproduceEnergy" => {
                        organisms[1].state.reproduction.infected_energy =
                            fields[3].parse().unwrap();
                    }
                    "b._savedGeneticCode" => {
                        organisms[1].saved_genome =
                            Some(Rc::clone(identity_holder(&organisms, fields[3]).unwrap()));
                    }
                    "c._savedGeneticCode" => {
                        organisms[2].saved_genome =
                            Some(Rc::clone(identity_holder(&organisms, fields[3]).unwrap()));
                    }
                    "a.genotype._cladeID" => organisms[0]
                        .genome.as_ref().unwrap().borrow_mut()
                        .set_clade_id(Some(fields[3].into())),
                    "a.genotype._generation" => organisms[0]
                        .genome.as_ref().unwrap().borrow_mut()
                        .set_generation(fields[3].parse().unwrap()),
                    "a.genotype._updateClade" => organisms[0]
                        .genome.as_ref().unwrap().borrow_mut()
                        .set_update_clade(fields[3].parse().unwrap()),
                    "a.genotype.gene0._length" => organisms[0]
                        .genome.as_ref().unwrap().borrow()
                        .gene_ref(0).unwrap().borrow_mut()
                        .set_length(decoded(fields[3])),
                    "a.genotype.gene0._redreaction" => organisms[0]
                        .genome.as_ref().unwrap().borrow()
                        .gene_ref(0).unwrap().borrow_mut()
                        .set_reaction(0, fields[3].parse().unwrap()),
                    field => panic!("Unknown identity input {field}"),
                }
            }
            "B" => {
                if stage != fields[1] {
                    stage = fields[1].into();
                    stages += 1;
                    // Controlled aliases protect representation; they do not exercise unimplemented infectedBy methods
                    let donor = match stage.as_str() {
                        "infection-organism-shared" | "infection-code-shared" => Some(0),
                        "infection-organism-clone" => Some(2),
                        "constructed" | "shared-edit" | "shared-second-edit" => None,
                        stage => panic!("Unknown identity stage {stage}"),
                    };
                    if let Some(donor) = donor {
                        organisms[1].infected_genome = Some(Rc::clone(organisms[donor].genome.as_ref().unwrap()));
                        organisms[1].saved_genome = None;
                    }
                }
                let a = organisms[0].genome.as_ref().unwrap().borrow();
                let b = organisms[1].genome.as_ref().unwrap().borrow();
                let c = organisms[2].genome.as_ref().unwrap().borrow();
                let actual = match fields[2] {
                    "a_genotype_is_b_genotype" => {
                        Rc::ptr_eq(organisms[0].genome.as_ref().unwrap(), organisms[1].genome.as_ref().unwrap())
                    }
                    "a_genotype_is_c_genotype" => {
                        Rc::ptr_eq(organisms[0].genome.as_ref().unwrap(), organisms[2].genome.as_ref().unwrap())
                    }
                    "a_genotype_values_equal_c" => *a == *c,
                    "a_gene_array_is_b_gene_array" => a.gene_array().unwrap().same_instance(b.gene_array().unwrap()),
                    "a_gene_array_is_c_gene_array" => a.gene_array().unwrap().same_instance(c.gene_array().unwrap()),
                    "a_gene0_is_b_gene0" => Rc::ptr_eq(&a.genes().unwrap()[0], &b.genes().unwrap()[0]),
                    "a_gene0_is_c_gene0" => Rc::ptr_eq(&a.genes().unwrap()[0], &c.genes().unwrap()[0]),
                    // Java immutable Color identity maps to exact nullable ARGB value semantics
                    "a_color0_is_c_color0" => a.gene_ref(0).unwrap().borrow().color() == c.gene_ref(0).unwrap().borrow().color(),
                    "b_infected_is_a_genotype" => organisms[1]
                        .infected_genome
                        .as_ref()
                        .is_some_and(|code| Rc::ptr_eq(code, organisms[0].genome.as_ref().unwrap())),
                    "b_infected_is_b_genotype" => organisms[1]
                        .infected_genome
                        .as_ref()
                        .is_some_and(|code| Rc::ptr_eq(code, organisms[1].genome.as_ref().unwrap())),
                    "b_infected_is_c_genotype" => organisms[1]
                        .infected_genome
                        .as_ref()
                        .is_some_and(|code| Rc::ptr_eq(code, organisms[2].genome.as_ref().unwrap())),
                    "b_saved_is_null" => organisms[1].saved_genome.is_none(),
                    "c_saved_is_a_genotype" => organisms[2]
                        .saved_genome
                        .as_ref()
                        .is_some_and(|code| Rc::ptr_eq(code, organisms[0].genome.as_ref().unwrap())),
                    "statistics_last_infected_is_b_genotype"
                    | "statistics_most_infections_is_a_genotype" => {
                        // No Rust statistics holder exists yet; preserve these observations for its later consumer
                        deferred_statistics += 1;
                        continue;
                    }
                    field => panic!("Unknown identity comparison {field}"),
                };
                assert_eq!(actual.to_string(), fields[3], "{stage} {}", fields[2]);
                compared_identities += 1;
            }
            "S" => {
                match fields[2] {
                    "a._nTotalInfected"
                    | "c._nTotalInfected"
                    | "b._infectedReproduceEnergy"
                    | "statistics.beingMostInfectionsNumber"
                    | "statistics.infectionsSum" => {}
                    field => panic!("Unknown deferred infection side effect {field}"),
                }
                // Infection counters, energy copying, and notifications require real infection methods before acceptance
                deferred_side_effects += 1;
            }
            "V" => {
                if matches!(
                    fields[2],
                    "statistics.lastInfected" | "statistics.mostInfections"
                ) {
                    assert!(matches!(
                        fields[3],
                        "present"
                            | "_generation"
                            | "_cladeID"
                            | "_updateClade"
                            | "gene0._length"
                            | "gene0._redreaction"
                    ));
                    deferred_statistics += 1;
                } else {
                    assert_eq!(
                        identity_value(identity_holder(&organisms, fields[2]), fields[3]),
                        fields[4],
                        "{} {} {}",
                        fields[1],
                        fields[2],
                        fields[3]
                    );
                    compared_values += 1;
                }
            }
            "R" => {
                let mut random = world.as_ref().unwrap().session.0.random.borrow_mut();
                assert_eq!(random.next_i64(), fields[1].parse::<i64>().unwrap());
                assert_eq!(
                    random.next_bounded(10000).unwrap(),
                    fields[2].parse::<i32>().unwrap()
                );
                assert_eq!(bits(random.next_f64()), fields[3]);
            }
            "E" => assert!(world.take().unwrap().organisms.borrow().is_empty()),
            record => panic!("Unknown identity record {record}"),
        }
    }
    assert!(world.is_none());
    assert_eq!(
        (stages, inputs, compared_identities, compared_values),
        (6, 16, 78, 165)
    );
    assert_eq!((deferred_statistics, deferred_side_effects), (74, 30));
}

#[test]
fn constructors_preserve_nonfinite_live_gases_in_the_java_energy_minimum() {
    let world = World::new(WorldConfig {
        initial_attempts: 0,
        ..WorldConfig::default()
    })
    .unwrap();
    world.add_carbon_dioxide(f64::NAN);
    let random = world.construct_random().unwrap().unwrap();
    assert!(random.state.energy.is_nan());
    let pasted = world
        .construct_pasted(rejection_genome(0), [3500, 3500])
        .unwrap()
        .unwrap();
    assert!(pasted.state.energy.is_nan());
    assert!(world.oxygen.get().is_nan());
    assert!(world.carbon_dioxide.get().is_nan());
}

#[path = "organism_image_oracle_tests.rs"]
mod organism_image_oracle_tests;

#[path = "settings_oracle_tests.rs"]
pub(in crate::world) mod settings_oracle_tests;
