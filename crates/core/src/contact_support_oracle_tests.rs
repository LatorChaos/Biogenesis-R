// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn contact_support_matches_original_java_state_energy_aliases_and_randomness() {
    compare_reference(
        "contact_support",
        include_str!("../tests/fixtures/contact_support.tsv"),
        Coverage {
            cases: 193,
            snapshots: 523,
            calls: [
                ("virusneutralized", 5),
                ("lavenderinhibition", 21),
                ("lavendershield", 21),
                ("weaklavendershield", 21),
                ("pretouchEffects", 229),
                ("touchfriendEffects", 33),
            ]
            .map(|(operation, count)| (operation.to_owned(), count))
            .into_iter()
            .collect(),
            compared_rows: 535_591,
            collaborator_rows: 3_138,
            selection_metadata_rows: 193,
            input_genotype_rows: 85_700,
            input_and_command_rows: 38_859,
        },
    );
}
