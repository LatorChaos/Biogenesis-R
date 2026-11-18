// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay original parse calls using immutable Java input and output assets
//! Rust writer roundtrips require a separate gate using their own produced bytes
//! Copied-source RNG rows do not observe a Rust generator

use super::death_oracle_tests::validate_genome;
use super::*;
use crate::xml::{XmlError, parse_genome_xml};
use std::{fs, path::PathBuf, rc::Rc};

const SOURCE_ROWS: usize = 69_870;
const SOURCE_COUNTS: [(&str, usize); 13] = [
    ("C", 261),
    ("U", 38_367),
    ("R", 848),
    ("F", 448),
    ("K", 587),
    ("O", 587),
    ("G", 7_280),
    ("N", 20_640),
    ("A", 260),
    ("T", 184),
    ("E", 261),
    ("X", 144),
    ("V", 3),
];

#[derive(Default)]
struct XmlCase {
    name: String,
    writer: bool,
    settings: Fields,
    assets: BTreeMap<String, (String, usize)>,
    commands: Vec<(String, String, String)>,
    outcomes: Fields,
    stages: BTreeMap<String, Stage>,
    aliases: BTreeMap<String, Vec<usize>>,
    external: BTreeSet<String>,
    random: BTreeMap<String, (i64, i32, u64)>,
}

fn external_case(name: &str) -> Option<&'static str> {
    match name {
        "owned-external-dtd-default" => Some("owned.dtd"),
        "owned-external-parameter-entity" => Some("parameter.dtd"),
        "owned-external-general-entity" => Some("owned-marker.txt"),
        _ => None,
    }
}

fn insert(fields: &mut Fields, key: &str, value: &str) {
    assert!(
        fields.insert(key.into(), value.into()).is_none(),
        "Duplicate XML field {key}"
    );
}

fn source_ledger(row: &[&str]) -> &'static str {
    match row[0] {
        "G" | "N" | "A" if row[1] == "input" => "writer_input_model",
        "G" | "N" | "A" => "parsed_model",
        "F" if matches!(row[2], "supplied" | "owned") => "supplied_asset",
        "F" => "written_asset",
        "O" => "method_outcome",
        "R" => "copied_source_rng",
        "X" | "T" | "V" => "diagnostic",
        _ => "settings_commands",
    }
}

fn fixture_asset(relative: &str, size: usize) -> Vec<u8> {
    let parts: Vec<_> = relative.split('/').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "cases");
    assert_eq!(parts[1].len(), 4);
    assert!(parts[1].bytes().all(|byte| byte.is_ascii_digit()));
    assert!(
        parts[2].bytes().all(|byte| byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || b"._-".contains(&byte))
    );
    assert!(!parts[2].starts_with('.'));
    let mut path = crate::oracle_tests::fixture_path("crates/core").join("tests/fixtures/xml");
    assert!(
        !fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        let kind = fs::symlink_metadata(&path).unwrap().file_type();
        assert!(!kind.is_symlink());
        assert!(if index == parts.len() - 1 {
            kind.is_file()
        } else {
            kind.is_dir()
        });
    }
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.len(), size, "XML asset size: {}", path.display());
    bytes
}

fn parse_cases() -> Vec<XmlCase> {
    let (settings_schema, _) = primitive_reference_schemas();
    let mut cases = Vec::new();
    let mut active: Option<XmlCase> = None;
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut ledger = BTreeMap::<&str, usize>::new();
    let mut assets = BTreeSet::new();
    let shapes = BTreeMap::from([
        ("C", 4),
        ("U", 4),
        ("F", 7),
        ("K", 4),
        ("O", 3),
        ("X", 4),
        ("G", 5),
        ("N", 6),
        ("A", 4),
        ("T", 3),
        ("R", 5),
        ("V", 4),
        ("E", 1),
    ]);
    for line in include_str!("../tests/fixtures/xml.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), shapes[row[0]], "XML row shape: {line}");
        *counts.entry(row[0]).or_default() += 1;
        *ledger.entry(source_ledger(&row)).or_default() += 1;
        if row[0] == "C" {
            assert!(active.is_none());
            assert_eq!(row[3], "827364");
            assert!(matches!(row[2], "parse" | "write"));
            active = Some(XmlCase {
                name: row[1].into(),
                writer: row[2] == "write",
                ..Default::default()
            });
        }
        let case = active.as_mut().expect("XML row outside a case");
        match row[0] {
            "C" => {}
            "U" => {
                assert_eq!(settings_schema[row[1]], row[2]);
                super::death_oracle_tests::validate(row[2], row[3]);
                insert(&mut case.settings, row[1], row[3]);
            }
            "F" => {
                assert!(matches!(
                    row[2],
                    "supplied" | "owned" | "written" | "partial"
                ));
                assert!(assets.insert(row[3].to_owned()));
                assert_eq!(row[4].len(), 64);
                assert!(row[4].bytes().all(|byte| byte.is_ascii_hexdigit()));
                let bytes = row[5].parse::<usize>().unwrap();
                assert!(bytes <= 104_762);
                assert!(
                    case.assets
                        .insert(row[3].into(), (row[2].into(), bytes))
                        .is_none()
                );
            }
            "K" => {
                assert_eq!(row[1], format!("step-{}", case.commands.len()));
                assert!(matches!(row[2], "parse" | "write"));
                case.commands
                    .push((row[1].into(), row[2].into(), row[3].into()));
            }
            "O" => {
                assert!(matches!(row[2], "success" | "failure"));
                insert(&mut case.outcomes, row[1], row[2]);
            }
            "G" => {
                assert_eq!(row[2], "genotype");
                insert(
                    &mut case.stages.entry(row[1].into()).or_default().genome,
                    row[3],
                    row[4],
                );
            }
            "N" => {
                assert_eq!(row[2], "genotype");
                let index = row[3].parse().unwrap();
                insert(
                    case.stages
                        .entry(row[1].into())
                        .or_default()
                        .genes
                        .entry(index)
                        .or_default(),
                    row[4],
                    row[5],
                );
            }
            "A" => {
                assert_eq!(row[2], "gene_alias_roots");
                let aliases = if row[3].is_empty() {
                    Vec::new()
                } else {
                    row[3]
                        .split(',')
                        .map(|value| value.parse::<usize>().unwrap())
                        .collect()
                };
                assert!(case.aliases.insert(row[1].into(), aliases).is_none());
            }
            "R" => {
                let suffix = (
                    row[2].parse().unwrap(),
                    row[3].parse::<i32>().unwrap(),
                    u64::from_str_radix(row[4], 16).unwrap(),
                );
                assert!((0..10000).contains(&suffix.1));
                assert!(case.random.insert(row[1].into(), suffix).is_none());
            }
            "V" => {
                assert_eq!(row[1], "step-0");
                assert_eq!(row[2], "owned_external_resolution");
                assert_eq!(Some(row[3]), external_case(&case.name));
                assert!(case.external.insert(row[1].into()));
            }
            "X" | "T" => {}
            "E" => cases.push(active.take().unwrap()),
            _ => unreachable!(),
        }
    }
    assert!(active.is_none());
    assert_eq!(counts.values().sum::<usize>(), SOURCE_ROWS);
    let expected: BTreeMap<_, _> = SOURCE_COUNTS.into_iter().collect();
    assert_eq!(counts, expected);
    assert_eq!(
        ledger,
        BTreeMap::from([
            ("settings_commands", 39_476),
            ("copied_source_rng", 848),
            ("supplied_asset", 188),
            ("method_outcome", 587),
            ("parsed_model", 17_816),
            ("diagnostic", 331),
            ("written_asset", 260),
            ("writer_input_model", 10_364),
        ])
    );
    assert_eq!(assets.len(), 448);
    assert_eq!(cases.len(), 261);
    assert_eq!(
        cases
            .iter()
            .map(|case| &case.name)
            .collect::<BTreeSet<_>>()
            .len(),
        261
    );
    cases
}

