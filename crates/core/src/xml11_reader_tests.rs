// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay matched XML1.0/XML1.1 inputs through the actual observed reader
//! Complete expected fields and copied Source RNG reuse the qualified charset model

use super::*;
use super::xml_charset_catalog_tests::{
    charset_base64, charset_compare_model, charset_expected_model, charset_source_settings_and_rng,
};

const FIXTURE: &str = "tests/fixtures/xml11_reader";
pub(super) const CARRIER_KEYS: [&str; 9] = [
    "probe", "id", "ref", "refs", "other", "symmetry", "branch", "length", "theta",
];

fn xml11_text(encoded: &str, codepoints: &str) -> String {
    let text = if codepoints.is_empty() { String::new() } else {
        codepoints.split(',').map(|point| {
            assert_eq!(point.len(), 8);
            char::from_u32(u32::from_str_radix(point, 16).unwrap()).unwrap()
        }).collect::<String>()
    };
    assert!(text.encode_utf16().count() <= 128);
    assert_eq!(charset_base64(text.as_bytes()), encoded);
    text
}

pub(super) fn xml11_expected_carriers(rows: &str, expected_cases: usize, expected_rows: usize) -> BTreeMap<usize, Vec<Vec<String>>> {
    let mut cases = BTreeMap::<usize, Vec<Vec<String>>>::new();
    for line in rows.lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 7);
        let case = cases.entry(row[0].parse().unwrap()).or_default();
        assert_eq!(row[1].parse::<usize>().unwrap(), case.len() / CARRIER_KEYS.len());
        assert!(matches!(row[2], "genetic_code" | "gene" | "aux"));
        assert_eq!(row[3], CARRIER_KEYS[case.len() % CARRIER_KEYS.len()]);
        let present = row[4].parse::<bool>().unwrap();
        let text = xml11_text(row[5], row[6]);
        if !present { assert!(text.is_empty()); }
        case.push(row[1..].iter().map(|value| (*value).to_owned()).collect());
    }
    assert_eq!(cases.len(), expected_cases);
    assert_eq!(cases.values().map(Vec::len).sum::<usize>(), expected_rows);
    assert!(cases.values().all(|rows| rows.len() % CARRIER_KEYS.len() == 0 && rows.len() <= 16 * CARRIER_KEYS.len()));
    cases
}

pub(super) fn xml11_expected_text(rows: &str, expected_cases: usize) -> BTreeMap<usize, String> {
    let mut cases = BTreeMap::new();
    for line in rows.lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 4);
        assert_eq!(row[1], "input");
        let text = xml11_text(row[2], row[3]);
        assert!(cases.insert(row[0].parse().unwrap(), text).is_none());
    }
    assert_eq!(cases.len(), expected_cases);
    cases
}

