// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay the finite Original charset catalog through the observed XML boundary
//! Shared expected fields were compared across every successful Source call
//! Copied Java RNG rows establish Source purity, not Rust generator measurements

use super::*;

const FIXTURE: &str = "tests/fixtures/xml_charset_catalog";
const CARRIER_KEYS: [&str; 7] = ["probe", "id", "ref", "refs", "other", "symmetry", "branch"];

pub(super) fn charset_base64(bytes: &[u8]) -> String {
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

pub(super) fn charset_expected_model() -> (Stage, Vec<usize>) {
    let mut stage = Stage::default();
    let mut aliases = None;
    let mut counts = BTreeMap::<&str, usize>::new();
    for line in include_str!("../tests/fixtures/xml_charset_catalog/model.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        *counts.entry(row[0]).or_default() += 1;
        match row[0] {
            "G" => {
                assert_eq!(row.len(), 3);
                insert(&mut stage.genome, row[1], row[2]);
            }
            "N" => {
                assert_eq!(row.len(), 4);
                insert(stage.genes.entry(row[1].parse().unwrap()).or_default(), row[2], row[3]);
            }
            "A" => {
                assert_eq!(row.len(), 2);
                let values = row[1].split(',').map(|value| value.parse::<usize>().unwrap()).collect();
                assert!(aliases.replace(values).is_none());
            }
            _ => panic!("Unexpected charset model row: {line}"),
        }
    }
    assert_eq!(counts, BTreeMap::from([("G", 28), ("N", 40), ("A", 1)]));
    let schema = gene_fields(&Gene::default()).keys().cloned().collect();
    validate_genome(&stage, &schema);
    let aliases = aliases.unwrap();
    assert_eq!(aliases, vec![0]);
    (stage, aliases)
}

pub(super) fn charset_source_settings_and_rng() {
    let (schema, _) = primitive_reference_schemas();
    let mut settings = Fields::new();
    let mut rng = BTreeMap::new();
    for line in include_str!("../tests/fixtures/xml_charset_catalog/settings-rng.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        match row[0] {
            "U" => {
                assert_eq!(row.len(), 4);
                assert_eq!(schema[row[1]], row[2]);
                super::super::death_oracle_tests::validate(row[2], row[3]);
                insert(&mut settings, row[1], row[3]);
            }
            "R" => {
                assert_eq!(row.len(), 5);
                let value = (row[2].parse::<i64>().unwrap(), row[3].parse::<i32>().unwrap(),
                             u64::from_str_radix(row[4], 16).unwrap());
                assert!((0..10000).contains(&value.1));
                assert!(rng.insert(row[1], value).is_none());
            }
            _ => panic!("Unexpected charset settings row: {line}"),
        }
    }
    assert_eq!(settings.len(), 147);
    assert_eq!(rng.len(), 2);
    assert_eq!(rng["initial"], rng["after"]);
}

fn charset_expected_carriers() -> BTreeMap<usize, Vec<Vec<String>>> {
    let mut cases = BTreeMap::<usize, Vec<Vec<String>>>::new();
    for line in include_str!("../tests/fixtures/xml_charset_catalog/carriers.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 7);
        let case = cases.entry(row[0].parse().unwrap()).or_default();
        assert_eq!(row[1].parse::<usize>().unwrap(), case.len() / 7);
        assert!(matches!(row[2], "genetic_code" | "gene"));
        assert_eq!(row[3], CARRIER_KEYS[case.len() % 7]);
        let present = row[4].parse::<bool>().unwrap();
        let text = if row[6].is_empty() { String::new() } else {
            row[6].split(',').map(|point| {
                assert_eq!(point.len(), 8);
                char::from_u32(u32::from_str_radix(point, 16).unwrap()).unwrap()
            }).collect::<String>()
        };
        assert!(text.encode_utf16().count() <= 128);
        assert_eq!(charset_base64(text.as_bytes()), row[5]);
        if !present { assert!(text.is_empty()); }
        case.push(row[1..].iter().map(|value| (*value).to_owned()).collect());
    }
    assert_eq!(cases.len(), 2185);
    assert!(cases.values().all(|rows| rows.len() == 14));
    cases
}

pub(super) fn charset_compare_model(label: &str, genome: &Genome, expected: &Stage, errors: &mut Vec<String>) {
    let genes = genome.genes().unwrap();
    let mut fields = expected.genome.clone();
    assert_eq!(fields.remove("_reference").as_deref(), Some("g0"));
    let mut actual = genome_fields(genome);
    actual.insert("_nGenes".into(), genes.len().to_string());
    compare_fields(label, "parsed", "CHARSET-G", &actual, &fields, errors);
    for (index, fields) in &expected.genes {
        compare_fields(label, "parsed", "CHARSET-N", &gene_fields(&genes[*index].borrow()), fields, errors);
    }
}