#[test]
fn reader_only_xml_matches_frozen_java_parse_observations() {
    let cases = parse_cases();
    let gene_schema = gene_fields(&Gene::default()).keys().cloned().collect();
    let mut decoded = Vec::<Genome>::new();
    let mut errors = Vec::new();
    let mut calls = 0;
    let mut success_states = 0;
    let mut genome_values = 0;
    let mut gene_values = 0;
    let mut alias_vectors = 0;
    let mut allocation_metadata = 0;
    let mut security_model_rows = 0;
    let mut security_rejections = 0;
    let mut writer_outcomes = 0;
    for case in cases {
        assert_eq!(case.settings.len(), 147);
        assert_eq!(case.outcomes.len(), case.commands.len());
        assert_eq!(case.random.len(), case.commands.len() + 1);
        assert!(case.random.contains_key("initial"));
        assert!(
            case.random
                .values()
                .all(|suffix| suffix == &case.random["initial"])
        );
        assert_eq!(case.stages.contains_key("input"), case.writer);
        assert_eq!(
            case.stages.keys().collect::<BTreeSet<_>>(),
            case.aliases.keys().collect()
        );
        let mut source_identities = BTreeSet::new();
        for (name, stage) in &case.stages {
            validate_genome(stage, &gene_schema);
            assert!(source_identities.insert(&stage.genome["_reference"]));
            let aliases = &case.aliases[name];
            assert_eq!(aliases.len(), stage.genes.len());
            for (index, root) in aliases.iter().enumerate() {
                assert!(*root <= index && aliases[*root] == *root);
                assert_eq!(stage.genes[&index], stage.genes[root]);
            }
            if name == "input" {
                assert!(stage.genome["_cladeID"].starts_with("b64:"));
            }
        }
        for (stage_name, operation, relative) in &case.commands {
            assert!(case.random.contains_key(stage_name));
            let expected_success = case.outcomes[stage_name] == "success";
            if operation == "write" {
                writer_outcomes += 1;
                assert!(!case.stages.contains_key(stage_name));
                continue;
            }
            calls += 1;
            let (role, size) = &case.assets[relative];
            assert!(matches!(role.as_str(), "supplied" | "written"));
            let bytes = fixture_asset(relative, *size);
            let result = parse_genome_xml(&bytes);
            if case.external.contains(stage_name) {
                assert!(expected_success);
                security_rejections += 1;
                let observation = &case.stages[stage_name];
                security_model_rows += observation.genome.len()
                    + observation.genes.values().map(Fields::len).sum::<usize>()
                    + 1;
                if !matches!(result, Err(XmlError::ExternalResource)) {
                    errors.push(format!("{} {stage_name}: external resource did not return ExternalResource: {result:?}", case.name));
                }
                continue;
            }
            assert_eq!(case.stages.contains_key(stage_name), expected_success);
            match result {
                Err(error) => {
                    if expected_success || matches!(error, XmlError::ExternalResource) {
                        errors.push(format!(
                            "{} {stage_name}: unexpected reader error {error:?}",
                            case.name
                        ));
                    }
                }
                Ok(genome) if !expected_success => {
                    errors.push(format!(
                        "{} {stage_name}: source parse failed but Rust accepted {} genes",
                        case.name,
                        genome.genes().unwrap().len()
                    ));
                }
                Ok(genome) => {
                    success_states += 1;
                    let target = &case.stages[stage_name];
                    let mut expected = target.genome.clone();
                    assert!(expected.remove("_reference").is_some());
                    allocation_metadata += 1;
                    let mut actual = genome_fields(&genome);
                    actual.insert("_nGenes".into(), genome.genes().unwrap().len().to_string());
                    compare_fields(
                        &case.name,
                        stage_name,
                        "XML-G",
                        &actual,
                        &expected,
                        &mut errors,
                    );
                    genome_values += expected.len();
                    for (index, fields) in &target.genes {
                        if *index < genome.genes().unwrap().len() {
                            compare_fields(
                                &case.name,
                                stage_name,
                                "XML-N",
                                &gene_fields(&genome.gene_ref(*index).unwrap().borrow()),
                                fields,
                                &mut errors,
                            );
                            gene_values += fields.len();
                        }
                    }
                    let roots: Vec<_> = genome
                        .genes().unwrap()
                        .iter()
                        .enumerate()
                        .map(|(index, gene)| {
                            genome.genes().unwrap()[..=index]
                                .iter()
                                .position(|other| Rc::ptr_eq(gene, other))
                                .unwrap()
                        })
                        .collect();
                    if roots != case.aliases[stage_name] {
                        errors.push(format!("{} {stage_name}: Gene aliases differ", case.name));
                    }
                    alias_vectors += 1;
                    for previous in &decoded {
                        if genome.genes().unwrap().iter().any(|gene| {
                            previous.genes().unwrap().iter().any(|other| Rc::ptr_eq(gene, other))
                        }) {
                            errors.push(format!(
                                "{} {stage_name}: separate XML parses share mutable Genes",
                                case.name
                            ));
                        }
                    }
                    decoded.push(genome);
                }
            }
        }
    }
    assert_eq!(calls, 327);
    assert_eq!(writer_outcomes, 260);
    assert_eq!(security_rejections, 3);
    assert_eq!(security_model_rows, 207);
    assert!(
        errors.is_empty(),
        "XML reader mismatches ({}):\n{}",
        errors.len(),
        errors.join("\n")
    );
    assert_eq!(success_states, 181);
    assert_eq!(genome_values, 4_887);
    assert_eq!(gene_values, 12_360);
    assert_eq!(alias_vectors, 181);
    assert_eq!(allocation_metadata, 181);
    assert_eq!(calls + genome_values + gene_values + alias_vectors, 17_755);
    let source_metadata_and_assets = 10_364 + 448 + 848 + 331 + 39_476;
    assert_eq!(
        calls
            + genome_values
            + gene_values
            + alias_vectors
            + allocation_metadata
            + security_model_rows
            + writer_outcomes
            + source_metadata_and_assets,
        SOURCE_ROWS
    );
    println!(
        "XML_READER PASS calls=327 successful_states=181 obligations=17755 source_rows=69870 writer_calls_executed=0 security_rejections=3 copied_source_rng_rows=848"
    );
}


