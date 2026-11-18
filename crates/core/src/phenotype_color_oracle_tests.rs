// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay captured nullable phenotype calls from declared constructor and ordered inputs

use super::death_oracle_tests::{apply_reference_input, statistics_schemas};
use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{assert_observation, observed_field};
use crate::{ColorArrayRef, ColorKind, JavaColor, JavaText};
use std::rc::Rc;

#[path = "phenotype_color_graph_fixture.rs"]
pub(super) mod phenotype_color_graph_fixture;

const SOURCE: &str = include_str!("../tests/fixtures/phenotype-color/phenotype_color.tsv");
const DIRECTORY: &str = "crates/core/tests/fixtures/phenotype-color/";

type Row<'a> = Vec<&'a str>;

pub(super) fn cases(source: &str) -> Vec<Vec<Row<'_>>> {
    let mut cases = Vec::new();
    let mut current = Vec::new();
    for line in source.lines() {
        let row: Row<'_> = line.split('\t').collect();
        if row[0] == "C" {
            assert!(current.is_empty());
        }
        let end = row[0] == "E";
        current.push(row);
        if end {
            assert_eq!(current[0][0], "C");
            cases.push(std::mem::take(&mut current));
        }
    }
    assert!(current.is_empty());
    cases
}

#[derive(Default)]
pub(super) struct Identities {
    labels: BTreeMap<String, (char, usize)>,
    reverse: BTreeMap<(char, usize), String>,
    colors: Vec<JavaColor>,
    arrays: Vec<ColorArrayRef>,
    ints: Vec<crate::IntArrayRef>,
    doubles: Vec<crate::DoubleArrayRef>,
}

impl Identities {
    pub(super) fn bind(&mut self, label: &str, identity: (char, usize)) {
        if let Some(previous) = self.labels.insert(label.into(), identity) {
            assert_eq!(previous, identity, "Source owner {label} changed identity");
        }
        if let Some(previous) = self.reverse.insert(identity, label.into()) {
            assert_eq!(previous, label, "Distinct source owners collapsed");
        }
    }

    pub(super) fn color(&mut self, expected: &str, actual: Option<&JavaColor>) {
        if expected == "~" {
            assert!(actual.is_none());
        } else {
            let (label, argb) = expected.split_once(':').unwrap();
            let color = actual.expect("Captured Color must be present");
            assert_eq!(color.argb(), u32::from_str_radix(argb, 16).unwrap());
            self.bind(label, ('C', color.identity()));
            self.colors.push(color.clone());
        }
    }

    pub(super) fn array(&mut self, expected: &str, actual: Option<&ColorArrayRef>) {
        if expected == "~" {
            assert!(actual.is_none());
        } else {
            let array = actual.expect("Captured Color array must be present");
            self.bind(expected, ('A', array.identity()));
            self.arrays.push(array.clone());
        }
    }
    pub(super) fn ints(&mut self, expected: &str, actual: Option<&crate::IntArrayRef>) {
        if expected == "~" {
            assert!(actual.is_none());
        } else {
            let owner = actual.expect("Captured int array must be present");
            self.bind(expected, ('I', owner.identity()));
            self.ints.push(owner.clone());
        }
    }

    pub(super) fn doubles(&mut self, expected: &str, actual: Option<&crate::DoubleArrayRef>) {
        if expected == "~" {
            assert!(actual.is_none());
        } else {
            let owner = actual.expect("Captured double array must be present");
            self.bind(expected, ('D', owner.identity()));
            self.doubles.push(owner.clone());
        }
    }

}

struct Replay {
    name: String,
    world: World,
    actors: BTreeMap<String, OrganismRef>,
    genomes: BTreeMap<String, GenomeRef>,
    input_colors: BTreeMap<String, JavaColor>,
    input_arrays: BTreeMap<String, ColorArrayRef>,
    identities: Identities,
    clade: JavaText,
    arguments: Option<[usize; 2]>,
    outcome: Option<Result<(), WorldError>>,
    lines: Vec<RenderSegment>,
    paint: u32,
    image: Option<super::super::organism_image::OrganismImage>,
    errors: Vec<String>,
    calls: usize,
    snapshots: usize,
}

