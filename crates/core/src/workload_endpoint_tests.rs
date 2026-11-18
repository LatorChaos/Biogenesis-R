// SPDX-License-Identifier: GPL-2.0-or-later

//! Required expensive endpoint gate; run only through tools/check_workload_endpoints.py

use super::*;
use crate::GeneRef;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{Observation, observed_field, observed_getter};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

const HISTORY_FIELDS: [&str; 6] = [
    "populationList",
    "deathList",
    "birthList",
    "oxygenList",
    "carbonDioxideList",
    "methaneList",
];

const GENOME_HOLDERS: [&str; 9] = [
    "aliveBeingMostChildren",
    "aliveBeingMostInfections",
    "aliveBeingMostKills",
    "beingMostChildren",
    "beingMostInfections",
    "beingMostKills",
    "lastBornBeing",
    "lastDeadBeing",
    "lastInfectedBeing",
];
const W_FIELDS: [&str; 10] = [
    "_population",
    "organism_count",
    "corpses",
    "_O2",
    "_CO2",
    "_CH4",
    "NEXT_ID",
    "nFrames",
    "time",
    "organism_ids",
];

fn pointer<T>(value: &Rc<RefCell<T>>) -> usize {
    Rc::as_ptr(value) as usize
}
fn label(prefix: &str, index: usize) -> String {
    format!("{prefix}{index}")
}
fn index(value: &str, prefix: &str) -> usize {
    let suffix = value
        .strip_prefix(prefix)
        .expect("Missing reference prefix");
    let result: usize = suffix.parse().expect("Invalid reference index");
    assert_eq!(value, label(prefix, result), "Noncanonical reference label");
    result
}
fn difference(field: &str, actual: &str, expected: &str, geometry: bool) -> Option<String> {
    if actual == expected {
        return None;
    }
    let a: Vec<_> = actual.split(',').collect();
    let b: Vec<_> = expected.split(',').collect();
    if a.len() != b.len() {
        return Some(format!(
            "{field}: component count actual={} expected={}",
            a.len(),
            b.len()
        ));
    }
    for (component, (a, b)) in a.into_iter().zip(b).enumerate() {
        if a == b {
            continue;
        }
        let close = geometry && a != "~" && b != "~" && {
            let a = decoded(a);
            let b = decoded(b);
            if b.is_nan() {
                a.is_nan()
            } else if b.is_infinite() {
                a == b
            } else {
                a.is_finite() && (a - b).abs() <= 1e-10 * b.abs().max(1.0)
            }
        };
        if !close {
            return Some(format!("{field}[{component}]: actual={a} expected={b}"));
        }
    }
    None
}
fn check(context: &str, field: &str, actual: &str, expected: &str, geometry: bool) {
    if let Some(message) = difference(field, actual, expected, geometry) {
        panic!("{context} {message}");
    }
}

