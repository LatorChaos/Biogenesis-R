// SPDX-License-Identifier: GPL-2.0-or-later

use super::super::organism_oracle_tests::reproduction_oracle_tests;
use super::*;
use std::collections::BTreeMap;

const SOURCE: &str = include_str!("../tests/fixtures/player-actions.tsv");

pub(in crate::world) fn invoke_selected(
    world: &mut World,
    reference: &OrganismRef,
    operation: &str,
) -> Result<(), WorldError> {
    let handle = OrganismHandle(Rc::clone(reference), world.handle());
    assert!(world.inspect_handle(&handle).is_some());
    let action = match operation {
        "feed" => OrganismAction::Feed,
        "weaken" => OrganismAction::Weaken,
        "kill" => OrganismAction::Kill,
        "revive" => OrganismAction::Revive,
        "disperse" => OrganismAction::Disperse,
        "reproduce" => OrganismAction::Reproduce,
        "rejuvenate" => OrganismAction::Rejuvenate,
        _ => panic!("Unknown selected callback {operation}"),
    };
    let needs_alive = !matches!(action, OrganismAction::Revive | OrganismAction::Disperse);
    let accepted_guard = reference.borrow().state.alive == needs_alive;
    let accepted = world.act_on_organism(&handle, action)?;
    // This assertion protects the typed action contract; Java's void callback has no B record
    assert_eq!(
        accepted, accepted_guard,
        "Typed callback acceptance: {operation}"
    );
    Ok(())
}

pub(in crate::world) fn pick_role(
    world: &World,
    position: [i32; 2],
    actors: &BTreeMap<String, OrganismRef>,
) -> Option<String> {
    let handle = world.pick_organism(position).unwrap()?;
    assert!(world.inspect_handle(&handle).is_some());
    Some(
        actors
            .iter()
            .find_map(|(role, reference)| {
                let actual = OrganismHandle(Rc::clone(reference), world.handle());
                (actual == handle).then(|| role.clone())
            })
            .expect("Picked handle must identify an actual constructed or discovered role"),
    )
}

#[test]
fn actual_java_selected_callbacks_preserve_state_aliases_statistics_and_rng() {
    reproduction_oracle_tests::compare_fixture(SOURCE, "player-actions");
    reproduction_oracle_tests::compare_raw_null_gene_revive_prefix();
    // Other raw source observations remain unpassed compatibility diagnostics
    assert_eq!(
        include_str!("../tests/fixtures/player-actions-diagnostic.tsv")
            .lines()
            .count(),
        8_658
    );
}

#[test]
fn action_checker_rejects_changed_source_rng_without_using_it_as_input() {
    reproduction_oracle_tests::reject_player_actions_rng_change(SOURCE);
}

#[test]
fn action_checker_rejects_changed_revived_energy_without_using_it_as_input() {
    reproduction_oracle_tests::reject_player_actions_energy_change(SOURCE);
}
