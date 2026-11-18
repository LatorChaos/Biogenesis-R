// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay additional XML1.0/XML1.1 lexical and declaration contexts from Original calls

use super::*;
use super::xml_charset_catalog_tests::{
    charset_base64, charset_compare_model, charset_expected_model, charset_source_settings_and_rng,
};
use super::xml11_reader_tests::{CARRIER_KEYS, xml11_expected_carriers, xml11_expected_text};

const FIXTURE: &str = "tests/fixtures/xml11_contexts";

#[test]
fn xml11_contexts_match_original_fields_carriers_text_and_completion() {
    charset_source_settings_and_rng();
    let (expected, aliases) = charset_expected_model();
    let mut carriers = xml11_expected_carriers(
        include_str!("../tests/fixtures/xml11_contexts/carriers.tsv"), 53, 1026,
    );
    assert_eq!(carriers.values().filter(|rows| rows.len() == 18).count(), 45);
    assert_eq!(carriers.values().filter(|rows| rows.len() == 27).count(), 8);
    let mut texts = xml11_expected_text(
        include_str!("../tests/fixtures/xml11_contexts/text.tsv"), 45,
    );
    let base = crate::oracle_tests::fixture_path("crates/core").join(FIXTURE);
    let mut lines = include_str!("../tests/fixtures/xml11_contexts/cases.tsv").lines();
    assert_eq!(lines.next().unwrap(), "index\tcohort\tSource_case\tfile\tbytes\tsha256\toutcome\tDOM\texception\tmessage_b64\tlabel_b64\txml_encoding_b64\tinput_encoding_b64\txml_version_b64\tname\terror_context");
    let mut decoded = Vec::<Genome>::new();
    let mut arrays = BTreeSet::new();
    let mut genes_seen = BTreeSet::new();
    let mut errors = Vec::new();
    let mut parser_mismatches = 0;
    let mut parser_examples = Vec::new();
    let (mut calls, mut rejected, mut completed, mut compared_carriers, mut compared_texts) = (0, 0, 0, 0, 0);
    let mut typed_child_errors = 0;
    for (index, line) in lines.enumerate() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 16);
        assert_eq!(row[0].parse::<usize>().unwrap(), index);
        assert_eq!(row[1], "xml11-contexts");
        assert_eq!(row[2].parse::<usize>().unwrap(), index);
        assert_eq!(row[3], format!("cases/case-{index:05}.bgg"));
        assert_eq!(row[5].len(), 64);
        assert!(row[5].bytes().all(|byte| byte.is_ascii_hexdigit()));
        let label = format!("XML1.1 context Source{index} {}", row[14]);
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
                assert_eq!(text, texts.remove(&index).unwrap(), "{label} Document text");
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
                    assert_eq!(row[15], "aux");
                    assert!(observed.iter().any(|(name, _)| name == "aux"));
                    assert!(matches!(error, XmlError::Malformed(_)), "{label}: {error}");
                    typed_child_errors += 1;
                } else {
                    assert_eq!(row[15], "~");
                    // Source rejection does not select Rust's existing declaration or XML error variant
                    assert!(matches!(error, XmlError::Encoding(_) | XmlError::Malformed(_)), "{label}: {error}");
                }
                rejected += 1;
            }
            _ => panic!("Unexpected Source XML1.1 context outcome: {}", row[6]),
        }
        calls += 1;
    }
    assert_eq!(parser_mismatches, 0, "XML1.1 context parser/completion mismatches: {parser_mismatches}\n{}", parser_examples.join("\n"));
    assert_eq!((calls, decoded.len(), rejected), (98, 45, 53));
    assert_eq!((completed, compared_carriers, compared_texts, typed_child_errors), (53, 1026, 45, 8));
    assert!(carriers.is_empty() && texts.is_empty());
    assert_eq!((arrays.len(), genes_seen.len()), (45, 45));
    for (index, genome) in decoded.iter().enumerate() {
        charset_compare_model(&format!("retained XML1.1 context owner {index}"), genome, &expected, &mut errors);
    }
    assert!(errors.is_empty(), "XML1.1 context field mismatches:\n{}", errors.join("\n"));
    println!("XML11_CONTEXTS PASS calls=98 admitted=45 rejected=53 Source_G28=1260 N40=1800 compared_DOM=53 carriers=1026 text=45 fresh_arrays=45 copied_Source_RNG_rows=196 writer_calls=0");
}
