// SPDX-License-Identifier: GPL-2.0-or-later

use super::death_oracle_tests::{Coverage, compare_reference};

#[test]
fn active_frame_methods_and_contact_hatching_match_original_java_observations() {
    compare_reference(
        "frame_effects",
        include_str!("../tests/fixtures/frame_effects.tsv"),
        Coverage {
            cases: 222,
            snapshots: 537,
            calls: [
                ("segmentsFrameEffects", 8),
                ("leafExtraEffects", 26),
                ("jadeExtraEffects", 24),
                ("segmentsHealingEffects", 14),
                ("segmentsUpdateEffects", 7),
                ("plantsUpdateEffects", 23),
                ("segmentsExtraEffects", 3),
                ("segmentsExtraPlantsEffects", 9),
                ("darkfireRestoreEffects", 7),
                ("touchHatch", 194),
            ]
            .map(|(operation, count)| (operation.to_owned(), count))
            .into_iter()
            .collect(),
            compared_rows: 599_709,
            collaborator_rows: 3_222,
            selection_metadata_rows: 222,
            input_genotype_rows: 129_120,
            input_and_command_rows: 42_100,
        },
    );
}
