// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn contact_state_and_restoration_match_java_fields_aliases_statistics_and_randomness() {
    compare_reference(
        "contact_state",
        include_str!("../tests/fixtures/contact_state.tsv"),
        Coverage {
            cases: 94,
            snapshots: 218,
            calls: [
                ("survive", 4),
                ("inhibited", 10),
                ("frozen", 32),
                ("paralyzed", 7),
                ("neutralized", 23),
                ("segmentsRestoreEffects", 48),
            ]
            .map(|(operation, count)| (operation.to_owned(), count))
            .into_iter()
            .collect(),
            compared_rows: 317_626,
            collaborator_rows: 1_308,
            selection_metadata_rows: 94,
            input_genotype_rows: 89_440,
            input_and_command_rows: 18_399,
        },
    );
}