#[test]
fn iso_8859_9_matches_frozen_source_genomes_errors_and_aliases() {
    #[derive(Default)]
    struct CharsetCase {
        name: String,
        file: String,
        bytes: usize,
        settings: Fields,
        stage: Stage,
        success: Option<bool>,
        aliases: Option<Vec<usize>>,
        random: BTreeMap<String, (i64, i32, u64)>,
        diagnostic: bool,
        carrier: bool,
    }
    let (settings_schema, _) = primitive_reference_schemas();
    let gene_schema = gene_fields(&Gene::default()).keys().cloned().collect();
    let shapes = BTreeMap::from([("C",4),("F",7),("U",4),("R",5),("O",2),
        ("X",3),("G",3),("N",4),("A",2),("D",3),("E",1)]);
    let rows = include_str!("../tests/fixtures/xml_iso_8859_9/observations.tsv");
    let mut counts = BTreeMap::<&str,usize>::new();
    let mut cases = Vec::new();
    let mut active = None;
    for line in rows.lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), shapes[row[0]], "ISO-8859-9 row: {line}");
        *counts.entry(row[0]).or_default() += 1;
        if row[0] == "C" {
            assert!(active.is_none());assert_eq!(row[1].parse::<usize>().unwrap(), cases.len());
            assert_eq!(row[3], "827364");
            assert!(!row[2].is_empty() && row[2].bytes().all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-'));
            active = Some(CharsetCase { name:row[2].into(), ..Default::default() });
        }
        let case = active.as_mut().expect("ISO-8859-9 row outside case");
        match row[0] {
            "C" => {}
            "F" => {
                assert!(case.file.is_empty());assert_eq!(row[1], format!("{}.bgg",case.name));
                assert_eq!(row[2].len(),64);assert!(row[2].bytes().all(|value|value.is_ascii_hexdigit()));
                assert_eq!(row[6],"ISO-8859-9");case.file=row[1].into();case.bytes=row[3].parse().unwrap();
                assert!(case.bytes<=32768);
            }
            "U" => {
                assert_eq!(settings_schema[row[1]],row[2]);
                super::death_oracle_tests::validate(row[2],row[3]);insert(&mut case.settings,row[1],row[3]);
            }
            "R" => {
                let value=(row[2].parse().unwrap(),row[3].parse::<i32>().unwrap(),u64::from_str_radix(row[4],16).unwrap());
                assert!((0..10000).contains(&value.1));assert!(case.random.insert(row[1].into(),value).is_none());
            }
            "O" => {
                assert!(matches!(row[1],"success"|"error"));assert!(case.success.replace(row[1]=="success").is_none());
            }
            "X" => { assert!(!case.diagnostic);assert_eq!(row[1],"org.xml.sax.SAXException");case.diagnostic=true; }
            "G" => insert(&mut case.stage.genome,row[1],row[2]),
            "N" => insert(case.stage.genes.entry(row[1].parse().unwrap()).or_default(),row[2],row[3]),
            "A" => { assert!(case.aliases.replace(row[1].split(',').map(|value|value.parse().unwrap()).collect()).is_none()); }
            "D" => { assert!(!case.carrier);assert_eq!(row[2],"ISO-8859-9");case.carrier=true; }
            "E" => cases.push(active.take().unwrap()),
            _ => unreachable!(),
        }
    }
    assert!(active.is_none());assert_eq!(cases.len(),260);assert_eq!(rows.lines().count(),55634);
    assert_eq!(counts,BTreeMap::from([("C",260),("F",260),("U",38220),("R",520),("O",260),
        ("X",34),("E",260),("G",6328),("N",9040),("A",226),("D",226)]));
    let base=crate::oracle_tests::fixture_path("crates/core").join("tests/fixtures/xml_iso_8859_9/cases");
    let casing_targets:Vec<_>=cases.iter().filter(|case|matches!(case.name.as_str(),"ascii-control"|"non-ascii-positive"))
        .map(|case|(case.stage.genome.clone(),case.stage.genes.clone(),case.aliases.clone().unwrap())).collect();
    assert_eq!(casing_targets.len(),2);
    let mut decoded=Vec::<Genome>::new();let mut failures=0;let mut errors=Vec::new();
    let mut genome_values=0;let mut gene_values=0;
    for case in cases {
        assert_eq!(case.settings.len(),147);assert_eq!(case.random.len(),2);
        assert_eq!(case.random["initial"],case.random["after"]);
        let bytes=fs::read(base.join(&case.file)).unwrap();assert_eq!(bytes.len(),case.bytes);
        if case.success.unwrap() {
            assert!(!case.diagnostic && case.carrier);validate_genome(&case.stage,&gene_schema);
            let genome=parse_genome_xml(&bytes).unwrap_or_else(|error|panic!("{}: {error}",case.name));
            let mut expected=case.stage.genome.clone();assert!(expected.remove("_reference").is_some());
            let mut actual=genome_fields(&genome);actual.insert("_nGenes".into(),genome.genes().unwrap().len().to_string());
            compare_fields(&case.name,"parsed","ISO9-G",&actual,&expected,&mut errors);genome_values+=expected.len();
            for (index,fields) in &case.stage.genes {
                compare_fields(&case.name,"parsed","ISO9-N",&gene_fields(&genome.gene_ref(*index).unwrap().borrow()),fields,&mut errors);
                gene_values+=fields.len();
            }
            let genes=genome.genes().unwrap();
            let aliases:Vec<_>=genes.iter().enumerate().map(|(index,gene)|genes[..=index].iter().position(|other|Rc::ptr_eq(gene,other)).unwrap()).collect();
            assert_eq!(aliases,case.aliases.unwrap(),"{}",case.name);
            for previous in &decoded {
                assert!(!genes.iter().any(|gene|previous.genes().unwrap().iter().any(|other|Rc::ptr_eq(gene,other))),"{} reused a previous Gene",case.name);
            }
            decoded.push(genome);
        } else {
            assert!(case.diagnostic && !case.carrier && case.aliases.is_none());
            assert!(case.stage.genome.is_empty() && case.stage.genes.is_empty());
            let error=parse_genome_xml(&bytes).unwrap_err();
            if case.name=="declaration-conflict" { assert!(matches!(error,XmlError::Encoding(_))); }
            else { assert!(matches!(error,XmlError::Malformed(_)),"{}: {error}",case.name); }
            failures+=1;
        }
    }
    assert_eq!((decoded.len(),failures,genome_values,gene_values),(226,34,6102,9040));
    let casing=base.parent().unwrap().join("casing");let mut casing_calls=0;
    let casing_rows=include_str!("../tests/fixtures/xml_iso_8859_9/casing/observations.tsv");
    assert_eq!(casing_rows.lines().count(),2);
    for (line,(model,genes,aliases)) in casing_rows.lines().zip(&casing_targets) {
        let row:Vec<_>=line.split('\t').collect();assert_eq!(row.len(),5);assert_eq!(&row[1..3],&["success","1"]);
        let genome=parse_genome_xml(&fs::read(casing.join(format!("{}.bgg",row[0]))).unwrap()).unwrap();
        let mut expected=model.clone();assert!(expected.remove("_reference").is_some());
        let mut actual=genome_fields(&genome);actual.insert("_nGenes".into(),genome.genes().unwrap().len().to_string());
        compare_fields(row[4],"parsed","ISO9-G",&actual,&expected,&mut errors);
        for (index,fields) in genes {
            compare_fields(row[4],"parsed","ISO9-N",&gene_fields(&genome.gene_ref(*index).unwrap().borrow()),fields,&mut errors);
        }
        let parsed=genome.genes().unwrap();
        let actual_aliases:Vec<_>=parsed.iter().enumerate().map(|(index,gene)|parsed[..=index].iter().position(|other|Rc::ptr_eq(gene,other)).unwrap()).collect();
        assert_eq!(&actual_aliases,aliases);casing_calls+=1;
    }
    assert_eq!(casing_calls,2);
    assert!(errors.is_empty(),"ISO-8859-9 mismatches:\n{}",errors.join("\n"));
    println!("XML_ISO_8859_9_READER PASS calls=260 successful_states=226 failures=34 genome_fields=6102 gene_fields=9040 aliases=226 copied_source_rng_rows=520 casing_calls=2");
}


