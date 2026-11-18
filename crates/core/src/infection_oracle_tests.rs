// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn actual_infection_methods_match_java_state_aliases_statistics_and_randomness() {
    compare_reference(
        "infection",
        include_str!("../tests/fixtures/infection.tsv"),
        Coverage {
            cases: 46,
            snapshots: 108,
            calls: [
                ("infectedByCode", 11),
                ("infectedBy", 19),
                ("infectedByFruit", 16),
                ("infectedByFruitWeak", 16),
            ]
            .map(|(operation, count)| (operation.to_owned(), count))
            .into_iter()
            .collect(),
            compared_rows: 124_956,
            collaborator_rows: 648,
            selection_metadata_rows: 46,
            input_genotype_rows: 26_680,
            input_and_command_rows: 7_607,
        },
    );
}
