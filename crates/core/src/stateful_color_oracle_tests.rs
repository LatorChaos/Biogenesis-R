// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay source-driven color sequences with complete state and retained object identities

use super::*;
use std::sync::OnceLock;

const SOURCE: &str = include_str!("../tests/fixtures/stateful_color.tsv");
const INPUTS: &str = include_str!("../tests/fixtures/stateful_color_inputs.tsv");

#[derive(Default)]
struct Specification {
    family: String,
    suite: String,
    seed: i64,
    controls: Vec<(String, String, String, String)>,
    calls: Vec<PlannedCall>,
}

struct PlannedCall {
    operation: String,
    target: String,
    argument: String,
    witnesses: Vec<[String; 5]>,
}

fn specifications() -> &'static BTreeMap<String, Specification> {
    static SPECS: OnceLock<BTreeMap<String, Specification>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let mut result = BTreeMap::new();
        let mut current: Option<(String, Specification)> = None;
        for line in INPUTS.lines() {
            let row: Vec<_> = line.split('\t').collect();
            match row[0] {
                "C" => {
                    assert_eq!(row.len(), 5);
                    assert!(current.is_none());
                    current = Some((row[1].to_owned(), Specification {
                        family: row[2].to_owned(),
                        suite: row[3].to_owned(),
                        seed: row[4].parse().unwrap(),
                        ..Specification::default()
                    }));
                }
                "I" => {
                    assert_eq!(row.len(), 5);
                    let spec = &mut current.as_mut().unwrap().1;
                    assert!(row[1] == "setup" || row[1] == format!("step-{}", spec.calls.len()));
                    spec.controls.push((row[1].to_owned(), row[2].to_owned(), row[3].to_owned(), row[4].to_owned()));
                }
                "K" => {
                    assert_eq!(row.len(), 5);
                    let spec = &mut current.as_mut().unwrap().1;
                    assert_eq!(row[1], format!("step-{}", spec.calls.len()));
                    spec.calls.push(PlannedCall {
                        operation: row[2].to_owned(),
                        target: row[3].to_owned(),
                        argument: row[4].to_owned(),
                        witnesses: Vec::new(),
                    });
                }
                "F" => {
                    assert_eq!(row.len(), 7);
                    let spec = &mut current.as_mut().unwrap().1;
                    assert_eq!(row[1], format!("step-{}", spec.calls.len() - 1));
                    let call = spec.calls.last_mut().unwrap();
                    assert!(call.witnesses.iter().all(|value| value[0] != row[2]));
                    call.witnesses.push(std::array::from_fn(|index| row[index + 2].to_owned()));
                }
                "M" | "G" | "U" => {
                    assert!(current.is_some());
                    assert_eq!(row.len(), if row[0] == "G" { 8 } else { 4 });
                    // Actual input G/N and construction commands remain the established constructor authority
                }
                "E" => {
                    assert_eq!(row.len(), 1);
                    let (name, spec) = current.take().unwrap();
                    assert!(result.insert(name, spec).is_none());
                }
                tag => panic!("Unexpected frozen input-plan row {tag}"),
            }
        }
        assert!(current.is_none());
        assert_eq!(result.len(), 20);
        assert_eq!(result.values().map(|spec| spec.calls.len()).sum::<usize>(), 383);
        assert_eq!(result.values().flat_map(|spec| &spec.calls).map(|call| call.witnesses.len()).sum::<usize>(), 66);
        result
    })
}

pub(super) fn declared_ledger() -> Ledger {
    Ledger {
        rows: 468_473,
        cases: 20,
        snapshots: 403,
        observations: 449_338,
        supplied_genotypes: 8_520,
        organism_collaborators: 2_704,
        world_collaborators: 1_209,
        selection_metadata: 20,
        scope_metadata: 2_138,
        inputs_and_commands: 4_544,
        calls: [
            ("symmetric", 36),
            ("bounds", 71),
            ("build_tree", 23),
            ("checkHit", 23),
            ("time", 218),
            ("reproduce", 4),
            ("touchEffects1.DEFAULT", 4),
            ("touchEffects2.DEFAULT", 4),
        ].into_iter().map(|(name, count)| (name.to_owned(), count)).collect(),
        ..Ledger::default()
    }
}

pub(super) fn validate_family(name: &str, row: &[&str]) {
    assert_eq!(row.len(), 3);
    let spec = &specifications()[name];
    assert_eq!(row[1], spec.family);
    assert_eq!(row[2], spec.suite);
}

pub(super) fn validate_call(name: &str, index: usize, row: &[&str]) {
    let expected = &specifications()[name].calls[index];
    assert_eq!((row[2], row[3], row[4]), (expected.operation.as_str(), expected.target.as_str(), expected.argument.as_str()));
    assert!([
        "symmetric", "bounds", "build_tree", "checkHit", "time", "reproduce",
        "touchEffects1.DEFAULT", "touchEffects2.DEFAULT",
    ].contains(&row[2]));
    if ["time", "build_tree"].contains(&row[2]) {
        assert_eq!(row[3], "world");
    }
}

pub(super) fn validate_default(name: &str, operation: &str, argument: &str) {
    assert!(specifications()[name].family.starts_with("RAW-"));
    assert!(["touchEffects1.DEFAULT", "touchEffects2.DEFAULT"].contains(&operation));
    assert_eq!(argument, "killer,0,0");
}

