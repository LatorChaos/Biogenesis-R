// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn five_draining_contacts_match_original_java_fields_energy_and_randomness() {
    compare_reference(
        "draining",
        include_str!("../tests/fixtures/draining.tsv"),
        Coverage {
            cases: 449,
            snapshots: 901,
            calls: [
                ("touchEffects1.ORANGE", 136),
                ("touchEffects1.FIRE", 108),
                ("touchEffects1.RED", 50),
                ("touchEffects1.PINK", 81),
                ("touchEffects1.MAROON", 77),
            ]
            .map(|(operation, count)| (operation.to_owned(), count))
            .into_iter()
            .collect(),
            compared_rows: 934_417,
            collaborator_rows: 5_406,
            selection_metadata_rows: 449,
            input_genotype_rows: 206_580,
            input_and_command_rows: 99_704,
        },
    );
}