impl Replay {
    fn new(rows: &[Row<'_>]) -> Self {
        let settings: Fields = rows.iter().filter(|r| r[0] == "U")
            .map(|r| (r[1].into(), r[3].into())).collect();
        let world = empty_reference_world(&settings, rows[0][3].parse().unwrap());
        let mut input_colors = BTreeMap::new();
        let mut identities = Identities::default();
        for (label, kind) in [
            ("fixed_green", ColorKind::Green), ("fixed_red", ColorKind::Red),
            ("fixed_dark", ColorKind::Dark), ("fixed_eye", ColorKind::Eye),
            ("fixed_brown", ColorKind::Brown), ("fixed_vision", ColorKind::Vision),
        ] {
            let color = kind.color_ref().unwrap();
            identities.bind(label, ('C', color.identity()));
            identities.colors.push(color.clone());
            input_colors.insert(label.into(), color);
        }
        Self {
            name: rows[0][1].into(), world, actors: BTreeMap::new(), genomes: BTreeMap::new(),
            input_colors, input_arrays: BTreeMap::new(), identities,
            clade: JavaText::from_utf8("phenotype_witness"), arguments: None, outcome: None,
            lines: Vec::new(), paint: 0xffff_ffff, image: None, errors: Vec::new(),
            calls: 0, snapshots: 0,
        }
    }

    fn construct(&mut self, row: &[&str]) {
        let role = row[1];
        let count = match (self.name.as_str(), role) {
            (_, "selector") => 1,
            ("image_reverse_null_first" | "pretouch_dark_shared_prefix", "victim") => 4,
            ("reaction_reverse_bounds_before_null", "victim") => 2,
            ("reaction_zero_segments_null_colors", "victim") => 0,
            ("pretouch_dark_shared_prefix", "killer") => 4,
            (_, "victim") => 3,
            (_, "killer") => 1,
            _ => panic!("Unknown declared role"),
        };
        let genes: Vec<_> = (0..count).map(|index| {
            let kind = if self.name == "symmetric_late_null_gene" && role == "victim" && index == 0 {
                ColorKind::Eye
            } else { ColorKind::Green };
            Gene::new_with_color(12.0, 0.0, kind.color_ref(), 0, [0; 36]).shared()
        }).collect();
        let mut genome = Genome::from_parts(&genes, GenomeProperties {
            symmetry: 1, mirror: 0, mutation_rate: 0, activity: 0,
            modifiers: GenomeModifiers { cream: 1, fallow: 1, spore: 1, adapt_spore: 1,
                ..GenomeModifiers::default() },
        });
        genome.set_generation(41);
        genome.set_clade_text(Some(self.clade.clone()));
        genome.set_update_clade(false);
        let id = self.world.paste_genome(genome,
            [row[3].parse().unwrap(), row[4].parse().unwrap()]).unwrap().unwrap();
        let actor = Rc::clone(self.world.organisms.borrow().last().unwrap());
        assert_eq!(actor.borrow().id, id);
        let genome = Rc::clone(actor.borrow().genome.as_ref().unwrap());
        assert!(self.genomes.insert(row[2].into(), genome).is_none());
        assert!(self.actors.insert(role.into(), actor).is_none());
    }

    fn input_color(&mut self, label: &str) -> Option<JavaColor> {
        if label == "~" { return None; }
        let label = label.split(':').next().unwrap();
        if !self.input_colors.contains_key(label) {
            let kind = match label {
                "fresh_equal_eye" => ColorKind::Eye,
                "fresh_equal_red" => ColorKind::Red,
                _ => panic!("Undeclared Color input {label}"),
            };
            let color = JavaColor::from_argb(kind.argb().unwrap());
            self.identities.bind(label, ('C', color.identity()));
            self.identities.colors.push(color.clone());
            self.input_colors.insert(label.into(), color);
        }
        Some(self.input_colors[label].clone())
    }

    fn input(&mut self, row: &[&str], rows: &[Row<'_>]) {
        let (target, kind, value) = (row[1], row[2], row[3]);
        let parts: Vec<_> = target.split(':').collect();
        match parts.as_slice() {
            ["utils", "DRAIN_SUBS_DIVISOR"] => {
                assert_eq!(kind, "int");
                self.world.session.0.parameters.borrow_mut().atmosphere.drain_divisor = value.parse().unwrap();
            }
            ["utils", "TEAL_ENERGY_CONSUMPTION"] => {
                assert_eq!(kind, "double");
                self.world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = decoded(value);
            }
            ["actor", role, "_color"] => {
                assert_eq!(kind, "Color");
                let color = self.input_color(value);
                self.actors[*role].borrow_mut().state.temporary_color = color;
            }
            ["actor", role, "_segColor"] => {
                assert_eq!(kind, "Color[]");
                let array = if value == "~" { None } else {
                    if !self.input_arrays.contains_key(value) {
                        let recipes: Vec<_> = rows.iter().filter(|r|
                            r[0] == "I" && ["cached_colors", "shared_colors"].contains(&r[1])).collect();
                        assert_eq!(recipes.len(), 1);
                        let slots = recipes[0][3].split(',').map(|label| self.input_color(label)).collect();
                        let array = ColorArrayRef::from_slots(slots);
                        self.identities.bind(value, ('A', array.identity()));
                        self.identities.arrays.push(array.clone());
                        self.input_arrays.insert(value.into(), array);
                    }
                    Some(self.input_arrays[value].clone())
                };
                self.actors[*role].borrow_mut().body.phenotype_colors = array;
            }
            ["actor", role, "bounds"] => {
                assert_eq!(kind, "Rectangle");
                let values: Vec<i32> = value.split(',').map(|v| v.parse().unwrap()).collect();
                let [x,y,width,height] = values.as_slice() else { panic!("Rectangle input"); };
                self.actors[*role].borrow().state.bounds.set(IntRect {
                    x: *x, y: *y, width: *width, height: *height,
                });
            }
            ["gene", role, index, "_color"] => {
                assert_eq!(kind, "Color");
                let color = self.input_color(value);
                self.actors[*role].borrow().genome.as_ref().unwrap().borrow()
                    .genes().unwrap()[index.parse::<usize>().unwrap()].borrow_mut().set_color_handle(color);
            }
            ["gene", role, index, "_branch"] => {
                assert_eq!(kind, "int");
                self.actors[*role].borrow().genome.as_ref().unwrap().borrow()
                    .genes().unwrap()[index.parse::<usize>().unwrap()].borrow_mut().set_branch(value.parse().unwrap());
            }
            ["genome", role, "_mimicall"] => {
                assert_eq!(kind, "boolean");
                let reference = Rc::clone(self.actors[*role].borrow().genome.as_ref().unwrap());
                let mut genome = reference.borrow_mut();
                let mut properties = genome.properties();
                properties.modifiers.mimic_all = value.parse().unwrap();
                genome.set_properties(properties);
            }
            ["pretouch_args" | "reaction_args" | "brake_args"] => {
                assert_eq!(kind, "String");
                let args: Vec<_> = value.split(',').collect();
                assert_eq!(args[0], "killer");
                self.arguments = Some([args[1].parse().unwrap(), args[2].parse().unwrap()]);
            }
            ["shared_colors" | "cached_colors"] => assert_eq!(kind, "Color[]"),
            _ => apply_reference_input(&self.name, &BTreeMap::new(), target, kind, value,
                &mut self.world, &self.actors, &mut self.genomes, &mut self.errors),
        }
    }

    fn call(&mut self, row: &[&str]) {
        assert_eq!(row[3], "victim");
        let actor = Rc::clone(&self.actors["victim"]);
        let peer = Rc::clone(&self.actors["killer"]);
        self.lines.clear();
        self.image = None;
        let result = match row[2] {
            "die" => self.world.die(&mut actor.borrow_mut(),
                Some(super::super::death::Killer::Other(&mut peer.borrow_mut()))),
            "leafExtraEffects" => self.world.leaf_extra_effects(&mut actor.borrow_mut()),
            "symmetric" => actor.borrow_mut().rebuild_body().map_err(WorldError::Geometry),
            "draw" => super::super::outgoing::emit_actor_segments(&actor.borrow(), &mut self.paint,
                |line| self.lines.push(line)).map_err(WorldError::Geometry),
            "getImage" => {
                self.image = Some(super::super::organism_image::image_from_organism(
                    &actor.borrow(), 1_048_576).expect("Captured image completes"));
                Ok(())
            }
            operation => {
                let [segment, peer_segment] = self.arguments.expect("Explicit argument input");
                match operation {
                    "pretouchEffects" => self.world.pre_touch_effects(&mut actor.borrow_mut(),
                        &mut peer.borrow_mut(), segment, peer_segment),
                    "touchReaction" => self.world.touch_reaction(&mut actor.borrow_mut(),
                        &peer.borrow(), segment, peer_segment),
                    "touchfriendReaction" => self.world.touch_friend_reaction(&mut actor.borrow_mut(),
                        &peer.borrow(), segment, peer_segment),
                    "touchfriendBrake" => self.world.touch_friend_brake(&mut actor.borrow_mut(),
                        &peer.borrow(), segment, peer_segment),
                    _ => panic!("Unknown captured operation {operation}"),
                }
            }
        };
        self.outcome = Some(result);
        self.calls += 1;
    }

    fn outcome(&self, row: &[&str]) {
        use crate::GeometryError::*;
        let result = self.outcome.as_ref().expect("Call precedes outcome");
        if row[2] == "none" {
            assert_eq!(result, &Ok(()), "{} {}", self.name, row[1]);
            return;
        }
        let expected = match self.name.as_str() {
            "die_whole_null_colors" => MissingColorArray,
            "die_short_shared_colors" => ColorIndex { segment: 2, length: 2 },
            "leaf_null_temporary_prefix" => MissingTemporaryColor,
            "draw_eyes_null_prefix" => MissingPhenotypeColor { segment: 1 },
            "pretouch_dark_shared_prefix" => MissingPhenotypeColor { segment: 3 },
            "reaction_reverse_bounds_before_null" => ColorIndex { segment: 1, length: 1 },
            "friend_brake_null_actor" => MissingPhenotypeColor { segment: 0 },
            _ => panic!("Unexpected captured exception"),
        };
        assert_eq!(result, &Err(WorldError::Geometry(expected)), "{} {}", self.name, row[1]);
        let class = if matches!(expected, ColorIndex { .. }) {
            "java.lang.ArrayIndexOutOfBoundsException"
        } else { "java.lang.NullPointerException" };
        assert_eq!(row[2], class);
        assert_eq!(row[3], "biogenesis.Organism");
        let method = match self.name.as_str() {
            "die_whole_null_colors" | "die_short_shared_colors" => "die",
            "leaf_null_temporary_prefix" => "leafExtraEffects",
            "draw_eyes_null_prefix" => "draw",
            "pretouch_dark_shared_prefix" | "friend_brake_null_actor" => "getTypeColor",
            "reaction_reverse_bounds_before_null" if row[1] == "step_0" => "touchReaction",
            "reaction_reverse_bounds_before_null" => "touchfriendReaction",
            _ => unreachable!(),
        };
        assert_eq!(row[4], method);
        assert!(row[5].parse::<u32>().unwrap() > 0);
    }

    fn snapshot(&mut self, rows: &[Row<'_>]) {
        let stage = rows[0][1];
        assert_eq!(rows[0][0], "P");
        for role in ["selector", "victim", "killer"] {
            let actor = self.actors[role].borrow();
            let expected: Fields = rows.iter().filter(|r| r[0] == "O" && r[2] == role)
                .map(|r| (r[3].into(), r[4].into())).collect();
            assert_eq!(expected.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                ["_I", "_age", "_canmove", "_canreact", "_centerX", "_centerY", "_colonyPhotosynthesis", "_color", "_endPointX", "_endPointY", "_energy", "_forestphoto", "_framesColor", "_growthRatio", "_haseyes", "_hasgoodvision", "_infectedGeneticCode", "_m", "_m1", "_m2", "_mass", "_methanotrophy", "_mphoto", "_nTotalKills", "_photosynthesis", "_savedGeneticCode", "_segColor", "_segmentpretoucheffects", "_segments", "_sizeRect", "_startPointX", "_startPointY", "_updateEffects", "_useframemovement", "_usefriendeffects", "_usepretoucheffects", "active", "alive", "dtheta", "dx", "dy", "hasMoved", "x1", "x2", "y1", "y2"].into_iter().collect());
            let ordinary: Fields = expected.iter().filter(|(key, _)|
                !["_color", "_segColor", "_infectedGeneticCode", "_savedGeneticCode"].contains(&key.as_str()))
                .map(|(key,value)| (key.clone(),value.clone())).collect();
            let all = organism_fields(&View::from(&*actor));
            let actual: Fields = ordinary.keys().map(|key| (key.clone(), all[key].clone())).collect();
            compare_fields(&self.name, stage, "O", &actual, &ordinary, &mut self.errors);
            self.identities.color(&expected["_color"], actor.state.temporary_color.as_ref());
            self.identities.array(&expected["_segColor"], actor.body.phenotype_colors.as_ref());
            for (field, actual) in [("_infectedGeneticCode", &actor.infected_genome),
                ("_savedGeneticCode", &actor.saved_genome)] {
                compare_genome_reference(actual.as_ref(), &expected[field], &self.genomes);
            }
            let q: Vec<_> = rows.iter().filter(|r| r[0] == "Q" && r[2] == role).collect();
            assert_eq!(q.len(), 36);
            for row in q {
                let channel = REACTION_NAMES.iter().position(|name|
                    format!("_seg{name}Reaction") == row[3]).unwrap();
                assert_eq!(actor.body.arrays.reactions[channel].as_ref().unwrap().values().iter().map(|value| value.to_string())
                    .collect::<Vec<_>>().join(","), row[4]);
            }
            let genome = actor.genome.as_ref().unwrap().borrow();
            let gene_rows: Vec<_> = rows.iter().filter(|r| r[0] == "N" && r[2] == role).collect();
            assert_eq!(gene_rows.len(), genome.genes().unwrap().len());
            for row in gene_rows {
                self.identities.color(row[4], genome.gene_ref(row[3].parse().unwrap()).unwrap().borrow().color_handle());
            }
            let array_row = rows.iter().find(|r| r[0] == "AR" && r[2] == role).unwrap();
            self.identities.array(array_row[3], actor.body.phenotype_colors.as_ref());
            let slots: Vec<_> = rows.iter().filter(|r| r[0] == "AC" && r[2] == role).collect();
            if let Some(array) = &actor.body.phenotype_colors {
                assert_eq!(array.len(), array_row[4].parse::<usize>().unwrap());
                assert_eq!(slots.len(), array.len());
                for row in slots {
                    let value = array.get(row[3].parse().unwrap()).unwrap();
                    let expected = if row[4] == "~" { "~".into() } else { format!("{}:{}",row[4],row[5]) };
                    self.identities.color(&expected, value.as_ref());
                }
            } else {
                assert_eq!(array_row[4], "~");
                assert!(slots.is_empty());
            }
            let g = rows.iter().find(|r| r[0] == "G" && r[2] == role).unwrap();
            assert!(Rc::ptr_eq(actor.genome.as_ref().unwrap(), &self.genomes[g[3]]));
            let gm = rows.iter().find(|r| r[0] == "GM" && r[2] == role).unwrap();
            assert_eq!(genome.modifiers().mimic_all, gm[3].parse::<bool>().unwrap());
            let b = rows.iter().find(|r| r[0] == "B" && r[2] == role).unwrap();
            assert_eq!(rectangle(actor.state.bounds.get()), b[3..7].join(","));
        }
        let expected: Fields = rows.iter().filter(|r| r[0] == "W")
            .map(|r| (r[2].into(), r[3].into())).collect();
        assert_eq!(expected.len(), 8);
        let mut all = world_fields(&self.world);
        all.insert("nFrames".into(), self.world.frame.get().to_string());
        let actual = expected.keys().map(|key| (key.clone(), all[key].clone())).collect();
        compare_fields(&self.name, stage, "W", &actual, &expected, &mut self.errors);
        let member = rows.iter().find(|r| r[0] == "M").unwrap();
        let actual = self.world.organisms.borrow().iter().map(|reference| self.actors.iter()
            .find_map(|(role, actor)| Rc::ptr_eq(reference, actor).then_some(role.as_str())).unwrap())
            .collect::<Vec<_>>().join(",");
        assert_eq!(actual, member[2]);
        let (types, _) = statistics_schemas();
        let statistics_owner = self.world.statistics_ref();
        let statistics = statistics_owner.borrow();
        let s: Vec<_> = rows.iter().filter(|r| r[0] == "S").collect();
        assert_eq!(s.len(), 52);
        for row in s {
            assert_observation(observed_field(&statistics, row[2]), &types[row[2]], row[3],
                &self.genomes, &format!("{} {stage} {}", self.name, row[2]));
        }
        let rp = rows.iter().find(|r| r[0] == "RP").unwrap();
        let mut random = self.world.random_snapshot();
        assert_eq!(random.next_i64(), rp[2].parse::<i64>().unwrap());
        assert_eq!(random.bounded(10_000), rp[3].parse::<i32>().unwrap());
        assert_eq!(bits(random.next_f64()), rp[4]);
        // R retains the Java serialized-Random provenance; RP checks the model RNG without consuming it
        let r = rows.iter().find(|r| r[0] == "R").unwrap();
        assert_eq!(r[2], "55b2a1d6ca0c49501893ce8d7eefb5efb1718defdab6f19beea2e3e5e7dc682e");
        self.snapshots += 1;
    }
}

fn compare_genome_reference(actual: Option<&GenomeRef>, expected: &str, genomes: &BTreeMap<String, GenomeRef>) {
    if expected == "~" { assert!(actual.is_none()); }
    else { assert!(Rc::ptr_eq(actual.unwrap(), &genomes[expected])); }
}

fn replay(rows: &[Row<'_>]) -> Replay {
    let mut replay = Replay::new(rows);
    let mut snapshot_start = None;
    let mut line_count = 0;
    for (index,row) in rows.iter().enumerate() {
        match row[0] {
            "A" => replay.construct(row),
            "I" => replay.input(row, rows),
            "K" => { replay.call(row); line_count = 0; }
            "X" => replay.outcome(row),
            "P" => { assert!(snapshot_start.replace(index).is_none()); }
            "Z" => {
                let start = snapshot_start.take().unwrap();
                replay.snapshot(&rows[start..=index]);
            }
            "L" => {
                let line = &replay.lines[line_count];
                assert_eq!(row[2].parse::<usize>().unwrap(), line_count);
                assert_eq!([line.x1,line.y1,line.x2,line.y2],
                    [row[3],row[4],row[5],row[6]].map(|v| f64::from(v.parse::<i32>().unwrap())));
                assert_eq!((u32::from(line.alpha)<<24)|line.rgb, row[7].parse::<i32>().unwrap() as u32);
                line_count += 1;
            }
            "D" => {
                assert_eq!(replay.lines.len(), line_count);
                assert_eq!(replay.paint, row[2].parse::<i32>().unwrap() as u32);
            }
            "PX" => {
                let image = replay.image.as_ref().unwrap();
                let x = row[2].parse::<usize>().unwrap();
                let y = row[3].parse::<usize>().unwrap();
                let offset = (y * image.width as usize + x) * 3;
                let pixel = &image.rgb[offset..offset+3];
                assert_eq!(format!("ff{:02x}{:02x}{:02x}",pixel[0],pixel[1],pixel[2]),row[4]);
            }
            "IMG" => {
                let image = replay.image.as_ref().unwrap();
                assert_eq!(image.width, row[3].parse::<u32>().unwrap());
                assert_eq!(image.height, row[4].parse::<u32>().unwrap());
                assert_eq!(rows.iter().filter(|r| r[0]=="PX").count(), 4);
            }
            "C" | "U" | "O" | "AR" | "AC" | "Q" | "GM" | "G" | "N" | "B" |
            "W" | "M" | "S" | "R" | "RP" | "E" => {}
            tag => panic!("Unknown witness row {tag}"),
        }
    }
    assert!(snapshot_start.is_none());
    replay
}

#[test]
fn actual_java_nullable_phenotype_calls_preserve_prefixes_aliases_statistics_and_rng() {
    let cases = cases(SOURCE);
    assert_eq!(SOURCE.lines().count(), 10_952);
    assert_eq!(cases.len(), 12);
    let mut calls = 0;
    let mut snapshots = 0;
    for rows in cases {
        let result = replay(&rows);
        assert!(result.errors.is_empty(), "{}", result.errors.join("\n"));
        calls += result.calls;
        snapshots += result.snapshots;
    }
    assert_eq!((calls,snapshots), (15,27));
    phenotype_color_graph_fixture::verify_graph_fixtures();
}

#[test]
fn nullable_phenotype_checker_rejects_changed_output_energy_without_using_it_as_input() {
    let mut replaced = false;
    let changed = SOURCE.lines().map(|line| {
        if !replaced && line.starts_with("O\tinitial\tvictim\t_energy\t") {
            replaced = true;
            let (prefix, value) = line.rsplit_once('\t').unwrap();
            assert_eq!(value.len(), 16);
            format!("{prefix}\t{:016x}", u64::from_str_radix(value, 16).unwrap() ^ 1)
        } else { line.to_owned() }
    }).collect::<Vec<_>>().join("\n");
    assert!(replaced);
    let input_rows = |source: &str| source.lines().filter(|line|
        ["C", "U", "A", "I", "K"].contains(&line.split('\t').next().unwrap()))
        .map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(input_rows(SOURCE), input_rows(&changed));
    assert_eq!(SOURCE.lines().zip(changed.lines()).filter(|(a,b)| a != b).count(), 1);
    let rows = cases(&changed).remove(0);
    let result = replay(&rows);
    assert_eq!(result.errors.len(), 1, "{}", result.errors.join("\n"));
    assert!(result.errors[0].starts_with("die_whole_null_colors\tinitial\tO\t_energy\t"));
}
