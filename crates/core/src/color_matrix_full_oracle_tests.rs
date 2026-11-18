// SPDX-License-Identifier: GPL-2.0-or-later

//! Required external-shard gate; source declarations are generated only after accepted corpus freeze

use super::*;
use std::{fs::File, io::Read, path::PathBuf};

const MAX_SHARD_BYTES: u64 = 25_000_000;

struct RowDeclaration {
    expanded_bytes: u64,
    ledger: Ledger,
    tags: &'static [(&'static str, usize)],
}

// The preparation deliberately has no declaration file before independent source-data acceptance
include!("color_matrix_full_ledgers.rs");

#[test]
#[ignore = "Required full color matrix gate; run the explicit frozen-shard checker"]
fn full_color_matrix_external_shard_matches_original_java() {
    let actor = std::env::var("BGR_COLOR_MATRIX_ACTOR").expect("Pass the verified actor row");
    assert!(MATRIX_COLORS.split_ascii_whitespace().any(|color| color == actor));
    let declaration = expected_full_row(&actor);
    assert!(declaration.expanded_bytes > 0 && declaration.expanded_bytes <= MAX_SHARD_BYTES);
    let path = PathBuf::from(std::env::var_os("BGR_COLOR_MATRIX_SHARD").expect("Pass the verified expanded shard"));
    assert!(path.is_absolute());
    let file = File::open(&path).unwrap();
    assert_eq!(file.metadata().unwrap().len(), declaration.expanded_bytes);
    let mut source = String::new();
    file.take(MAX_SHARD_BYTES + 1).read_to_string(&mut source).unwrap();
    assert_eq!(source.len() as u64, declaration.expanded_bytes);

    let mut tags = BTreeMap::new();
    let mut pairs = Vec::new();
    let mut owner_projections = 0;
    for line in source.lines() {
        let row: Vec<_> = line.split('\t').collect();
        *tags.entry(row[0].to_owned()).or_insert(0) += 1;
        if row[0] == "C" {
            assert_eq!(row.len(), 4);
            pairs.push(row[1].to_owned());
        }
        if row[0] == "V" && row.get(2) == Some(&"worldStatistics") {
            owner_projections += 1;
        }
    }
    let expected_tags: BTreeMap<_, _> = declaration.tags.iter().map(|(tag, count)| ((*tag).to_owned(), *count)).collect();
    assert_eq!(tags, expected_tags, "{actor} complete source-tag ledger");
    let expected_pairs: Vec<_> = MATRIX_COLORS.split_ascii_whitespace().map(|target| format!("pair-{actor}-{target}")).collect();
    assert_eq!(pairs, expected_pairs);
    assert_eq!(owner_projections, 368);
    let actual = compare_declared_fixture(&source, FULL_MATRIX_MODE, declaration.ledger);
    assert_eq!((actual.cases, actual.snapshots), (46, 368));
    assert_eq!(actual.calls.values().sum::<usize>(), 322);
    println!(
        "FULL_MATRIX_ROW_MATCH\tactor={actor}\trows={}\tcases={}\tsnapshots={}\toperations={}\teligible={}\tinput_GN={}\tcollaborators={}\tmetadata={}\tactor_views={}\tdiscoveries={}\tinsertions={}\towner_projections={owner_projections}",
        actual.rows,
        actual.cases,
        actual.snapshots,
        actual.calls.values().sum::<usize>(),
        actual.observations,
        actual.supplied_genotypes,
        actual.organism_collaborators + actual.world_collaborators,
        actual.selection_metadata + actual.scope_metadata + actual.inputs_and_commands,
        tags["O"] / 154,
        tags.get("D").copied().unwrap_or(0),
        tags.get("J").copied().unwrap_or(0),
    );
}