struct InternalDtdReplay {
    cases: usize,
    rows: usize,
    counts: BTreeMap<&'static str, usize>,
    totals: (usize, usize, usize, usize, usize),
}

fn replay_internal_dtd(
    rows: &'static str,
    fixture: &str,
    unsupported_markup: &[&str],
) -> InternalDtdReplay {
    #[derive(Default)]
    struct SequenceCase {
        name: String,
        model: String,
        file: String,
        bytes: usize,
        settings: Fields,
        stage: Stage,
        success: Option<bool>,
        aliases: Option<Vec<usize>>,
        random: BTreeMap<String, (i64, i32, u64)>,
        diagnostic: bool,
    }
    let (settings_schema, _) = primitive_reference_schemas();
    let gene_schema = gene_fields(&Gene::default()).keys().cloned().collect();
    let shapes = BTreeMap::from([
        ("C", 4), ("F", 7), ("U", 4), ("R", 5), ("O", 2),
        ("X", 3), ("G", 3), ("N", 4), ("A", 2), ("E", 1),
    ]);
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut cases = Vec::new();
    let mut active = None;
    for line in rows.lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), shapes[row[0]], "Internal DTD row: {line}");
        *counts.entry(row[0]).or_default() += 1;
        if row[0] == "C" {
            assert!(active.is_none());
            assert_eq!(row[1].parse::<usize>().unwrap(), cases.len());
            assert!(!row[2].is_empty() && row[2].bytes().all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-'));
            active = Some(SequenceCase { name: row[2].into(), model: row[3].into(), ..Default::default() });
        }
        let case = active.as_mut().expect("Internal DTD row outside case");
        match row[0] {
            "C" => {}
            "F" => {
                assert!(case.file.is_empty());
                assert_eq!(row[1], format!("{}.bgg", case.name));
                assert_eq!(row[2].len(), 64);
                assert!(row[2].bytes().all(|value| value.is_ascii_hexdigit()));
                assert_eq!(row[4], case.model);
                assert!(row[5].parse::<usize>().unwrap() <= 4);
                assert!(["0000", "0246", "0247", "0248", "0258"].contains(&row[6]));
                case.file = row[1].into();
                case.bytes = row[3].parse().unwrap();
                assert!(case.bytes <= 32768);
            }
            "U" => {
                assert_eq!(settings_schema[row[1]], row[2]);
                super::death_oracle_tests::validate(row[2], row[3]);
                insert(&mut case.settings, row[1], row[3]);
            }
            "R" => {
                let value = (row[2].parse().unwrap(), row[3].parse::<i32>().unwrap(), u64::from_str_radix(row[4], 16).unwrap());
                assert!((0..10000).contains(&value.1));
                assert!(case.random.insert(row[1].into(), value).is_none());
            }
            "O" => {
                assert!(matches!(row[1], "success" | "error"));
                assert!(case.success.replace(row[1] == "success").is_none());
            }
            "X" => {
                assert!(!case.diagnostic);
                assert_eq!(row[1], "org.xml.sax.SAXException");
                case.diagnostic = true;
            }
            "G" => insert(&mut case.stage.genome, row[1], row[2]),
            "N" => insert(case.stage.genes.entry(row[1].parse().unwrap()).or_default(), row[2], row[3]),
            "A" => {
                let roots = if row[1].is_empty() { Vec::new() }
                    else { row[1].split(',').map(|value| value.parse().unwrap()).collect() };
                assert!(case.aliases.replace(roots).is_none());
            }
            "E" => cases.push(active.take().unwrap()),
            _ => unreachable!(),
        }
    }
    assert!(active.is_none());
    let case_count = cases.len();
    let base = crate::oracle_tests::fixture_path("crates/core").join("tests/fixtures").join(fixture).join("cases");
    let mut decoded = Vec::<Genome>::new();
    let mut errors = Vec::new();
    let (mut failures, mut genome_values, mut gene_values, mut empty) = (0, 0, 0, 0);
    for case in cases {
        assert_eq!(case.settings.len(), 147);
        assert_eq!(case.random.len(), 2);
        assert_eq!(case.random["initial"], case.random["after"]);
        let bytes = fs::read(base.join(&case.file)).unwrap();
        assert_eq!(bytes.len(), case.bytes);
        if case.success.unwrap() {
            assert!(!case.diagnostic);
            validate_genome(&case.stage, &gene_schema);
            let genome = parse_genome_xml(&bytes).unwrap_or_else(|error| panic!("{}: {error}", case.name));
            let mut expected = case.stage.genome.clone();
            assert!(expected.remove("_reference").is_some());
            let genes = genome.genes().unwrap();
            let mut actual = genome_fields(&genome);
            actual.insert("_nGenes".into(), genes.len().to_string());
            compare_fields(&case.name, "parsed", "DTD-G", &actual, &expected, &mut errors);
            genome_values += expected.len();
            for (index, fields) in &case.stage.genes {
                compare_fields(&case.name, "parsed", "DTD-N", &gene_fields(&genes[*index].borrow()), fields, &mut errors);
                gene_values += fields.len();
            }
            let aliases: Vec<_> = genes.iter().enumerate().map(|(index, gene)| {
                genes[..=index].iter().position(|other| Rc::ptr_eq(gene, other)).unwrap()
            }).collect();
            assert_eq!(aliases, case.aliases.unwrap(), "{}", case.name);
            for previous in &decoded {
                assert!(!genome.gene_array().unwrap().same_instance(previous.gene_array().unwrap()), "{} reused a Gene array", case.name);
                assert!(!genes.iter().any(|gene| previous.genes().unwrap().iter().any(|other| Rc::ptr_eq(gene, other))), "{} reused a previous Gene", case.name);
            }
            if genes.is_empty() { empty += 1; }
            decoded.push(genome);
        } else {
            assert!(case.diagnostic && case.aliases.is_none());
            assert!(case.stage.genome.is_empty() && case.stage.genes.is_empty());
            let error = parse_genome_xml(&bytes).unwrap_err();
            if unsupported_markup.contains(&case.name.as_str()) {
                assert!(matches!(error, XmlError::UnsupportedDtd("entity markup in element text")), "{}: {error}", case.name);
            } else {
                assert!(matches!(error, XmlError::Malformed(_)), "{}: {error}", case.name);
            }
            failures += 1;
        }
    }
    assert!(errors.is_empty(), "Internal DTD mismatches:\n{}", errors.join("\n"));
    InternalDtdReplay {
        cases: case_count,
        rows: rows.lines().count(),
        counts,
        totals: (decoded.len(), failures, genome_values, gene_values, empty),
    }
}