pub(super) fn stored_in_source_actor_order(
    world: &World,
    actors: &BTreeMap<String, OrganismRef>,
    actor_order: &[String],
) -> Vec<(String, IntRect)> {
    let stored = stored_tree(world, actors);
    let count = stored.len();
    let mut entries: BTreeMap<_, _> = stored.into_iter().collect();
    assert_eq!(entries.len(), count);
    let mut ordered = Vec::new();
    for role in actor_order {
        if let Some(bounds) = entries.remove(role) {
            ordered.push((role.clone(), bounds));
        }
    }
    assert!(entries.is_empty());
    assert_eq!(ordered.len(), count);
    ordered
}

pub(super) fn validate_witness(name: &str, row: &[&str]) {
    assert_eq!(row.len(), 8);
    assert_eq!(row[7], "true");
    let index: usize = row[1].strip_prefix("step-").unwrap().parse().unwrap();
    let call = &specifications()[name].calls[index];
    let witness = call.witnesses.iter().find(|value| value[0] == row[2]).unwrap();
    let expected: Vec<_> = witness[..4].iter().map(String::as_str).collect();
    assert_eq!(&row[2..6], expected.as_slice());
    if ["eq", "elementEq"].contains(&row[3]) {
        assert_eq!(row[6], witness[4]);
    }
}

pub(super) fn validate_case(case: &CallerCase, witnesses: &[(String, String)]) {
    let spec = &specifications()[&case.name];
    assert_eq!(case.seed, spec.seed);
    let expected_witnesses: Vec<_> = spec.calls.iter().enumerate().flat_map(|(index, call)| {
        call.witnesses.iter().map(move |value| (format!("step-{index}"), value[0].clone()))
    }).collect();
    assert_eq!(witnesses, expected_witnesses.as_slice());
    let expected_stages: BTreeSet<_> = std::iter::once("initial".to_owned())
        .chain((0..spec.calls.len()).map(|index| format!("step-{index}"))).collect();
    assert_eq!(case.snapshots.keys().cloned().collect::<BTreeSet<_>>(), expected_stages);
    let mut expected_inputs = Vec::new();
    for index in 0..5 {
        let construction = if index == 4 { "clone:g0" } else { "explicit" };
        expected_inputs.push(("setup".to_owned(), format!("genome:g{index}:construction"), "String".to_owned(), construction.to_owned()));
    }
    for gas in ["_CO2", "_O2", "_CH4"] {
        expected_inputs.push(("setup".to_owned(), format!("world:{gas}"), "double".to_owned(),
            case.settings[&format!("INITIAL{gas}")].clone()));
    }
    expected_inputs.push(("setup".to_owned(), "visible:selected".to_owned(), "String".to_owned(), "selector".to_owned()));
    expected_inputs.extend(spec.controls.iter().cloned());
    let actual_inputs: Vec<_> = case.steps.iter().filter_map(|step| match step {
        Step::Input { stage, target, kind, value } => Some((stage.clone(), target.clone(), kind.clone(), value.clone())),
        _ => None,
    }).collect();
    assert_eq!(actual_inputs, expected_inputs);
    let supplied_actors: Vec<_> = case.steps.iter().filter_map(|step| match step {
        Step::Actor { role, genotype, center } => Some((role.as_str(), genotype.as_str(), *center)),
        _ => None,
    }).collect();
    assert_eq!(supplied_actors, [("selector", "g2", [1000, 1000]), ("victim", "g0", [3500, 3500]), ("killer", "g1", [5000, 5000])]);
}

#[test]
fn actual_java_stateful_colors_preserve_complete_state_identity_tree_and_rng() {
    compare_fixture(SOURCE, STATEFUL_MODE);
}

#[test]
fn stateful_checker_rejects_changed_post_frame_rng_after_real_execution() {
    let (mut cases, ledger) = parse(SOURCE, STATEFUL_MODE);
    assert_eq!(ledger, declared_ledger());
    let case = cases.iter_mut().find(|case| case.name == "violet-next-turn-magenta-repair").unwrap();
    case.snapshots.get_mut("step-7").unwrap().suffix.as_mut().unwrap().0 ^= 1;
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        replay(case, &mut Vec::new())
    })).unwrap_err();
    let text = failure.downcast_ref::<String>().map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied()).unwrap();
    assert!(text.contains("step-7 RNG suffix"), "{text}");
}

#[test]
fn stateful_checker_rejects_changed_poison_color_without_using_it_as_input() {
    let (mut cases, ledger) = parse(SOURCE, STATEFUL_MODE);
    assert_eq!(ledger, declared_ledger());
    let case = cases.iter_mut().find(|case| case.name == "violet-next-turn-magenta-repair").unwrap();
    let field = case.snapshots.get_mut("step-5").unwrap().organisms.get_mut("victim").unwrap().get_mut("_segColor").unwrap();
    let mut colors: Vec<String> = field.split(',').map(str::to_owned).collect();
    colors[1] = format!("{:08x}", u32::from_str_radix(&colors[1], 16).unwrap() ^ 1);
    *field = colors.join(",");
    let mut errors = Vec::new();
    replay(case, &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("step-5:victim") && errors[0].contains("_segColor"), "{errors:?}");
}
