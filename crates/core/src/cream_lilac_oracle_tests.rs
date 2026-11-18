// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn cream_and_lilac_contacts_match_original_java_fields_energy_and_randomness() {
    compare_reference(
        "cream_lilac",
        include_str!("../tests/fixtures/cream_lilac.tsv"),
        Coverage {
            cases: 265,
            snapshots: 585,
            calls: [("touchEffects1.CREAM", 200), ("touchEffects1.LILAC", 120)]
                .map(|(operation, count)| (operation.to_owned(), count))
                .into_iter()
                .collect(),
            compared_rows: 537_085,
            collaborator_rows: 3_510,
            selection_metadata_rows: 265,
            input_genotype_rows: 90_420,
            input_and_command_rows: 57_320,
        },
    );
}