#[test]
fn internal_dtd_sequences_match_frozen_source_models_errors_and_fresh_owners() {
    let result = replay_internal_dtd(
        include_str!("../tests/fixtures/xml_internal_dtd_sequences/observations.tsv"),
        "xml_internal_dtd_sequences", &[],
    );
    assert_eq!(result.cases, 66);
    assert_eq!(result.rows, 14504);
    assert_eq!(result.counts, BTreeMap::from([
        ("C", 66), ("F", 66), ("U", 9702), ("R", 132), ("O", 66),
        ("X", 31), ("G", 980), ("N", 3360), ("A", 35), ("E", 66),
    ]));
    assert_eq!(result.totals, (35, 31, 945, 3360, 3));
    println!("XML_INTERNAL_DTD_READER PASS calls=66 successful_states=35 failures=31 genome_fields=945 gene_fields=3360 aliases=35 fresh_arrays=35 zero_gene_successes=3 copied_source_rng_rows=132 writer_calls_executed=0");
}

#[test]
fn mixed_content_matches_frozen_source_models_entities_and_fresh_owners() {
    let result = replay_internal_dtd(
        include_str!("../tests/fixtures/xml_mixed_content/observations.tsv"),
        "xml_mixed_content", &[],
    );
    assert_eq!(result.cases, 79);
    assert_eq!(result.rows, 16478);
    assert_eq!(result.counts, BTreeMap::from([
        ("C", 79), ("F", 79), ("U", 11613), ("R", 158), ("O", 79),
        ("X", 25), ("G", 1512), ("N", 2800), ("A", 54), ("E", 79),
    ]));
    assert_eq!(result.totals, (54, 25, 1458, 2800, 10));
    println!("XML_MIXED_CONTENT_READER PASS calls=79 successful_states=54 failures=25 unsupported_markup=0 malformed=25 genome_fields=1458 gene_fields=2800 aliases=54 fresh_arrays=54 zero_gene_successes=10 copied_source_rng_rows=158 writer_calls_executed=0");
}

