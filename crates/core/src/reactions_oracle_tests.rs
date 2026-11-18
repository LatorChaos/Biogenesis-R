// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn reactions_and_brakes_match_original_java_fields_payments_and_randomness() {
    compare_reference(
        "reactions",
        include_str!("../tests/fixtures/reactions.tsv"),
        Coverage {
            cases: 186,
            snapshots: 654,
            calls: [
                ("touchReaction", 267),
                ("touchBrake", 177),
                ("touchfriendReaction", 12),
                ("touchfriendBrake", 12),
            ]
            .map(|(operation, count)| (operation.to_owned(), count))
            .into_iter()
            .collect(),
            compared_rows: 679_638,
            collaborator_rows: 3_924,
            selection_metadata_rows: 186,
            input_genotype_rows: 86_040,
            input_and_command_rows: 59_205,
        },
    );
}