struct Graph {
    world: World,
    actors: Vec<OrganismRef>,
    genomes: Vec<GenomeRef>,
    genes: Vec<GeneRef>,
    actor_indices: HashMap<usize, usize>,
    genome_indices: HashMap<usize, usize>,
    gene_indices: HashMap<usize, usize>,
    tree: Vec<(usize, [f64; 4])>,
}
impl Graph {
    fn run(settings: &Fields, seed: i64, attempts: usize) -> Self {
        assert_eq!(integer(settings, "INITIAL_ORGANISMS"), attempts as i32);
        let world = World::with_parameters(
            WorldConfig {
                seed,
                width: integer(settings, "WORLD_WIDTH").try_into().unwrap(),
                height: integer(settings, "WORLD_HEIGHT").try_into().unwrap(),
                initial_attempts: attempts.try_into().unwrap(),
            },
            parameters(settings),
        )
        .expect("Actual seeded World construction failed");
        for frame in 0..80 {
            assert!(
                world
                    .single_step()
                    .unwrap_or_else(|error| panic!("attempts={attempts} frame={frame}: {error}"))
            );
        }
        assert_eq!(world.frame(), 80);
        assert_eq!(world.time(), 0);
        Self::from_world(world)
    }
    fn from_world(world: World) -> Self {
        let stored = world.spatial.borrow().stored_envelopes();
        let mut by_token = BTreeMap::new();
        for (token, envelope) in stored {
            assert!(by_token.insert(token, envelope).is_none());
            assert!(envelope.iter().all(|value| value.is_finite()));
        }
        // Query only the union of actual retained Rust envelopes to expose packed traversal order
        let ordered = if by_token.is_empty() {
            Vec::new()
        } else {
            let min_x = by_token
                .values()
                .map(|v| v[0])
                .fold(f64::INFINITY, f64::min);
            let min_y = by_token
                .values()
                .map(|v| v[1])
                .fold(f64::INFINITY, f64::min);
            let max_x = by_token
                .values()
                .map(|v| v[0] + v[2])
                .fold(f64::NEG_INFINITY, f64::max);
            let max_y = by_token
                .values()
                .map(|v| v[1] + v[3])
                .fold(f64::NEG_INFINITY, f64::max);
            let exact = |value: f64| {
                assert!(value >= f64::from(i32::MIN) && value <= f64::from(i32::MAX));
                let result = value as i32;
                assert_eq!(f64::from(result), value);
                result
            };
            let bounds = IntRect {
                x: exact(min_x),
                y: exact(min_y),
                width: exact(max_x - min_x),
                height: exact(max_y - min_y),
            };
            let order = world.spatial.borrow().query(bounds);
            assert_eq!(order.len(), by_token.len());
            assert_eq!(
                order.iter().copied().collect::<BTreeSet<_>>(),
                by_token.keys().copied().collect()
            );
            order
        };
        let mut actors = world.organisms.borrow().clone();
        let mut actor_indices: HashMap<_, _> = actors
            .iter()
            .enumerate()
            .map(|(i, a)| (pointer(a), i))
            .collect();
        assert_eq!(actor_indices.len(), actors.len());
        for token in &ordered {
            let actor = Rc::clone(&world.frame_objects.borrow()[*token]);
            if !actor_indices.contains_key(&pointer(&actor)) {
                actor_indices.insert(pointer(&actor), actors.len());
                actors.push(Rc::clone(&actor));
            }
        }
        let tree = ordered
            .into_iter()
            .map(|token| {
                let [x, y, w, h] = by_token[&token];
                (
                    actor_indices[&pointer(&world.frame_objects.borrow()[token])],
                    [x, x + w, y, y + h],
                )
            })
            .collect();
        let mut genomes = Vec::new();
        let mut genome_indices = HashMap::new();
        let mut register = |code: &GenomeRef| {
            if !genome_indices.contains_key(&pointer(code)) {
                genome_indices.insert(pointer(code), genomes.len());
                genomes.push(Rc::clone(code));
            }
        };
        for actor in &actors {
            let actor = actor.borrow();
            if let Some(code) = &actor.genome { register(code); }
            if let Some(code) = &actor.infected_genome {
                register(code)
            }
            if let Some(code) = &actor.saved_genome {
                register(code)
            }
        }
        for field in GENOME_HOLDERS {
            let statistics_owner = world.statistics_ref();
            let statistics = statistics_owner.borrow();
            let Observation::Genome(code) = observed_field(&statistics, field) else {
                unreachable!()
            };
            if let Some(code) = code {
                register(code)
            }
        }
        let mut genes = Vec::new();
        let mut gene_indices = HashMap::new();
        for code in &genomes {
            for gene in code.borrow().genes().unwrap().iter() {
                if !gene_indices.contains_key(&pointer(gene)) {
                    gene_indices.insert(pointer(gene), genes.len());
                    genes.push(Rc::clone(gene));
                }
            }
        }
        Self {
            world,
            actors,
            genomes,
            genes,
            actor_indices,
            genome_indices,
            gene_indices,
            tree,
        }
    }
    fn genome_label(&self, code: Option<&GenomeRef>) -> String {
        code.map_or_else(
            || "~".into(),
            |code| label("g", self.genome_indices[&pointer(code)]),
        )
    }
    fn actor_fields(&self, actor: usize) -> Fields {
        let actor = self.actors[actor].borrow();
        let mut fields = organism_fields(&View::from(&*actor));
        fields.insert(
            "_geneticCode".into(),
            self.genome_label(actor.genome.as_ref()),
        );
        fields.insert(
            "_infectedGeneticCode".into(),
            self.genome_label(actor.infected_genome.as_ref()),
        );
        fields.insert(
            "_savedGeneticCode".into(),
            self.genome_label(actor.saved_genome.as_ref()),
        );
        // World owns every current or frozen actor; the model has no redundant back-pointer
        fields.insert("_world".into(), "w0".into());
        fields.insert("_visibleWorld".into(), "absent".into());
        fields
    }
    fn observation(&self, value: Observation<'_>) -> (&'static str, String) {
        match value {
            Observation::Int(v) => ("int", v.to_string()),
            Observation::Long(v) => ("long", v.to_string()),
            Observation::Double(v) => ("double", bits(v)),
            Observation::Genome(v) => ("GeneticCode", self.genome_label(v)),
            Observation::History(v) => ("List", array(v.expect("Pinned endpoint histories are non-null").numeric_prefix("endpoint", usize::MAX).unwrap().into_iter().map(bits))),
        }
    }
    fn world_value(&self, field: &str) -> (&'static str, String) {
        let w = &self.world;
        match field {
            "_width" => ("int", w.config.get().width.to_string()),
            "_height" => ("int", w.config.get().height.to_string()),
            "NEXT_ID" => ("int", w.next_id.get().to_string()),
            "_population" => ("int", w.population.get().to_string()),
            "_O2" => ("double", bits(w.oxygen.get())),
            "_CO2" => ("double", bits(w.carbon_dioxide.get())),
            "_CH4" => ("double", bits(w.methane.get())),
            "nFrames" => ("int", w.frame.get().to_string()),
            "_organisms" => (
                "List",
                array(
                    w.organisms.borrow()
                        .iter()
                        .map(|a| label("o", self.actor_indices[&pointer(a)])),
                ),
            ),
            "colDetTree" => ("STRtree", format!("present,{}", self.tree.len())),
            "worldStatistics" => ("WorldStatistics", "s0".into()),
            "_corridorexists" => ("boolean", w.corridors_exist.get().to_string()),
            "_visibleWorld" => ("VisibleWorld", "absent".into()),
            "inCorridors" | "outCorridors" => ("List", "absent".into()),
            _ => panic!("Unknown World field {field}"),
        }
    }
    fn getter(&self, field: &str) -> (&'static str, String) {
        match field {
            "organism_count" => ("int", self.world.organisms.borrow().len().to_string()),
            "corpses" => ("int", self.world.corpse_count().unwrap().to_string()),
            "nFrames" => ("long", self.world.frame.get().to_string()),
            "time" => ("long", self.world.time().to_string()),
            "organism_ids" => (
                "int[]",
                array(
                    self.world.organisms.borrow()
                        .iter()
                        .map(|a| a.borrow().id.to_string()),
                ),
            ),
            _ => self.world_value(field),
        }
    }
}