fn replay_attribute_observations(
    rows: &str,
    directory: &str,
    expected_counts: BTreeMap<&str, usize>,
    expected: (usize, usize, usize, usize, usize, usize, usize),
    expected_source: (usize, usize, usize, usize, usize),
) {
    let base = crate::oracle_tests::fixture_path("crates/core").join("tests/fixtures").join(directory).join("cases");
    replay_attribute_observations_at(rows, base, expected_counts, expected, expected_source);
}

fn replay_attribute_observations_at(
    rows: &str,
    base: PathBuf,
    expected_counts: BTreeMap<&str, usize>,
    expected: (usize, usize, usize, usize, usize, usize, usize),
    expected_source: (usize, usize, usize, usize, usize),
) {
    fn base64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();
        for part in bytes.chunks(3) {
            let value = (u32::from(part[0]) << 16)
                | (u32::from(*part.get(1).unwrap_or(&0)) << 8)
                | u32::from(*part.get(2).unwrap_or(&0));
            for index in 0..4 {
                result.push(if index > part.len() { '=' }
                    else { ALPHABET[((value >> (18 - 6 * index)) & 63) as usize] as char });
            }
        }
        result
    }
    #[derive(Default)]
    struct AttributeCase {
        index: usize,
        name: String,
        model: String,
        file: String,
        bytes: usize,
        settings: Fields,
        stage: Stage,
        success: Option<bool>,
        aliases: Option<Vec<usize>>,
        random: BTreeMap<String, (i64, i32, u64)>,
        diagnostic: bool,
        dom: Option<bool>,
        carriers: Vec<Vec<String>>,
    }
    const CARRIERS: [&str; 7] = ["probe", "id", "ref", "refs", "other", "symmetry", "branch"];
    let (settings_schema, _) = primitive_reference_schemas();
    let gene_schema = gene_fields(&Gene::default()).keys().cloned().collect();
    let shapes = BTreeMap::from([
        ("C", 4), ("F", 7), ("U", 4), ("R", 5), ("O", 2), ("X", 3),
        ("G", 3), ("N", 4), ("A", 2), ("D", 2), ("V", 7), ("E", 1),
    ]);
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut cases = Vec::new();
    let mut active = None;
    for line in rows.lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), shapes[row[0]], "Tokenized row: {line}");
        *counts.entry(row[0]).or_default() += 1;
        if row[0] == "C" {
            assert!(active.is_none());
            let index = row[1].parse().unwrap();
            assert_eq!(index, cases.len());
            assert!(!row[2].is_empty() && row[2].bytes().all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-'));
            active = Some(AttributeCase { index, name: row[2].into(), model: row[3].into(), ..Default::default() });
        }
        let case = active.as_mut().expect("Tokenized row outside case");
        match row[0] {
            "C" => {}
            "F" => {
                assert!(case.file.is_empty());
                assert_eq!(row[1], format!("{}.bgg", case.name));
                assert_eq!(row[2].len(), 64);
                assert!(row[2].bytes().all(|value| value.is_ascii_hexdigit()));
                assert_eq!(row[4], case.model);
                assert!(row[5].parse::<usize>().unwrap() <= 4);
                assert!(["0000", "0246", "0247", "0248", "0258"].contains(&row[6]));
                case.file = row[1].into();
                case.bytes = row[3].parse().unwrap();
                assert!(case.bytes <= 32768);
            }
            "U" => {
                assert_eq!(settings_schema[row[1]], row[2]);
                super::death_oracle_tests::validate(row[2], row[3]);
                insert(&mut case.settings, row[1], row[3]);
            }
            "R" => {
                let value = (row[2].parse().unwrap(), row[3].parse::<i32>().unwrap(), u64::from_str_radix(row[4], 16).unwrap());
                assert!((0..10000).contains(&value.1));
                assert!(case.random.insert(row[1].into(), value).is_none());
            }
            "O" => {
                assert!(matches!(row[1], "success" | "error"));
                assert!(case.success.replace(row[1] == "success").is_none());
            }
            "X" => {
                assert!(!case.diagnostic);
                assert_eq!(row[1], "org.xml.sax.SAXException");
                case.diagnostic = true;
            }
            "G" => insert(&mut case.stage.genome, row[1], row[2]),
            "N" => insert(case.stage.genes.entry(row[1].parse().unwrap()).or_default(), row[2], row[3]),
            "A" => {
                let aliases = if row[1].is_empty() { Vec::new() }
                    else { row[1].split(',').map(|value| value.parse().unwrap()).collect() };
                assert!(case.aliases.replace(aliases).is_none());
            }
            "D" => assert!(case.dom.replace(row[1].parse().unwrap()).is_none()),
            "V" => {
                assert_eq!(case.dom, Some(true));
                assert_eq!(row[1].parse::<usize>().unwrap(), case.carriers.len() / 7);
                assert_eq!(row[3], CARRIERS[case.carriers.len() % 7]);
                assert!(["genetic_code", "gene", "aux"].contains(&row[2]));
                let present = row[4].parse::<bool>().unwrap();
                let value = if row[6].is_empty() { String::new() } else {
                    row[6].split(',').map(|point| {
                        assert_eq!(point.len(), 8);
                        assert!(point.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
                        char::from_u32(u32::from_str_radix(point, 16).unwrap()).unwrap()
                    }).collect::<String>()
                };
                assert!(value.encode_utf16().count() <= 128);
                assert_eq!(base64(value.as_bytes()), row[5]);
                if !present { assert!(value.is_empty()); }
                case.carriers.push(row[1..].iter().map(|value| (*value).to_owned()).collect());
            }
            "E" => cases.push(active.take().unwrap()),
            _ => unreachable!(),
        }
    }
    assert!(active.is_none());
    assert_eq!(cases.len(), expected_counts["C"]);
    assert_eq!(rows.lines().count(), expected_counts.values().sum::<usize>());
    assert_eq!(counts, expected_counts);
    let mut decoded = Vec::<Genome>::new();
    let mut errors = Vec::new();
    let (mut failures, mut genome_values, mut gene_values, mut empty) = (0, 0, 0, 0);
    let (mut source_successes, mut source_genome_values, mut source_gene_values, mut source_dom, mut source_elements) = (0, 0, 0, 0, 0);
    let (mut compared_dom, mut compared_carriers) = (0, 0);
    for case in cases {
        assert_eq!(case.settings.len(), 147);
        assert_eq!(case.random.len(), 2);
        assert_eq!(case.random["initial"], case.random["after"]);
        assert!(case.dom.is_some());
        assert_eq!(case.carriers.len() % 7, 0);
        assert!(case.carriers.len() <= 112);
        if case.dom.unwrap() {
            assert!(!case.carriers.is_empty());
            source_dom += 1;
            source_elements += case.carriers.len() / 7;
        } else { assert!(case.carriers.is_empty()); }
        if case.success.unwrap() {
            assert!(!case.diagnostic && case.dom.unwrap());
            validate_genome(&case.stage, &gene_schema);
            source_successes += 1;
            source_genome_values += case.stage.genome.len() - 1;
            source_gene_values += case.stage.genes.values().map(Fields::len).sum::<usize>();
            let aliases = case.aliases.as_ref().unwrap();
            assert_eq!(aliases.len(), case.stage.genes.len());
            for (index, root) in aliases.iter().enumerate() {
                assert!(*root <= index && aliases[*root] == *root);
                assert_eq!(case.stage.genes[&index], case.stage.genes[root]);
            }
        } else {
            assert!(case.diagnostic && case.aliases.is_none());
            assert!(case.stage.genome.is_empty() && case.stage.genes.is_empty());
        }
        let bytes = fs::read(base.join(&case.file)).unwrap();
        assert_eq!(bytes.len(), case.bytes);
        let mut observed = Vec::new();
        let mut complete = false;
        let result = crate::xml::parse_genome_xml_observed(&bytes, &mut observed, &mut complete);
        assert_eq!(complete, case.dom.unwrap(), "{} document completion", case.name);
        let mut actual_carriers = Vec::new();
        if complete {
            compared_dom += 1;
            for (index, (name, attributes)) in observed.iter().enumerate() {
                for key in CARRIERS {
                    let value = attributes.get(key);
                    let text = value.map(String::as_str).unwrap_or("");
                    let points = text.chars().map(|value| format!("{:08x}", value as u32)).collect::<Vec<_>>().join(",");
                    actual_carriers.push(vec![index.to_string(), name.clone(), key.to_owned(), value.is_some().to_string(), base64(text.as_bytes()), points]);
                }
            }
        }
        compared_carriers += actual_carriers.len();
        assert_eq!(actual_carriers, case.carriers, "{} normalized parser attributes", case.name);
        if case.success.unwrap() {
            let genome = result.unwrap_or_else(|error| panic!("{}: {error}", case.name));
            let mut expected = case.stage.genome.clone();
            assert!(expected.remove("_reference").is_some());
            let genes = genome.genes().unwrap();
            let mut actual = genome_fields(&genome);
            actual.insert("_nGenes".into(), genes.len().to_string());
            compare_fields(&case.name, "parsed", "ATTRIBUTE-G", &actual, &expected, &mut errors);
            genome_values += expected.len();
            for (index, fields) in &case.stage.genes {
                compare_fields(&case.name, "parsed", "ATTRIBUTE-N", &gene_fields(&genes[*index].borrow()), fields, &mut errors);
                gene_values += fields.len();
            }
            let aliases: Vec<_> = genes.iter().enumerate().map(|(index, gene)| genes[..=index].iter().position(|other| Rc::ptr_eq(gene, other)).unwrap()).collect();
            assert_eq!(aliases, case.aliases.unwrap(), "{}", case.name);
            for previous in &decoded {
                assert!(!genome.gene_array().unwrap().same_instance(previous.gene_array().unwrap()), "{} reused a Gene array", case.name);
                assert!(!genes.iter().any(|gene| previous.genes().unwrap().iter().any(|other| Rc::ptr_eq(gene, other))), "{} reused a previous Gene", case.name);
            }
            if genes.is_empty() { empty += 1; }
            decoded.push(genome);
        } else {
            let error = result.unwrap_err();
            match case.name.as_str() {
                "id-required-missing" | "notation-required-missing" => assert_eq!(error, XmlError::MissingAttribute("probe".into())),
                "enum-direct-numeric-whitespace" | "precedence-root-duplicate-numeric-only" | "precedence-root-entity-numeric-only" | "precedence-root-numeric-only" | "root-numeric-only" => assert_eq!(error, XmlError::InvalidAttribute("symmetry".into())),
                "precedence-gene-unresolved-numeric-only" | "precedence-gene-notation-numeric-only" | "precedence-gene-numeric-only" => assert_eq!(error, XmlError::InvalidAttribute("branch".into())),
                "pe-numeric-only" => assert_eq!(error, XmlError::InvalidAttribute("mutationrate".into())),
                "notation-fixed-mismatch" => assert_eq!(error, XmlError::InvalidAttribute("probe".into())),
                "parsed-system-text-consumption" | "parsed-external-attribute-consumption"
                | "parsed-external-default-consumption" | "parsed-external-indirect-consumption"
                | "nested-external-consumption" | "pe-external-system-used" | "pe-external-public-used" => assert_eq!(error, XmlError::ExternalResource),
                _ => assert!(matches!(error, XmlError::Malformed(_)), "{}: {error}", case.name),
            }
            failures += 1;
        }
    }
    assert_eq!((source_successes, source_genome_values, source_gene_values, source_dom, source_elements), expected_source);
    assert_eq!((decoded.len(), failures, genome_values, gene_values, empty, compared_dom, compared_carriers), expected);
    assert!(errors.is_empty(), "Attribute mismatches:\n{}", errors.join("\n"));
}

