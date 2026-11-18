// SPDX-License-Identifier: GPL-2.0-or-later

//! Replay the observed Original declaration reader handoff inputs

use super::*;
use super::xml_charset_catalog_tests::{
    charset_base64, charset_compare_model, charset_expected_model, charset_source_settings_and_rng,
};
use super::xml11_reader_tests::{CARRIER_KEYS, xml11_expected_carriers, xml11_expected_text};

const FIXTURE: &str = "tests/fixtures/xml11_declaration_handoff";

#[test]
fn xml11_declaration_handoff_matches_original_fields_carriers_text_and_completion() {
    charset_source_settings_and_rng();
    let (expected, aliases) = charset_expected_model();
    let mut carriers = xml11_expected_carriers(
        include_str!("../tests/fixtures/xml11_declaration_handoff/carriers.tsv"), 6, 108,
    );
    assert!(carriers.values().all(|rows| rows.len() == 18));
    let mut texts = xml11_expected_text(
        include_str!("../tests/fixtures/xml11_declaration_handoff/text.tsv"), 6,
    );
    let base = crate::oracle_tests::fixture_path("crates/core").join(FIXTURE);
    let mut lines = include_str!("../tests/fixtures/xml11_declaration_handoff/cases.tsv").lines();
    assert_eq!(lines.next().unwrap(), "index\tcohort\tSource_case\tfile\tbytes\tsha256\toutcome\tDOM\texception\tmessage_b64\tlabel_b64\txml_encoding_b64\tinput_encoding_b64\txml_version_b64\tname\terror_context");
    let mut decoded = Vec::<Genome>::new();
    let mut arrays = BTreeSet::new();
    let mut genes_seen = BTreeSet::new();
    let mut errors = Vec::new();
    let mut parser_mismatches = 0;
    let mut parser_examples = Vec::new();
    let (mut calls, mut rejected, mut completed, mut compared_carriers, mut compared_texts) = (0, 0, 0, 0, 0);
    for (index, line) in lines.enumerate() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 16);
        assert_eq!(row[0].parse::<usize>().unwrap(), index);
        assert_eq!(row[1], "xml11-declaration-handoff");
        assert_eq!(row[2].parse::<usize>().unwrap(), index);
        assert_eq!(row[3], format!("cases/case-{index:05}.bgg"));
        assert_eq!(row[5].len(), 64);
        assert!(row[5].bytes().all(|byte| byte.is_ascii_hexdigit()));
        let label = format!("XML1.1 declaration handoff Source{index} {}", row[14]);
        let bytes = fs::read(base.join(row[3])).unwrap();
        assert_eq!(bytes.len(), row[4].parse::<usize>().unwrap(), "{label}");
        assert!(bytes.len() <= 8192);
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
            let actual = result.as_ref().map(|_| "success".to_owned()).unwrap_or_else(|error| error.to_string());
            parser_examples.push(format!("{label}: expected {} DOM={}, got {actual} DOM={complete}", row[6], row[7]));
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
        assert_eq!(row[15], "~");
        match row[6] {
            "success" => {
                assert!(complete);
                assert_eq!(&row[8..10], &["~", "~"]);
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
                assert!(!complete);
                assert_eq!(row[8], "org.xml.sax.SAXException");
                assert!(!row[9].is_empty() && row[9] != "~");
                assert!(!texts.contains_key(&index));
                let error = result.unwrap_err();
                // Source rejection does not select Rust's existing declaration or XML error variant
                assert!(matches!(error, XmlError::Encoding(_) | XmlError::Malformed(_)), "{label}: {error}");
                rejected += 1;
            }
            _ => panic!("Unexpected Source declaration handoff outcome: {}", row[6]),
        }
        calls += 1;
    }
    assert_eq!(parser_mismatches, 0, "XML1.1 declaration handoff parser/completion mismatches: {parser_mismatches}\n{}", parser_examples.join("\n"));
    assert_eq!((calls, decoded.len(), rejected), (10, 6, 4));
    assert_eq!((completed, compared_carriers, compared_texts), (6, 108, 6));
    assert!(carriers.is_empty() && texts.is_empty());
    assert_eq!((arrays.len(), genes_seen.len()), (6, 6));
    for (index, genome) in decoded.iter().enumerate() {
        charset_compare_model(&format!("retained XML1.1 declaration handoff owner {index}"), genome, &expected, &mut errors);
    }
    assert!(errors.is_empty(), "XML1.1 declaration handoff field mismatches:\n{}", errors.join("\n"));
    println!("XML11_DECLARATION_HANDOFF PASS calls=10 admitted=6 rejected=4 Source_G28=168 N40=240 compared_DOM=6 carriers=108 text=6 fresh_arrays=6 copied_Source_RNG_rows=20 writer_calls=0");
}