#[test]
fn xml11_reader_matches_original_fields_carriers_text_and_completion() {
    charset_source_settings_and_rng();
    let (expected, aliases) = charset_expected_model();
    let mut carriers = xml11_expected_carriers(
        include_str!("../tests/fixtures/xml11_reader/carriers.tsv"), 38, 684,
    );
    assert!(carriers.values().all(|rows| rows.len() == 18));
    assert!(carriers.values().flatten().all(|row| matches!(row[1].as_str(), "genetic_code" | "gene")));
    let mut texts = xml11_expected_text(
        include_str!("../tests/fixtures/xml11_reader/text.tsv"), 32,
    );
    let base = crate::oracle_tests::fixture_path("crates/core").join(FIXTURE);
    let mut lines = include_str!("../tests/fixtures/xml11_reader/cases.tsv").lines();
    assert_eq!(lines.next().unwrap(), "index\tcohort\tSource_case\tfile\tbytes\tsha256\toutcome\tDOM\texception\tmessage_b64\tlabel_b64\txml_encoding_b64\tinput_encoding_b64\txml_version_b64\tname\tinvalid_attribute");
    let mut decoded = Vec::<Genome>::new();
    let mut arrays = BTreeSet::new();
    let mut genes_seen = BTreeSet::new();
    let mut errors = Vec::new();
    let mut parser_mismatches = 0;
    let mut parser_examples = Vec::new();
    let (mut calls, mut rejected, mut completed, mut compared_carriers, mut compared_texts) = (0, 0, 0, 0, 0);
    let mut numeric_errors = 0;
    for (index, line) in lines.enumerate() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 16);
        assert_eq!(row[0].parse::<usize>().unwrap(), index);
        assert_eq!(row[1], "xml11");
        assert_eq!(row[2].parse::<usize>().unwrap(), index);
        assert_eq!(row[3], format!("cases/case-{index:05}.bgg"));
        assert_eq!(row[5].len(), 64);
        assert!(row[5].bytes().all(|byte| byte.is_ascii_hexdigit()));
        let label = format!("XML1.1 Source{index} {}", row[14]);
        let bytes = fs::read(base.join(row[3])).unwrap();
        assert_eq!(bytes.len(), row[4].parse::<usize>().unwrap(), "{label}");
        assert!(bytes.len() <= 32768);
        let mut observed = Vec::new();
        let mut text = String::new();
        let mut complete = false;
        let result = crate::xml::parse_genome_xml_observed_text(
            &bytes, &mut observed, &mut text, &mut complete,
        );
        let expected_success = row[6] == "success";
        let expected_complete = row[7].parse::<bool>().unwrap();
        if result.is_ok() != expected_success || complete != expected_complete {
            parser_mismatches += 1;
            if parser_examples.len() < 24 {
                let actual = result.as_ref().map(|_| "success".to_owned()).unwrap_or_else(|error| error.to_string());
                parser_examples.push(format!("{label}: expected {} DOM={}, got {actual} DOM={complete}", row[6], row[7]));
            }
            carriers.remove(&index);
            texts.remove(&index);
            calls += 1;
            continue;
        }
        let mut actual_carriers = Vec::new();
        if complete {
            completed += 1;
            assert!(row[11..14].iter().all(|cell| !cell.is_empty() && *cell != "~"));
            for (node, (name, attributes)) in observed.iter().enumerate() {
                for key in CARRIER_KEYS {
                    let value = attributes.get(key);
                    let text = value.map(String::as_str).unwrap_or("");
                    let points = text.chars().map(|value| format!("{:08x}", value as u32)).collect::<Vec<_>>().join(",");
                    actual_carriers.push(vec![node.to_string(), name.clone(), key.to_owned(),
                        value.is_some().to_string(), charset_base64(text.as_bytes()), points]);
                }
            }
        } else {
            assert_eq!(&row[11..14], &["~", "~", "~"]);
        }
        let expected_carriers = carriers.remove(&index).unwrap_or_default();
        compared_carriers += actual_carriers.len();
        assert_eq!(actual_carriers, expected_carriers, "{label} normalized attributes");
        match row[6] {
            "success" => {
                assert!(complete);
                assert_eq!(&row[8..10], &["~", "~"]);
                assert_eq!(row[15], "~");
                let expected_text = texts.remove(&index).unwrap();
                assert_eq!(text, expected_text, "{label} Document text");
                compared_texts += 1;
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
                assert_eq!(row[8], "org.xml.sax.SAXException");
                assert!(!row[9].is_empty() && row[9] != "~");
                assert!(!texts.contains_key(&index));
                let error = result.unwrap_err();
                if complete {
                    assert!(matches!(row[15], "length" | "branch"));
                    assert_eq!(error, XmlError::InvalidAttribute(row[15].to_owned()), "{label}");
                    numeric_errors += 1;
                } else {
                    assert_eq!(row[15], "~");
                    if matches!(index, 61 | 62) {
                        // Preserve Rust's declaration error category; Source only establishes rejection
                        assert!(matches!(error, XmlError::Encoding(_)), "{label}: {error}");
                    } else {
                        assert!(matches!(error, XmlError::Malformed(_)), "{label}: {error}");
                    }
                }
                rejected += 1;
            }
            _ => panic!("Unexpected Source XML1.1 outcome: {}", row[6]),
        }
        calls += 1;
    }
    assert_eq!(parser_mismatches, 0, "XML1.1 parser/completion mismatches: {parser_mismatches}\n{}", parser_examples.join("\n"));
    assert_eq!((calls, decoded.len(), rejected), (64, 32, 32));
    assert_eq!((completed, compared_carriers, compared_texts, numeric_errors), (38, 684, 32, 6));
    assert!(carriers.is_empty() && texts.is_empty());
    assert_eq!((arrays.len(), genes_seen.len()), (32, 32));
    for (index, genome) in decoded.iter().enumerate() {
        charset_compare_model(&format!("retained XML1.1 owner {index}"), genome, &expected, &mut errors);
    }
    assert!(errors.is_empty(), "XML1.1 field mismatches:\n{}", errors.join("\n"));
    println!("XML11_READER PASS calls=64 admitted=32 rejected=32 Source_G28=896 N40=1280 compared_DOM=38 carriers=684 text=32 fresh_arrays=32 copied_Source_RNG_rows=128 writer_calls=0");
}
