// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn spikepoint_ochre_and_silver_match_original_java_state_payments_and_rng() {
    compare_reference(
        "special_contacts",
        include_str!("../tests/fixtures/special_contacts.tsv"),
        Coverage {
            cases: 355,
            snapshots: 711,
            calls: [
                ("touchEffects1.SPIKEPOINT", 143),
                ("touchEffects1.OCHRE", 66),
                ("touchEffects1.SILVER", 147),
            ]
            .map(|(operation, count)| (operation.to_owned(), count))
            .into_iter()
            .collect(),
            compared_rows: 737_307,
            collaborator_rows: 4_266,
            selection_metadata_rows: 355,
            input_genotype_rows: 163_300,
            input_and_command_rows: 85_358,
        },
    );
}