#[test]
fn tokenized_attributes_match_frozen_source_fields_carriers_and_validation_order() {
    replay_attribute_observations(
        include_str!("../tests/fixtures/xml_tokenized_attributes/observations.tsv"),
        "xml_tokenized_attributes",
        BTreeMap::from([
            ("C", 71), ("F", 71), ("U", 10437), ("R", 142), ("O", 71), ("X", 31),
            ("G", 1120), ("N", 3040), ("A", 40), ("D", 71), ("V", 861), ("E", 71),
        ]),
        (40, 31, 1080, 3040, 1, 42, 861), (40, 1080, 3040, 42, 123),
    );
    println!("XML_TOKENIZED_ATTRIBUTES_READER PASS calls=71 admitted=40 supported_rejections=31 unsupported=0 genome_fields=1080 gene_fields=3040 compared_DOM=42 compared_carriers=861 fresh_arrays=40 zero_gene_successes=1 copied_source_rng_rows=142 writer_calls_executed=0");
}

#[test]
fn entity_notation_matches_frozen_source_fields_carriers_and_validation_order() {
    replay_attribute_observations(
        include_str!("../tests/fixtures/xml_entity_notation/observations.tsv"),
        "xml_entity_notation",
        BTreeMap::from([
            ("C", 79), ("F", 79), ("U", 11613), ("R", 158), ("O", 79), ("X", 45),
            ("G", 952), ("N", 2480), ("A", 34), ("D", 79), ("V", 728), ("E", 79),
        ]),
        (34, 45, 918, 2480, 1, 36, 728), (34, 918, 2480, 36, 104),
    );
    println!("XML_ENTITY_NOTATION_READER PASS calls=79 admitted=34 rejected=45 genome_fields=918 gene_fields=2480 compared_DOM=36 compared_carriers=728 fresh_arrays=34 zero_gene_successes=1 copied_source_rng_rows=158 writer_calls_executed=0");
}

