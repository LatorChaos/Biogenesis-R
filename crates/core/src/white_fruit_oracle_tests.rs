// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn white_and_fruit_match_original_java_infection_shields_donation_and_identity() {
    compare_reference(
        "white_fruit",
        include_str!("../tests/fixtures/white_fruit.tsv"),
        Coverage {
            cases: 409,
            snapshots: 821,
            calls: [("touchEffects2.WHITE", 254), ("touchEffects2.FRUIT", 158)]
                .map(|(operation, count)| (operation.to_owned(), count))
                .into_iter()
                .collect(),
            compared_rows: 852_177,
            collaborator_rows: 4_926,
            selection_metadata_rows: 409,
            input_genotype_rows: 188_540,
            input_and_command_rows: 99_820,
        },
    );
}