impl Graph {
    fn diagnostic(
        &self,
        actor: usize,
        fields: &Fields,
        schema: &BTreeMap<String, String>,
        context: &str,
    ) {
        let Some(path) = std::env::var_os("BGR_ENDPOINT_DIAGNOSTIC_FILE") else {
            return;
        };
        let mut output =
            BufWriter::new(File::create_new(path).expect("Create owned mismatch diagnostic"));
        writeln!(output, "D\t{context}\t{}", label("o", actor)).unwrap();
        for (field, value) in fields {
            writeln!(
                output,
                "O\tactual\t{}\t{field}\t{}\t{value}",
                label("o", actor),
                schema[field]
            )
            .unwrap();
        }
        let actor = self.actors[actor].borrow();
        let mut written = HashSet::new();
        for (holder, code) in [
            ("genotype", actor.genome.as_ref()),
            ("infected", actor.infected_genome.as_ref()),
            ("saved", actor.saved_genome.as_ref()),
        ] {
            let name = self.genome_label(code);
            writeln!(output, "H\tactual\t{holder}\t{name}").unwrap();
            if let Some(code) = code {
                if written.insert(pointer(code)) {
                    let code = code.borrow();
                    let mut values = genome_fields(&code);
                    values.insert("_reference".into(), name.clone());
                    values.insert("_nGenes".into(), code.genes().unwrap().len().to_string());
                    for (field, value) in values {
                        writeln!(output, "G\tactual\t{name}\t{field}\t{value}").unwrap();
                    }
                    for (i, gene) in code.genes().unwrap().iter().enumerate() {
                        writeln!(
                            output,
                            "J\t{name}\t{i}\t{}",
                            label("n", self.gene_indices[&pointer(gene)])
                        )
                        .unwrap();
                        for (field, value) in gene_fields(&gene.borrow()) {
                            writeln!(output, "N\tactual\t{name}\t{i}\t{field}\t{value}").unwrap();
                        }
                    }
                }
            }
        }
        for field in W_FIELDS {
            let (kind, value) = self.getter(field);
            writeln!(output, "W\tactual\t{field}\t{kind}\t{value}").unwrap();
        }
        output.flush().unwrap();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Reference {
    World,
    Statistics,
    Tree,
    Actor(usize),
    Genome(usize),
    Gene(usize),
    Members,
    History(usize),
    BoxedDouble(usize),
    GeneSlots(usize),
    ActorArray(usize, String),
    Rectangle(usize, String),
    Immutable(&'static str, String),
}
impl Graph {
    fn history(&self, identity: usize) -> crate::history::HistoryListRef {
        let owner = self.world.statistics_ref();
        let statistics = owner.borrow();
        HISTORY_FIELDS
            .into_iter()
            .find_map(|field| {
                let Observation::History(value) = observed_field(&statistics, field) else {
                    unreachable!()
                };
                value.filter(|value| value.identity() == identity).cloned()
            })
            .expect("Actual history owner must be reachable from statistics")
    }
    fn holder(&self, actor: usize, name: &str) -> Option<GenomeRef> {
        let a = self.actors[actor].borrow();
        match name {
            "genotype" | "_geneticCode" => Some(Rc::clone(a.genome.as_ref().unwrap())),
            "infected" | "_infectedGeneticCode" => a.infected_genome.clone(),
            "saved" | "_savedGeneticCode" => a.saved_genome.clone(),
            _ => panic!("Unknown holder {name}"),
        }
    }
    fn code_ref(&self, code: &GenomeRef) -> Reference {
        Reference::Genome(self.genome_indices[&pointer(code)])
    }
    fn reference(
        &self,
        owner: &str,
        name: &str,
        field: &str,
        bindings: &HashMap<String, Reference>,
        schema: &BTreeMap<String, String>,
    ) -> Option<(String, Reference)> {
        let mutability = |kind: &str, value: Reference| Some((kind.to_owned(), value));
        match owner {
            "V" => {
                assert_eq!(name, "w0");
                match field {
                    "_organisms" => mutability("List", Reference::Members),
                    "worldStatistics" => mutability("WorldStatistics", Reference::Statistics),
                    "colDetTree" => mutability("STRtree", Reference::Tree),
                    _ => panic!("Unknown World reference field {field}"),
                }
            }
            "O" => {
                let actor = index(name, "o");
                let a = self.actors[actor].borrow();
                let kind = &schema[field];
                match kind.as_str() {
                    "World" => mutability(kind, Reference::World),
                    "String" => {
                        assert_eq!(field, "_nPosition");
                        a.state.network_position.as_ref().map(|value| {
                            (kind.clone(), Reference::Immutable("String", value.to_utf8().expect("Fixture positions use valid UTF16")))
                        })
                    }
                    "GeneticCode" => self
                        .holder(actor, field)
                        .map(|code| (kind.clone(), self.code_ref(&code))),
                    "Rectangle" => mutability(kind, Reference::Rectangle(actor, field.into())),
                    "Color" => a
                        .state
                        .temporary_argb()
                        .map(|v| (kind.clone(), Reference::Immutable("Color", color(Some(v))))),
                    "int[]" | "double[]" | "Color[]" => {
                        mutability(kind, Reference::ActorArray(actor, field.into()))
                    }
                    _ => panic!("Unknown actor reference {field}:{kind}"),
                }
            }
            "G" => {
                let i = index(name, "g");
                let code = self.genomes[i].borrow();
                match field {
                    "_genes" => mutability("Gene[]", Reference::GeneSlots(i)),
                    "_cladeID" => code
                        .clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref()
                        .map(|v| ("String".into(), Reference::Immutable("String", v.into()))),
                    _ => panic!("Unknown genome reference {field}"),
                }
            }
            "N" => {
                assert_eq!(field, "_color");
                let gene = self.genes[index(name, "n")].borrow();
                gene.color().map(|v| {
                    (
                        "Color".into(),
                        Reference::Immutable("Color", color(Some(v))),
                    )
                })
            }
            "S" => {
                assert_eq!(name, "s0");
                match observed_field(&self.world.statistics_ref().borrow(), field) {
                    Observation::Genome(code) => {
                        code.map(|code| ("GeneticCode".into(), self.code_ref(code)))
                    }
                    Observation::History(value) => value
                        .map(|value| ("List".into(), Reference::History(value.identity()))),
                    _ => panic!("Unknown statistics reference {field}"),
                }
            }
            "container" => {
                let element: usize = field.parse().expect("Invalid container index");
                match &bindings[name] {
                    Reference::Members => {
                        let actor = &self.world.organisms.borrow()[element];
                        mutability(
                            "Organism",
                            Reference::Actor(self.actor_indices[&pointer(actor)]),
                        )
                    }
                    Reference::GeneSlots(i) => {
                        let code = self.genomes[*i].borrow();
                        let gene = &code.genes().unwrap()[element];
                        mutability("Gene", Reference::Gene(self.gene_indices[&pointer(gene)]))
                    }
                    Reference::History(identity) => {
                        let history = self.history(*identity);
                        history
                            .shared_ref()
                            .number("endpoint", element)
                            .unwrap()
                            .map(|value| ("Double".into(), Reference::BoxedDouble(value.identity())))
                    }
                    Reference::ActorArray(i, field) => {
                        assert_eq!(field, "_segColor");
                        let actor = self.actors[*i].borrow();
                        mutability(
                            "Color",
                            Reference::Immutable(
                                "Color",
                                color(Some(actor.body.color_argb(element).unwrap())),
                            ),
                        )
                    }
                    _ => panic!("Unexpected reference-bearing container {name}"),
                }
            }
            _ => panic!("Unknown alias owner {owner}"),
        }
    }
    fn alias_counts(&self, schema: &BTreeMap<String, String>) -> (usize, usize, usize) {
        let mut mutable = 3 + self.world.organisms.borrow().len();
        let mut immutable = 0;
        let mut boxed = 0;
        let mut history_owners = HashSet::new();
        for (i, actor) in self.actors.iter().enumerate() {
            let actor = actor.borrow();
            for (field, kind) in schema {
                if field == "bounds"
                    || field == "_visibleWorld"
                    || ["boolean", "int", "long", "double"].contains(&kind.as_str())
                {
                    continue;
                }
                let value = self.reference("O", &label("o", i), field, &HashMap::new(), schema);
                if let Some((_, value)) = value {
                    if matches!(value, Reference::Immutable(..)) {
                        immutable += 1
                    } else {
                        mutable += 1
                    }
                }
            }
            immutable += actor.body.forward_count();
        }
        for field in HISTORY_FIELDS {
            let statistics_owner = self.world.statistics_ref();
            let statistics = statistics_owner.borrow();
            let Observation::History(values) = observed_field(&statistics, field) else {
                unreachable!()
            };
            let values = values.expect("Pinned endpoint histories are non-null");
            if self.world.time() == 0 {
                assert!(
                    values.is_empty(),
                    "Pinned endpoint histories must be empty at time0"
                );
            }
            if history_owners.insert(values.identity()) {
                boxed += values.elements().iter().filter(|value| value.is_some()).count();
            }
            mutable += 1;
        }
        for field in GENOME_HOLDERS {
            let statistics_owner = self.world.statistics_ref();
            let statistics = statistics_owner.borrow();
            let Observation::Genome(code) = observed_field(&statistics, field) else {
                unreachable!()
            };
            mutable += usize::from(code.is_some());
        }
        for code in &self.genomes {
            let code = code.borrow();
            mutable += 1 + code.genes().unwrap().len();
            immutable += usize::from(code.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref().is_some());
        }
        immutable += self
            .genes
            .iter()
            .filter(|gene| gene.borrow().color().is_some())
            .count();
        (mutable, immutable, boxed)
    }
}

fn expected_rows(attempts: usize) -> BTreeMap<&'static str, usize> {
    let (actors, members, tree, codes, slots, aliases, x) = match attempts {
        0 => (0, 0, 0, 0, 0, 9, 6),
        32 => (32, 32, 32, 32, 49, 2131, 38),
        512 => (541, 541, 541, 522, 816, 35774, 547),
        12500 => (13192, 13190, 13191, 12729, 19131, 865866, 13198),
        _ => panic!("Unknown required workload"),
    };
    BTreeMap::from([
        ("C", 1),
        ("X", x),
        ("U", 147),
        ("M", members),
        ("F", tree),
        ("V", 15),
        ("A", aliases),
        ("W", 10),
        ("O", actors * 154),
        ("H", actors * 3),
        ("S", 52),
        ("T", 49),
        ("G", codes * 28),
        ("J", slots),
        ("N", slots * 40),
        ("E", 1),
    ])
}

#[test]
#[ignore = "Required expensive standalone gate: tools/check_workload_endpoints.py"]
fn workload_endpoints() {
    let attempts: usize = std::env::var("BGR_ENDPOINT_ATTEMPTS")
        .expect("Required endpoint runner sets attempts").parse().unwrap();
    compare_endpoint(attempts, false, None);
}

fn dense_expected_rows() -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("A", 873455),
        ("C", 1),
        ("D", 6),
        ("E", 1),
        ("F", 13278),
        ("G", 359828),
        ("H", 39843),
        ("J", 19319),
        ("L", 6),
        ("M", 13279),
        ("N", 772760),
        ("O", 2045274),
        ("S", 52),
        ("T", 49),
        ("U", 147),
        ("V", 15),
        ("W", 10),
        ("X", 13287),
    ])
}

