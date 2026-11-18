// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn violet_sky_olive_and_gray_match_original_java_damage_shields_and_state() {
    compare_reference(
        "disabling",
        include_str!("../tests/fixtures/disabling.tsv"),
        Coverage {
            cases: 545,
            snapshots: 1_098,
            calls: [
                ("touchEffects2.VIOLET", 194),
                ("touchEffects2.SKY", 145),
                ("touchEffects2.OLIVE", 75),
                ("touchEffects2.GRAY", 139),
            ]
            .map(|(operation, count)| (operation.to_owned(), count))
            .into_iter()
            .collect(),
            compared_rows: 1_150_146,
            collaborator_rows: 6_588,
            selection_metadata_rows: 545,
            input_genotype_rows: 256_460,
            input_and_command_rows: 140_176,
        },
    );
}
