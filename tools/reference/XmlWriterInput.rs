// SPDX-License-Identifier: GPL-2.0-or-later

//! Construct only supplied source-writer recipes, then call the real Rust XML writer

use biogenesis_core::encode_genome_xml;
use biogenesis_core::{Gene, GeneRef, Genome, GenomeModifiers, GenomeProperties, REACTION_NAMES};
use std::{collections::BTreeMap, fs, path::Path, rc::Rc};

type Fields = BTreeMap<String, String>;

fn i(fields: &Fields, name: &str) -> i32 {
    fields[name].parse().unwrap()
}
fn b(fields: &Fields, name: &str) -> bool {
    fields[name].parse().unwrap()
}
fn d(text: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(text, 16).unwrap())
}
fn hex_bytes(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    (0..text.len())
        .step_by(2)
        .map(|p| u8::from_str_radix(&text[p..p + 2], 16).unwrap())
        .collect()
}
fn make(g: &Fields, n: &BTreeMap<usize, Fields>, aliases: &[usize]) -> Genome {
    assert_eq!(g.len(), 28);
    let count = i(g, "_nGenes") as usize;
    assert_eq!(count, n.len());
    assert_eq!(count, aliases.len());
    let mut genes: Vec<GeneRef> = Vec::new();
    for index in 0..count {
        let fields = &n[&index];
        assert_eq!(fields.len(), 40);
        assert!(aliases[index] <= index);
        let gene = if aliases[index] < index {
            assert_eq!(fields, &n[&aliases[index]]);
            Rc::clone(&genes[aliases[index]])
        } else {
            let color = if fields["_color"] == "~" {
                None
            } else {
                Some(u32::from_str_radix(&fields["_color"], 16).unwrap())
            };
            Gene::new(
                d(&fields["_length"]),
                d(&fields["_theta"]),
                color,
                i(fields, "_branch"),
                REACTION_NAMES.map(|name| i(fields, &format!("_{name}reaction"))),
            )
            .shared()
        };
        genes.push(gene);
    }
    let modifiers = GenomeModifiers {
        cream: i(g, "_modifiescream"),
        fallow: i(g, "_modifiesfallow"),
        spore: i(g, "_modifiesspore"),
        adapt_spore: i(g, "_adaptspore"),
        plague: b(g, "_plague"),
        disperse_children: b(g, "_disperseChildren"),
        generation_battle: b(g, "_generationBattle"),
        sibling_battle: b(g, "_siblingBattle"),
        altruist: b(g, "_altruist"),
        familial: b(g, "_familial"),
        social: b(g, "_social"),
        peaceful: b(g, "_peaceful"),
        passive: b(g, "_passive"),
        clockwise: b(g, "_clockwise"),
        mimic_all: b(g, "_mimicall"),
        modifies_pink: b(g, "_modifiespink"),
        modifies_lilac: b(g, "_modifieslilac"),
        modifies_sky: b(g, "_modifiessky"),
        selfish: b(g, "_selfish"),
    };
    let mut genome = Genome::from_parts(
        &genes,
        GenomeProperties {
            symmetry: i(g, "_symmetry"),
            mirror: i(g, "_mirror"),
            mutation_rate: i(g, "_mutationrate"),
            activity: i(g, "_activity"),
            modifiers,
        },
    );
    genome.set_generation(i(g, "_generation"));
    genome.set_clade_id(if g["_cladeID"] == "~" {
        None
    } else {
        Some(String::from_utf8(hex_bytes(&g["_cladeID"])).unwrap())
    });
    genome.set_update_clade(b(g, "_updateClade"));
    for index in 0..count {
        let first = genes
            .iter()
            .position(|gene| Rc::ptr_eq(gene, &genome.genes()[index]))
            .unwrap();
        assert_eq!(first, aliases[index]);
    }
    genome
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        3,
        "Usage: XmlWriterInput INPUT_TSV NEW_OUTPUT_DIRECTORY"
    );
    let input = fs::read_to_string(&args[1]).unwrap();
    let output = Path::new(&args[2]);
    fs::create_dir(output).unwrap();
    let mut g = Fields::new();
    let mut n: BTreeMap<usize, Fields> = BTreeMap::new();
    let mut aliases = Vec::new();
    let mut id = String::new();
    let mut count = 0;
    let mut outcomes = String::new();
    for line in input.lines() {
        let row: Vec<_> = line.split('\t').collect();
        match row[0] {
            "C" => {
                assert!(id.is_empty());
                assert_eq!(row.len(), 2);
                id = row[1].into();
                assert_eq!(id.len(), 4);
                assert!(id.bytes().all(|v| v.is_ascii_digit()));
            }
            "G" => {
                assert_eq!(row.len(), 3);
                assert!(g.insert(row[1].into(), row[2].into()).is_none());
            }
            "N" => {
                assert_eq!(row.len(), 4);
                assert!(
                    n.entry(row[1].parse().unwrap())
                        .or_default()
                        .insert(row[2].into(), row[3].into())
                        .is_none()
                );
            }
            "A" => {
                assert_eq!(row.len(), 2);
                aliases = if row[1].is_empty() {
                    Vec::new()
                } else {
                    row[1].split(',').map(|s| s.parse().unwrap()).collect()
                };
            }
            "E" => {
                assert_eq!(row.len(), 1);
                assert!(!id.is_empty());
                let genome = make(&g, &n, &aliases);
                match encode_genome_xml(&genome) {
                    Ok(bytes) => {
                        fs::write(output.join(format!("{id}.bgg")), bytes).unwrap();
                        outcomes.push_str(&format!("O\t{id}\tsuccess\n"));
                    }
                    Err(error) => outcomes.push_str(&format!("O\t{id}\terror\t{error:?}\n")),
                }
                count += 1;
                id.clear();
                g.clear();
                n.clear();
                aliases.clear();
            }
            tag => panic!("Unexpected input tag {tag}"),
        }
    }
    assert!(id.is_empty());
    assert_eq!(count, 260);
    fs::write(output.join("outcomes.tsv"), outcomes).unwrap();
    println!("XML_WRITER_INPUT PASS inputs={count}");
}