#[test]
#[ignore = "Required expensive dense rollover gate: tools/check_workload_endpoints.py --dense-rollover"]
fn dense_world_first_rollover_matches_original_complete_state() {
    let (world, settings) = super::workload_trajectory_tests::dense_rollover_world(include_str!(
        "../tests/fixtures/workload_trajectory_12500_256.tsv"
    ));
    assert_eq!((world.frame(), world.time()), (0, 1));
    compare_endpoint(12500, true, Some((Graph::from_world(world), settings)));
}

fn compare_endpoint(attempts: usize, dense256: bool, mut supplied: Option<(Graph, Fields)>) {
    let expected = if dense256 {
        assert_eq!(attempts, 12500);
        dense_expected_rows()
    } else {
        expected_rows(attempts)
    };
    let path = std::env::var_os("BGR_ENDPOINT_FILE")
        .expect("Required endpoint runner supplies a verified private fixture");
    let file = File::open(path).unwrap();
    let (setting_schema, organism_schema) = primitive_reference_schemas();
    let o_order: Vec<_> = organism_schema
        .keys()
        .filter(|key| key.as_str() != "bounds")
        .cloned()
        .chain(std::iter::once("bounds".into()))
        .collect();
    let mut selected = false;
    let mut complete = false;
    let mut seed = 0;
    let mut settings = Fields::new();
    let mut graph: Option<Graph> = None;
    let mut counts: BTreeMap<&str, usize> = expected.keys().map(|key| (*key, 0)).collect();
    let mut bindings: HashMap<String, Reference> = HashMap::new();
    let mut reverse: HashMap<Reference, String> = HashMap::new();
    let mut alias_edges = HashSet::new();
    let mut aliases = [0usize; 2];
    let mut boxed_aliases = 0;
    let mut history_slots = HashMap::new();
    let mut boxed_values = HashSet::new();
    let mut last_s = String::new();
    let mut last_t = String::new();
    let mut seen_v = BTreeSet::new();
    let mut seen_x = HashSet::new();
    let mut actor_cache = (usize::MAX, Fields::new());
    let mut genome_cache = (usize::MAX, Fields::new());
    let mut gene_cache = (String::new(), Fields::new());
    let mut current_slot: Option<(usize, usize)> = None;
    let mut seen_slots = HashSet::new();
    let mut o_count = 0;
    let mut g_count = 0;
    let mut n_count = 0;
    let mut h_count = 0;
    let mut j_count = 0;
    for (line_number, line) in BufReader::new(file).lines().enumerate() {
        let line = line.unwrap();
        assert!(line.len() <= 1_000_000, "Endpoint line exceeds bound");
        let row: Vec<_> = line.split('\t').collect();
        if row[0] == "C" {
            assert_eq!(row.len(), 4);
            selected = row[1] == format!("workload-{attempts}");
            if selected {
                assert!(!complete);
                assert_eq!(row[2], if dense256 { "endpoint256" } else { "endpoint80" });
                seed = row[3].parse().unwrap();
                assert_eq!(seed, 827364);
                *counts.get_mut("C").unwrap() += 1;
            }
            continue;
        }
        if !selected {
            continue;
        }
        let context = format!(
            "attempts={attempts} line={} {} {}",
            line_number + 1,
            row[0],
            if ["O", "H", "G", "N"].contains(&row[0]) {
                row.get(2).copied().unwrap_or("?")
            } else {
                ""
            }
        );
        assert!(expected.contains_key(row[0]), "{context}: Unknown row");
        *counts.get_mut(row[0]).unwrap() += 1;
        match row[0] {
            "U" => {
                assert_eq!(row.len(), 4);
                assert_eq!(row[2], setting_schema[row[1]]);
                assert!(graph.is_none());
                assert!(settings.insert(row[1].into(), row[3].into()).is_none());
            }
            "X" => {
                assert_eq!(row.len(), 4);
                assert!(seen_x.insert((row[1].to_owned(), row[2].to_owned())));
                match (row[1], row[2], row[3]) {
                    (
                        "Utils",
                        "settings",
                        "reconstructed-pinned-preferences-and-source-defaults",
                    )
                    | ("Utils", "random", "static-absent-no-final-RNG-observation")
                    | ("Organism", "v", "static-transient-absent") => {}
                    (
                        "w0",
                        "_visibleWorld" | "inCorridors" | "outCorridors",
                        "transient-absent",
                    ) => {}
                    (owner, "_visibleWorld", "transient-absent") => {
                        let g = graph.as_ref().unwrap();
                        assert!(index(owner, "o") < g.actors.len());
                    }
                    _ => panic!("{context}: Unknown missing-state disposition"),
                }
            }
            _ => {
                if graph.is_none() {
                    assert_eq!(settings.len(), 147);
                    assert!(setting_schema.keys().eq(settings.keys()));
                    graph = Some(if dense256 {
                        let (graph, declared) = supplied.take().expect("Dense endpoint uses the trajectory World");
                        assert_eq!(settings, declared, "Both source fixtures must declare identical constructor inputs");
                        assert_eq!((graph.world.frame(), graph.world.time()), (0, 1));
                        graph
                    } else { Graph::run(&settings, seed, attempts) });
                }
                let g = graph.as_ref().unwrap();
                match row[0] {
                    "M" => {
                        assert_eq!(row.len(), 3);
                        let ordinal: usize = row[1].parse().unwrap();
                        assert_eq!(ordinal, counts["M"] - 1);
                        let actual =
                            label("o", g.actor_indices[&pointer(&g.world.organisms.borrow()[ordinal])]);
                        check(&context, "membership", &actual, row[2], false);
                    }
                    "F" => {
                        assert_eq!(row.len(), 7);
                        let ordinal: usize = row[1].parse().unwrap();
                        assert_eq!(ordinal, counts["F"] - 1);
                        let (actor, envelope) = g.tree[ordinal];
                        check(&context, "tree actor", &label("o", actor), row[2], false);
                        for (i, value) in envelope.iter().enumerate() {
                            check(
                                &context,
                                &format!("envelope[{i}]"),
                                &bits(*value),
                                row[i + 3],
                                false,
                            );
                        }
                    }
                    "V" => {
                        assert_eq!(row.len(), 6);
                        assert_eq!(row[1], "endpoint");
                        assert_eq!(row[2], "w0");
                        assert!(seen_v.insert(row[3].to_owned()));
                        let (kind, value) = g.world_value(row[3]);
                        assert_eq!(row[4], kind);
                        check(&context, row[3], &value, row[5], false);
                    }
                    "W" => {
                        assert_eq!(row.len(), 5);
                        assert_eq!(row[1], "endpoint");
                        assert_eq!(row[2], W_FIELDS[counts["W"] - 1]);
                        let (kind, value) = g.getter(row[2]);
                        assert_eq!(row[3], kind);
                        check(&context, row[2], &value, row[4], false);
                    }
                    "O" => {
                        assert_eq!(row.len(), 6);
                        assert_eq!(row[1], "endpoint");
                        let actor = index(row[2], "o");
                        assert_eq!(actor, o_count / 154);
                        assert_eq!(row[3], o_order[o_count % 154]);
                        assert_eq!(row[4], organism_schema[row[3]]);
                        if actor_cache.0 != actor {
                            actor_cache = (actor, g.actor_fields(actor));
                        }
                        if let Some(message) = difference(
                            row[3],
                            &actor_cache.1[row[3]],
                            row[5],
                            ["_m", "_m1", "_m2", "_mass", "_I"].contains(&row[3]),
                        ) {
                            g.diagnostic(actor, &actor_cache.1, &organism_schema, &context);
                            panic!("{context} {message}");
                        }
                        o_count += 1;
                    }
                    "H" => {
                        assert_eq!(row.len(), 5);
                        assert_eq!(row[1], "endpoint");
                        let actor = index(row[2], "o");
                        assert_eq!(actor, h_count / 3);
                        assert_eq!(row[3], ["genotype", "infected", "saved"][h_count % 3]);
                        let holder = g.holder(actor, row[3]);
                        check(
                            &context,
                            row[3],
                            &g.genome_label(holder.as_ref()),
                            row[4],
                            false,
                        );
                        h_count += 1;
                    }
                    "S" | "T" => {
                        assert_eq!(row.len(), 5);
                        assert_eq!(row[1], "endpoint");
                        let previous = if row[0] == "S" {
                            &mut last_s
                        } else {
                            &mut last_t
                        };
                        assert!(row[2] > &previous[..]);
                        *previous = row[2].into();
                        let statistics_owner = g.world.statistics_ref();
                        let statistics = statistics_owner.borrow();
                        let value = if row[0] == "S" {
                            observed_field(&statistics, row[2])
                        } else {
                            observed_getter(&statistics, row[2])
                        };
                        let (kind, value) = g.observation(value);
                        assert_eq!(row[3], kind);
                        check(&context, row[2], &value, row[4], false);
                    }
                    "G" => {
                        assert_eq!(row.len(), 5);
                        assert_eq!(row[1], "endpoint");
                        let i = index(row[2], "g");
                        assert_eq!(i, g_count / 28);
                        if genome_cache.0 != i {
                            let code = g.genomes[i].borrow();
                            let mut data = genome_fields(&code);
                            data.insert("_reference".into(), label("g", i));
                            data.insert("_nGenes".into(), code.genes().unwrap().len().to_string());
                            genome_cache = (i, data);
                        }
                        let order: Vec<_> = ["_reference", "_nGenes"]
                            .into_iter()
                            .map(str::to_owned)
                            .chain(
                                genome_cache
                                    .1
                                    .keys()
                                    .filter(|key| {
                                        !["_reference", "_nGenes"].contains(&key.as_str())
                                    })
                                    .cloned(),
                            )
                            .collect();
                        assert_eq!(row[3], order[g_count % 28]);
                        check(&context, row[3], &genome_cache.1[row[3]], row[4], false);
                        g_count += 1;
                    }
                    "J" => {
                        assert_eq!(row.len(), 4);
                        let code = index(row[1], "g");
                        let slot: usize = row[2].parse().unwrap();
                        assert_eq!(code + 1, g_count / 28);
                        assert!(
                            seen_slots.insert((code, slot)),
                            "{context}: Duplicate gene slot"
                        );
                        let code_ref = g.genomes[code].borrow();
                        let gene = &code_ref.genes().unwrap()[slot];
                        check(
                            &context,
                            "Gene alias",
                            &label("n", g.gene_indices[&pointer(gene)]),
                            row[3],
                            false,
                        );
                        current_slot = Some((code, slot));
                        gene_cache = (row[3].into(), gene_fields(&gene.borrow()));
                        j_count += 1;
                    }
                    "N" => {
                        assert_eq!(row.len(), 6);
                        assert_eq!(row[1], "endpoint");
                        assert_eq!(
                            Some((index(row[2], "g"), row[3].parse().unwrap())),
                            current_slot
                        );
                        assert_eq!(row[4], gene_cache.1.keys().nth(n_count % 40).unwrap());
                        check(&context, row[4], &gene_cache.1[row[4]], row[5], false);
                        n_count += 1;
                    }
                    "L" => {
                        assert_eq!(row.len(), 5);
                        assert_eq!(row[1], "endpoint");
                        let Reference::History(identity) = &bindings[row[2]] else {
                            panic!("History owner binding required")
                        };
                        let history = g.history(*identity);
                        assert_eq!(row[3], history.shared_ref().class_name());
                        assert_eq!(row[4].parse::<usize>().unwrap(), history.len());
                        assert!(history_slots.insert(*identity, 0usize).is_none(), "Duplicate history owner");
                    }
                    "D" => {
                        assert_eq!(row.len(), 6);
                        assert_eq!(row[1], "endpoint");
                        let Reference::History(identity) = &bindings[row[2]] else {
                            panic!("History owner binding required")
                        };
                        let count = history_slots.get_mut(identity).expect("History slots follow their L row");
                        assert_eq!(row[3].parse::<usize>().unwrap(), *count);
                        let history = g.history(*identity);
                        let value = history.shared_ref().number("endpoint", *count).unwrap();
                        if row[4] == "~" {
                            assert_eq!(row[5], "~");
                            assert!(value.is_none());
                        } else {
                            let value = value.expect("Actual boxed history element required");
                            assert_eq!(bindings[row[4]], Reference::BoxedDouble(value.identity()));
                            check(&context, "history raw bits", &format!("{:016x}", value.bits()), row[5], false);
                            boxed_values.insert(value.identity());
                        }
                        *count += 1;
                    }
                    "A" => {
                        assert_eq!(row.len(), 7);
                        let (kind, reference) = g
                            .reference(row[1], row[2], row[3], &bindings, &organism_schema)
                            .expect("Source reference is unexpectedly null in Rust");
                        assert_eq!(kind, row[4], "{context}");
                        let immutable = matches!(reference, Reference::Immutable(..));
                        let boxed = matches!(reference, Reference::BoxedDouble(..));
                        assert_eq!(row[6], if immutable || boxed { "immutable" } else { "mutable" });
                        if boxed {
                            boxed_aliases += 1;
                        } else {
                            aliases[usize::from(immutable)] += 1;
                        }
                        let owner = if row[1] == "container" {
                            format!("{:?}", bindings[row[2]])
                        } else {
                            format!("{}:{}", row[1], row[2])
                        };
                        assert!(
                            alias_edges.insert((owner, row[3].to_owned())),
                            "{context}: Duplicate logical reference edge"
                        );
                        if let Some(previous) = bindings.get(row[5]) {
                            assert_eq!(
                                previous, &reference,
                                "{context}: Real source sharing differs"
                            )
                        } else {
                            if !immutable {
                                assert!(
                                    reverse.insert(reference.clone(), row[5].into()).is_none(),
                                    "{context}: Distinct source objects alias in Rust"
                                );
                            }
                            bindings.insert(row[5].into(), reference.clone());
                        }
                        let canonical = match reference {
                            Reference::World => Some("w0".into()),
                            Reference::Statistics => Some("s0".into()),
                            Reference::Tree => Some("t0".into()),
                            Reference::Actor(i) => Some(label("o", i)),
                            Reference::Genome(i) => Some(label("g", i)),
                            Reference::Gene(i) => Some(label("n", i)),
                            _ => None,
                        };
                        if let Some(canonical) = canonical {
                            check(&context, "reference identity", &canonical, row[5], false);
                        }
                    }
                    "E" => {
                        assert_eq!(row.len(), 1);
                        assert_eq!(counts, expected, "Complete endpoint row ledger");
                        let (mutable, immutable, boxed) = g.alias_counts(&organism_schema);
                        assert_eq!(boxed_aliases, boxed, "Complete boxed history alias coverage");
                        assert_eq!(
                            aliases,
                            [mutable, immutable],
                            "Complete actual reference edge coverage"
                        );
                        assert_eq!(o_count, g.actors.len() * 154);
                        assert_eq!(h_count, g.actors.len() * 3);
                        assert_eq!(g_count, g.genomes.len() * 28);
                        let slots: usize = g
                            .genomes
                            .iter()
                            .map(|code| code.borrow().genes().unwrap().len())
                            .sum();
                        assert_eq!(j_count, slots);
                        assert_eq!(n_count, slots * 40);
                        if dense256 {
                            assert_eq!(history_slots.len(), 6);
                            for (identity, count) in &history_slots {
                                assert_eq!(*count, g.history(*identity).len());
                            }
                            assert_eq!(boxed_values.len(), 6);
                            assert_eq!(boxed_aliases, 6);
                            assert_eq!(aliases, [749803, 123646]);
                            assert_eq!(counts.values().sum::<usize>(), 4150610);
                            assert_eq!(4150610 - aliases[1] - (g.actors.len() + 3) - counts["X"] - 149, 4000244);
                        }
                        complete = true;
                        break;
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
    assert!(complete, "Required endpoint not found or incomplete");
    if dense256 {
        println!("DENSE_ROLLOVER_ENDPOINT PASS attempts=12500 ticks=256 rows=4150610 derived_comparisons=4000244 mutable_aliases=749803 immutable_alias_metadata=123646 boxed_aliases=6 absent_fields=13284 absent_metadata=13287 input_settings=149");
    } else {
        println!(
            "WORKLOAD_ENDPOINT PASS attempts={attempts} rows={} mutable_aliases={} immutable_alias_metadata={} absent_fields={} absent_metadata={} input_settings=149",
            counts.values().sum::<usize>(),
            aliases[0],
            aliases[1],
            graph.as_ref().unwrap().actors.len() + 3,
            counts["X"]
        );
    }
}