#[test]
fn empty_entities_defaults_match_frozen_source_application_and_unused_owners() {
    replay_attribute_observations(
        include_str!("../tests/fixtures/xml_entity_empty_defaults/observations.tsv"),
        "xml_entity_empty_defaults",
        BTreeMap::from([
            ("C", 3), ("F", 3), ("U", 441), ("R", 6), ("O", 3), ("X", 1),
            ("G", 56), ("N", 160), ("A", 2), ("D", 3), ("V", 42), ("E", 3),
        ]),
        (2, 1, 54, 160, 0, 2, 42), (2, 54, 160, 2, 6),
    );
    println!("XML_ENTITY_EMPTY_DEFAULTS_READER PASS calls=3 admitted=2 rejected=1 genome_fields=54 gene_fields=160 compared_DOM=2 compared_carriers=42 fresh_arrays=2 copied_source_rng_rows=6 writer_calls_executed=0");
}

#[test]
fn child_models_match_frozen_source_order_fields_carriers_and_completion() {
    replay_attribute_observations(
        include_str!("../tests/fixtures/xml_child_models/observations.tsv"),
        "xml_child_models",
        BTreeMap::from([
            ("C", 113), ("F", 113), ("U", 16611), ("R", 226), ("O", 113), ("X", 45),
            ("G", 1904), ("N", 5040), ("A", 68), ("D", 113), ("V", 1862), ("E", 113),
        ]),
        (68, 45, 1836, 5040, 6, 72, 1862), (68, 1836, 5040, 72, 266),
    );
    println!("XML_CHILD_MODELS_READER PASS calls=113 admitted=68 rejected=45 genome_fields=1836 gene_fields=5040 compared_DOM=72 compared_carriers=1862 fresh_arrays=68 zero_gene_successes=6 copied_source_rng_rows=226 writer_calls_executed=0");
}

#[test]
fn entity_markup_matches_frozen_source_fields_carriers_and_completion() {
    replay_attribute_observations_at(
        include_str!("../../../tests/fixtures/java/xml_entity_markup/observations.tsv"),
        crate::oracle_tests::fixture_path("crates/core").join("../../tests/fixtures/java/xml_entity_markup/cases"),
        BTreeMap::from([
            ("C", 28), ("F", 28), ("U", 4116), ("R", 56), ("O", 28), ("X", 12),
            ("G", 448), ("N", 960), ("A", 16), ("D", 28), ("V", 343), ("E", 28),
        ]),
        (16, 12, 432, 960, 0, 17, 343), (16, 432, 960, 17, 49),
    );
    println!("XML_ENTITY_MARKUP_READER PASS calls=28 admitted=16 rejected=12 genome_fields=432 gene_fields=960 compared_DOM=17 compared_carriers=343 fresh_arrays=16 copied_source_rng_rows=56 writer_calls_executed=0");
}


#[test]
fn parameter_entities_match_frozen_source_fields_carriers_and_completion() {
    replay_attribute_observations(
        include_str!("../tests/fixtures/java/xml_parameter_entities/observations.tsv"),
        "java/xml_parameter_entities",
        BTreeMap::from([
            ("C", 32), ("F", 32), ("U", 4704), ("R", 64), ("O", 32), ("X", 16),
            ("G", 448), ("N", 680), ("A", 16), ("D", 32), ("V", 245), ("E", 32),
        ]),
        (16, 16, 432, 680, 0, 17, 245), (16, 432, 680, 17, 35),
    );
    println!("XML_PARAMETER_ENTITIES_READER PASS calls=32 admitted=16 rejected=16 genome_fields=432 gene_fields=680 compared_DOM=17 compared_carriers=245 fresh_arrays=16 copied_source_rng_rows=64 writer_calls_executed=0");
}

#[path = "xml_charset_catalog_tests.rs"]
mod xml_charset_catalog_tests;

#[path = "xml11_reader_tests.rs"]
mod xml11_reader_tests;

#[path = "xml11_context_tests.rs"]
mod xml11_context_tests;

#[path = "xml11_declaration_handoff_tests.rs"]
mod xml11_declaration_handoff_tests;
