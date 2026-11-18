// SPDX-License-Identifier: GPL-2.0-or-later

//! Observe actual decoded bytes, then encode the resulting genome without expected-state construction

use super::*;
use std::{collections::BTreeSet, fmt::Write as _, fs, path::PathBuf, rc::Rc};

#[test]
fn xml_roundtrip_actual_bytes() {
    let root = PathBuf::from(std::env::var_os("BGR_XML_CHAIN_ROOT").unwrap());
    let manifest = PathBuf::from(std::env::var_os("BGR_XML_CHAIN_MANIFEST").unwrap());
    let output = PathBuf::from(std::env::var_os("BGR_XML_CHAIN_OUTPUT").unwrap());
    let expected_count: usize = std::env::var("BGR_XML_CHAIN_COUNT").unwrap().parse().unwrap();
    assert!((1..=235).contains(&expected_count));
    fs::create_dir(&output).unwrap();
    let mut rows = String::new();
    let mut identities = BTreeSet::new();
    for line in fs::read_to_string(manifest).unwrap().lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 2);
        let id = row[0].strip_prefix("rust-").unwrap();
        assert_eq!(id.len(), 4);
        assert!(id.bytes().all(|byte| byte.is_ascii_digit()));
        assert!(identities.insert(row[0].to_owned()));
        let relative = std::path::Path::new(row[1]);
        assert!(relative.components().all(|part| matches!(part, std::path::Component::Normal(_))));
        let bytes = fs::read(root.join(relative)).unwrap();
        assert!(bytes.len() <= 16 * 1024 * 1024);
        let genome = crate::parse_genome_xml(&bytes).unwrap();
        writeln!(rows, "C\t{}\nO\tsuccess", row[0]).unwrap();
        let mut fields = genome_fields(&genome);
        assert!(fields.insert("_nGenes".into(), genome.genes().len().to_string()).is_none());
        assert_eq!(fields.len(), 27);
        for (name, value) in fields {
            writeln!(rows, "G\t{name}\t{value}").unwrap();
        }
        let mut roots = Vec::new();
        for index in 0..genome.genes().len() {
            let fields = gene_fields(&genome.gene(index));
            assert_eq!(fields.len(), 40);
            for (name, value) in fields {
                writeln!(rows, "N\t{index}\t{name}\t{value}").unwrap();
            }
            roots.push(genome.genes().iter().position(|gene| Rc::ptr_eq(gene, &genome.genes()[index])).unwrap().to_string());
        }
        writeln!(rows, "A\t{}\nE", roots.join(",")).unwrap();
        let encoded = crate::encode_genome_xml(&genome).unwrap();
        fs::write(output.join(format!("{id}.bgg")), encoded).unwrap();
    }
    assert_eq!(identities.len(), expected_count);
    fs::write(output.join("decoded.tsv"), rows).unwrap();
    println!("XML_ROUNDTRIP_RUST PASS parsed={} encoded={}", identities.len(), identities.len());
}