#[test]
fn charset_catalog_matches_complete_original_fields_carriers_and_completion() {
    charset_source_settings_and_rng();
    let (expected, aliases) = charset_expected_model();
    let mut carriers = charset_expected_carriers();
    let base = crate::oracle_tests::fixture_path("crates/core").join(FIXTURE);
    let rows = include_str!("../tests/fixtures/xml_charset_catalog/cases.tsv");
    let mut lines = rows.lines();
    assert_eq!(lines.next().unwrap(), "index\tcohort\tSource_case\tfile\tbytes\tsha256\toutcome\tDOM\texception\tmessage_b64\tlabel_b64\txml_encoding_b64\tinput_encoding_b64\txml_version_b64");
    let mut decoded = Vec::<Genome>::new();
    let mut arrays = BTreeSet::new();
    let mut genes_seen = BTreeSet::new();
    let mut errors = Vec::new();
    let mut parser_mismatches = 0;
    let mut parser_examples = Vec::new();
    let (mut calls, mut rejected, mut completed, mut compared_carriers) = (0, 0, 0, 0);
    let (mut main_calls, mut supplement_calls, mut streaming_calls, mut encoding_exceptions) = (0, 0, 0, 0);
    for (index, line) in lines.enumerate() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 14);
        assert_eq!(row[0].parse::<usize>().unwrap(), index);
        assert_eq!(row[3], format!("cases/case-{index:05}.bgg"));
        assert_eq!(row[5].len(), 64);
        assert!(row[5].bytes().all(|byte| byte.is_ascii_hexdigit()));
        match row[1] {
            "main" => { assert_eq!(index, main_calls); main_calls += 1; }
            "supplement" => { assert!(index >= 2769); supplement_calls += 1; }
            "streaming" => { assert!(index >= 2781); streaming_calls += 1; }
            _ => panic!("Unexpected charset cohort: {}", row[1]),
        }
        let label = format!("{} Source{} declared {}", row[1], row[2], row[10]);
        let bytes = fs::read(base.join(row[3])).unwrap();
        assert_eq!(bytes.len(), row[4].parse::<usize>().unwrap(), "{label}");
        assert!(bytes.len() < 16 * 1024 * 1024);
        let mut observed = Vec::new();
        let mut complete = false;
        let result = crate::xml::parse_genome_xml_observed(&bytes, &mut observed, &mut complete);
        let expected_success = row[6] == "success";
        if result.is_ok() != expected_success || complete != row[7].parse::<bool>().unwrap() {
            parser_mismatches += 1;
            if parser_examples.len() < 24 {
                let actual = result.as_ref().map(|_| "success".to_owned()).unwrap_or_else(|error| error.to_string());
                parser_examples.push(format!("{label}: expected {} DOM={}, got {actual} DOM={complete}", row[6], row[7]));
            }
            carriers.remove(&index);
            calls += 1;
            continue;
        }
        let mut actual_carriers = Vec::new();
        if complete {
            completed += 1;
            for (node, (name, attributes)) in observed.iter().enumerate() {
                for key in CARRIER_KEYS {
                    let value = attributes.get(key);
                    let text = value.map(String::as_str).unwrap_or("");
                    let points = text.chars().map(|value| format!("{:08x}", value as u32)).collect::<Vec<_>>().join(",");
                    actual_carriers.push(vec![node.to_string(), name.clone(), key.to_owned(),
                        value.is_some().to_string(), charset_base64(text.as_bytes()), points]);
                }
            }
        }
        let expected_carriers = carriers.remove(&index).unwrap_or_default();
        compared_carriers += actual_carriers.len();
        assert_eq!(actual_carriers, expected_carriers, "{label} normalized attributes");
        match row[6] {
            "success" => {
                assert_eq!(row[7], "true");
                assert_eq!(&row[8..10], &["~", "~"]);
                assert!(row[11..14].iter().all(|cell| !cell.is_empty()));
                let genome = result.unwrap_or_else(|error| panic!("{label}: {error}"));
                charset_compare_model(&label, &genome, &expected, &mut errors);
                let genes = genome.genes().unwrap();
                let actual_aliases: Vec<_> = genes.iter().enumerate().map(|(index, gene)| {
                    genes[..=index].iter().position(|other| Rc::ptr_eq(gene, other)).unwrap()
                }).collect();
                assert_eq!(actual_aliases, aliases, "{label} aliases");
                assert!(arrays.insert(genome.gene_array().unwrap().identity()), "{label} reused array");
                for gene in &genes {
                    assert!(genes_seen.insert(Rc::as_ptr(gene) as usize), "{label} reused Gene");
                }
                decoded.push(genome);
            }
            "error" => {
                assert_eq!(row[7], "false");
                assert_eq!(&row[11..14], &["~", "~", "~"]);
                assert!(!row[9].is_empty() && row[9] != "~");
                let error = result.unwrap_err();
                match row[8] {
                    "java.io.UnsupportedEncodingException" => {
                        assert!(matches!(error, XmlError::Encoding(_)), "{label}: {error}");
                        encoding_exceptions += 1;
                    }
                    "org.xml.sax.SAXException" => {
                        assert!(matches!(error, XmlError::Encoding(_) | XmlError::Malformed(_)), "{label}: {error}");
                    }
                    _ => panic!("Unexpected Source charset error: {}", row[8]),
                }
                rejected += 1;
            }
            _ => panic!("Unexpected Source outcome: {}", row[6]),
        }
        calls += 1;
    }
    assert_eq!(parser_mismatches, 0, "Charset parser/completion mismatches: {parser_mismatches}\n{}", parser_examples.join("\n"));
    assert_eq!((calls, main_calls, supplement_calls, streaming_calls), (2791, 2769, 12, 10));
    assert_eq!((decoded.len(), rejected, encoding_exceptions), (2185, 606, 22));
    assert_eq!((completed, compared_carriers), (2185, 30590));
    assert!(carriers.is_empty());
    assert_eq!((arrays.len(), genes_seen.len()), (2185, 2185));
    for (index, genome) in decoded.iter().enumerate() {
        charset_compare_model(&format!("retained charset owner {index}"), genome, &expected, &mut errors);
    }
    assert!(errors.is_empty(), "Charset field mismatches:\n{}", errors.join("\n"));
    println!("XML_CHARSET_CATALOG_READER PASS calls=2791 admitted=2185 rejected=606 Source_G28=61180 N40=87400 compared_DOM=2185 carriers=30590 fresh_arrays=2185 copied_Source_RNG_rows=5582 writer_calls=0");
}
