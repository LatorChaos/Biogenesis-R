// SPDX-License-Identifier: GPL-2.0-or-later
// Generated observations from tests/fixtures/java/core.jsonl
// Source SHA256: 5f5aa228b3ec67685b3a180a1dca1d42364d099240112bdd33c1ed209a080b10
use super::*;

struct ExpectedGene {
    length: u64,
    theta: u64,
    rgb: u32,
    branch: i32,
    reactions: [i32; 36],
}
fn check_gene(actual: &Gene, expected: ExpectedGene) {
    assert_eq!(actual.length().to_bits(), expected.length);
    assert_eq!(actual.theta().to_bits(), expected.theta);
    assert_eq!(actual.rgb(), expected.rgb);
    assert_eq!(actual.branch(), expected.branch);
    assert_eq!(*actual.reactions(), expected.reactions);
}

#[test]
fn java_random_mixed_draws_match_curated_oracle() {
    let mut random = JavaRandom::new(0_i64);
    assert_eq!(random.next_i32(), -1155484576);
    assert_eq!(random.next_bounded(10000).unwrap(), 5948);
    assert_eq!(random.next_f64().to_bits(), 0x3fcec9e5b3672e14);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 5700976833288827063_i64);
    assert_eq!(random.next_i32(), 502539523);
    assert_eq!(random.next_bounded(10000).unwrap(), 8719);
    assert_eq!(random.next_f64().to_bits(), 0x3fe90254eaaa6e69);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 7105486291024734541_i64);
    assert_eq!(random.next_i32(), -65105105);
    assert_eq!(random.next_bounded(10000).unwrap(), 8844);
    assert_eq!(random.next_f64().to_bits(), 0x3fec2243602f4588);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 3246199166113899023_i64);
    assert_eq!(random.next_i32(), 1344049776);
    assert_eq!(random.next_bounded(10000).unwrap(), 4524);
    assert_eq!(random.next_f64().to_bits(), 0x3fd78cea32587af8);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 428667830982598836_i64);
    assert_eq!(random.next_i32(), -1946737912);
    assert_eq!(random.next_bounded(10000).unwrap(), 223);
    assert_eq!(random.next_f64().to_bits(), 0x3feedd138c80ebc3);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 4664870386390374308_i64);
    assert_eq!(random.next_i32(), 1227951724);
    assert_eq!(random.next_bounded(10000).unwrap(), 8125);
    assert_eq!(random.next_f64().to_bits(), 0x3f8e92178d78cdc0);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -171134583860878546_i64);
    assert_eq!(random.next_i32(), 2092649110);
    assert_eq!(random.next_bounded(10000).unwrap(), 5730);
    assert_eq!(random.next_f64().to_bits(), 0x3fe7e135a7d8f9c6);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -7931467246759943776_i64);
    assert_eq!(random.next_i32(), 55004124);
    assert_eq!(random.next_bounded(10000).unwrap(), 3319);
    assert_eq!(random.next_f64().to_bits(), 0x3fda61c6886d5c62);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -1856941488587171136_i64);
    let mut random = JavaRandom::new(1_i64);
    assert_eq!(random.next_i32(), -1155869325);
    assert_eq!(random.next_bounded(10000).unwrap(), 4588);
    assert_eq!(random.next_f64().to_bits(), 0x3fda3ec39684df98);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 668424937071768037_i64);
    assert_eq!(random.next_i32(), -1465154083);
    assert_eq!(random.next_bounded(10000).unwrap(), 9978);
    assert_eq!(random.next_f64().to_bits(), 0x3fe6be61080c872b);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -669528114487223426_i64);
    assert_eq!(random.next_i32(), -258276172);
    assert_eq!(random.next_bounded(10000).unwrap(), 1234);
    assert_eq!(random.next_f64().to_bits(), 0x3fee4f6bb749a067);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 8985176291945559937_i64);
    assert_eq!(random.next_i32(), -369526632);
    assert_eq!(random.next_bounded(10000).unwrap(), 9310);
    assert_eq!(random.next_f64().to_bits(), 0x3fc4653d0968ea4c);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -9103770306483490189_i64);
    assert_eq!(random.next_i32(), 498074875);
    assert_eq!(random.next_bounded(10000).unwrap(), 1302);
    assert_eq!(random.next_f64().to_bits(), 0x3fe8a83adcaab56c);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 780266760877150279_i64);
    assert_eq!(random.next_i32(), -1023599386);
    assert_eq!(random.next_bounded(10000).unwrap(), 2689);
    assert_eq!(random.next_f64().to_bits(), 0x3fe3f795411e3be5);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -5627216606837767319_i64);
    assert_eq!(random.next_i32(), -836540342);
    assert_eq!(random.next_bounded(10000).unwrap(), 737);
    assert_eq!(random.next_f64().to_bits(), 0x3f74954768b52880);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 431986847459427401_i64);
    assert_eq!(random.next_i32(), -1266972581);
    assert_eq!(random.next_bounded(10000).unwrap(), 1452);
    assert_eq!(random.next_f64().to_bits(), 0x3fd19a1737b528be);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -8401480976436315613_i64);
    let mut random = JavaRandom::new(42_i64);
    assert_eq!(random.next_i32(), -1170105035);
    assert_eq!(random.next_bounded(10000).unwrap(), 2763);
    assert_eq!(random.next_f64().to_bits(), 0x3fe5dcf778622e01);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -1068554554583376509_i64);
    assert_eq!(random.next_i32(), -1255373459);
    assert_eq!(random.next_bounded(10000).unwrap(), 5519);
    assert_eq!(random.next_f64().to_bits(), 0x3fb7610cb9d0d9e8);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 6802844026563419272_i64);
    assert_eq!(random.next_i32(), 1184328952);
    assert_eq!(random.next_bounded(10000).unwrap(), 8032);
    assert_eq!(random.next_f64().to_bits(), 0x3fddac800c318574);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -33599001222115087_i64);
    assert_eq!(random.next_i32(), 652655727);
    assert_eq!(random.next_bounded(10000).unwrap(), 7226);
    assert_eq!(random.next_f64().to_bits(), 0x3fdc25ae2bff9d9c);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 7130900098642117381_i64);
    assert_eq!(random.next_i32(), 761834774);
    assert_eq!(random.next_bounded(10000).unwrap(), 8246);
    assert_eq!(random.next_f64().to_bits(), 0x3fe304ea1ab4daca);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 4623885210859241458_i64);
    assert_eq!(random.next_i32(), 1568355455);
    assert_eq!(random.next_bounded(10000).unwrap(), 5212);
    assert_eq!(random.next_f64().to_bits(), 0x3fc44ecaf2cc3484);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -4588066762141701118_i64);
    assert_eq!(random.next_i32(), -1842367650);
    assert_eq!(random.next_bounded(10000).unwrap(), 2737);
    assert_eq!(random.next_f64().to_bits(), 0x3fe28f9058b9ae4f);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -3714860777752295781_i64);
    assert_eq!(random.next_i32(), 1359496886);
    assert_eq!(random.next_bounded(10000).unwrap(), 7193);
    assert_eq!(random.next_f64().to_bits(), 0x3fc97b815a2b647c);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 7704975138223047541_i64);
    let mut random = JavaRandom::new(-1_i64);
    assert_eq!(random.next_i32(), 1155099827);
    assert_eq!(random.next_bounded(10000).unwrap(), 2225);
    assert_eq!(random.next_f64().to_bits(), 0x3f8921011897ff00);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -7364023878800395366_i64);
    assert_eq!(random.next_i32(), 1714930956);
    assert_eq!(random.next_bounded(10000).unwrap(), 7765);
    assert_eq!(random.next_f64().to_bits(), 0x3facf271e72c3a80);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 1047579610836041353_i64);
    assert_eq!(random.next_i32(), 353780684);
    assert_eq!(random.next_bounded(10000).unwrap(), 2250);
    assert_eq!(random.next_f64().to_bits(), 0x3fe72242b7388ef3);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -4952432241328847918_i64);
    assert_eq!(random.next_i32(), 1508412514);
    assert_eq!(random.next_bounded(10000).unwrap(), 7263);
    assert_eq!(random.next_f64().to_bits(), 0x3fdd126ff9f288f2);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 4291381384867427929_i64);
    assert_eq!(random.next_i32(), 756071893);
    assert_eq!(random.next_bounded(10000).unwrap(), 8220);
    assert_eq!(random.next_f64().to_bits(), 0x3fe9f688258da2a7);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 1946119930767781179_i64);
    assert_eq!(random.next_i32(), -1303581130);
    assert_eq!(random.next_bounded(10000).unwrap(), 9375);
    assert_eq!(random.next_f64().to_bits(), 0x3fd674b56f1e480c);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 4038282759222968018_i64);
    assert_eq!(random.next_i32(), 341774279);
    assert_eq!(random.next_bounded(10000).unwrap(), 8592);
    assert_eq!(random.next_f64().to_bits(), 0x3fe7ecaee3752fa3);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -2272770725395940025_i64);
    assert_eq!(random.next_i32(), 1336914540);
    assert_eq!(random.next_bounded(10000).unwrap(), 8011);
    assert_eq!(random.next_f64().to_bits(), 0x3fc7b0f445ca6068);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -3755948936872483272_i64);
    let mut random = JavaRandom::new(-9223372036854775808_i64);
    assert_eq!(random.next_i32(), -1155484576);
    assert_eq!(random.next_bounded(10000).unwrap(), 5948);
    assert_eq!(random.next_f64().to_bits(), 0x3fcec9e5b3672e14);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 5700976833288827063_i64);
    assert_eq!(random.next_i32(), 502539523);
    assert_eq!(random.next_bounded(10000).unwrap(), 8719);
    assert_eq!(random.next_f64().to_bits(), 0x3fe90254eaaa6e69);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 7105486291024734541_i64);
    assert_eq!(random.next_i32(), -65105105);
    assert_eq!(random.next_bounded(10000).unwrap(), 8844);
    assert_eq!(random.next_f64().to_bits(), 0x3fec2243602f4588);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 3246199166113899023_i64);
    assert_eq!(random.next_i32(), 1344049776);
    assert_eq!(random.next_bounded(10000).unwrap(), 4524);
    assert_eq!(random.next_f64().to_bits(), 0x3fd78cea32587af8);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), 428667830982598836_i64);
    assert_eq!(random.next_i32(), -1946737912);
    assert_eq!(random.next_bounded(10000).unwrap(), 223);
    assert_eq!(random.next_f64().to_bits(), 0x3feedd138c80ebc3);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 4664870386390374308_i64);
    assert_eq!(random.next_i32(), 1227951724);
    assert_eq!(random.next_bounded(10000).unwrap(), 8125);
    assert_eq!(random.next_f64().to_bits(), 0x3f8e92178d78cdc0);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -171134583860878546_i64);
    assert_eq!(random.next_i32(), 2092649110);
    assert_eq!(random.next_bounded(10000).unwrap(), 5730);
    assert_eq!(random.next_f64().to_bits(), 0x3fe7e135a7d8f9c6);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -7931467246759943776_i64);
    assert_eq!(random.next_i32(), 55004124);
    assert_eq!(random.next_bounded(10000).unwrap(), 3319);
    assert_eq!(random.next_f64().to_bits(), 0x3fda61c6886d5c62);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -1856941488587171136_i64);
    let mut random = JavaRandom::new(9223372036854775807_i64);
    assert_eq!(random.next_i32(), 1155099827);
    assert_eq!(random.next_bounded(10000).unwrap(), 2225);
    assert_eq!(random.next_f64().to_bits(), 0x3f8921011897ff00);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -7364023878800395366_i64);
    assert_eq!(random.next_i32(), 1714930956);
    assert_eq!(random.next_bounded(10000).unwrap(), 7765);
    assert_eq!(random.next_f64().to_bits(), 0x3facf271e72c3a80);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 1047579610836041353_i64);
    assert_eq!(random.next_i32(), 353780684);
    assert_eq!(random.next_bounded(10000).unwrap(), 2250);
    assert_eq!(random.next_f64().to_bits(), 0x3fe72242b7388ef3);
    assert_eq!(random.next_bool(), true);
    assert_eq!(random.next_i64(), -4952432241328847918_i64);
    assert_eq!(random.next_i32(), 1508412514);
    assert_eq!(random.next_bounded(10000).unwrap(), 7263);
    assert_eq!(random.next_f64().to_bits(), 0x3fdd126ff9f288f2);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 4291381384867427929_i64);
    assert_eq!(random.next_i32(), 756071893);
    assert_eq!(random.next_bounded(10000).unwrap(), 8220);
    assert_eq!(random.next_f64().to_bits(), 0x3fe9f688258da2a7);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 1946119930767781179_i64);
    assert_eq!(random.next_i32(), -1303581130);
    assert_eq!(random.next_bounded(10000).unwrap(), 9375);
    assert_eq!(random.next_f64().to_bits(), 0x3fd674b56f1e480c);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), 4038282759222968018_i64);
    assert_eq!(random.next_i32(), 341774279);
    assert_eq!(random.next_bounded(10000).unwrap(), 8592);
    assert_eq!(random.next_f64().to_bits(), 0x3fe7ecaee3752fa3);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -2272770725395940025_i64);
    assert_eq!(random.next_i32(), 1336914540);
    assert_eq!(random.next_bounded(10000).unwrap(), 8011);
    assert_eq!(random.next_f64().to_bits(), 0x3fc7b0f445ca6068);
    assert_eq!(random.next_bool(), false);
    assert_eq!(random.next_i64(), -3755948936872483272_i64);
}

#[test]
fn random_genes_match_all_curated_fields_and_stream_position() {
    let mut random = JavaRandom::new(0);
    check_gene(
        &Gene::random(&mut random),
        ExpectedGene {
            length: 0x402b64168ea6ca89,
            theta: 0x3ff82e6d79bee8fb,
            rgb: 0xb0ff00,
            branch: 0,
            reactions: [
                5, 5, 3, 3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 2, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5,
                5, 2, 2, 3, 5, 4, 3, 2,
            ],
        },
    );
    assert_eq!(random.next_i64(), -5751058041584006506_i64);
    let mut random = JavaRandom::new(42);
    check_gene(
        &Gene::random(&mut random),
        ExpectedGene {
            length: 0x402b4833a06ff457,
            theta: 0x40112bd8ec088f6b,
            rgb: 0x00ff80,
            branch: 0,
            reactions: [
                1, 5, 2, 1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2,
                5, 5, 5, 0, 1, 1, 0, 5,
            ],
        },
    );
    assert_eq!(random.next_i64(), -6574010802050714786_i64);
}

#[test]
fn random_genomes_match_initial_curated_inheritance_states() {
    let actual = Genome::random(&mut JavaRandom::new(0));
    assert_eq!(actual.symmetry(), 7);
    assert_eq!(actual.mirrored(), true);
    assert_eq!(actual.mutation_rate(), 25);
    assert_eq!(actual.activity(), 1);
    assert_eq!(
        actual.modifiers(),
        GenomeModifiers {
            cream: 2,
            fallow: 4,
            spore: 7,
            adapt_spore: 50,
            plague: false,
            disperse_children: true,
            generation_battle: true,
            sibling_battle: true,
            altruist: true,
            familial: false,
            social: true,
            peaceful: false,
            passive: false,
            clockwise: true,
            mimic_all: false,
            modifies_pink: true,
            modifies_lilac: true,
            modifies_sky: true,
            selfish: false,
        }
    );
    assert_eq!(actual.genes().unwrap().len(), 1);
    check_gene(
        &actual.gene_ref(0).unwrap().borrow(),
        ExpectedGene {
            length: 0x4027672e15196e4f,
            theta: 0x3fff11b3ef2a52fc,
            rgb: 0x808080,
            branch: -1,
            reactions: [
                3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 2, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5, 2, 2,
                3, 5, 4, 3, 2, 1, 5, 2,
            ],
        },
    );
    let actual = Genome::random(&mut JavaRandom::new(42));
    assert_eq!(actual.symmetry(), 1);
    assert_eq!(actual.mirrored(), true);
    assert_eq!(actual.mutation_rate(), 54);
    assert_eq!(actual.activity(), 2);
    assert_eq!(
        actual.modifiers(),
        GenomeModifiers {
            cream: 2,
            fallow: 2,
            spore: 1,
            adapt_spore: 4,
            plague: false,
            disperse_children: false,
            generation_battle: true,
            sibling_battle: true,
            altruist: false,
            familial: false,
            social: true,
            peaceful: false,
            passive: false,
            clockwise: true,
            mimic_all: true,
            modifies_pink: true,
            modifies_lilac: true,
            modifies_sky: false,
            selfish: false,
        }
    );
    assert_eq!(actual.genes().unwrap().len(), 2);
    check_gene(
        &actual.gene_ref(0).unwrap().borrow(),
        ExpectedGene {
            length: 0x400622e009e1079c,
            theta: 0x4017ad48b364193c,
            rgb: 0xffff00,
            branch: -1,
            reactions: [
                1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5, 5, 5,
                0, 1, 1, 0, 5, 1, 5, 1,
            ],
        },
    );
    check_gene(
        &actual.gene_ref(1).unwrap().borrow(),
        ExpectedGene {
            length: 0x400b9ae4f8148fbc,
            theta: 0x40141249828f2086,
            rgb: 0xffc800,
            branch: -1,
            reactions: [
                1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5, 2, 4,
                5, 3, 0, 4, 5, 2, 1, 3,
            ],
        },
    );
}

// Runtime fixture data can use a separately verified workspace-shaped test packet
pub(crate) fn fixture_path(relative: &str) -> std::path::PathBuf {
    let root = std::env::var_os("BGR_TEST_FIXTURE_ROOT").map(std::path::PathBuf::from);
    fixture_path_under(root.as_deref(), relative)
}

fn fixture_path_under(override_root: Option<&std::path::Path>, relative: &str) -> std::path::PathBuf {
    use std::path::{Component, Path};
    let relative = Path::new(relative);
    assert!(!relative.as_os_str().is_empty()
        && relative.components().all(|part| matches!(part, Component::Normal(_))),
        "Fixture path must be workspace-relative without traversal");
    let compiled = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = match override_root {
        None => compiled.parent().and_then(Path::parent).expect("Core crate workspace root"),
        Some(root) => {
            assert!(root.is_absolute(), "BGR_TEST_FIXTURE_ROOT must be absolute");
            assert!(root.components().all(|part| matches!(part, Component::RootDir | Component::Normal(_))),
                "BGR_TEST_FIXTURE_ROOT must not contain traversal");
            for directory in [root.to_path_buf(), root.join("crates/core/tests/fixtures"),
                root.join("tests/fixtures/java/xml_entity_markup/cases")] {
                for ancestor in directory.ancestors() {
                    let metadata = std::fs::symlink_metadata(ancestor)
                        .expect("BGR_TEST_FIXTURE_ROOT must contain both fixture data roots");
                    assert!(metadata.is_dir() && !metadata.file_type().is_symlink(),
                        "BGR_TEST_FIXTURE_ROOT data roots must be ordinary directories");
                }
            }
            root
        }
    };
    root.join(relative)
}

struct FixturePathLayout(std::path::PathBuf);

impl FixturePathLayout {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("bgr-fixture-path-{}-{index}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn complete(&self) {
        for relative in ["crates/core/tests/fixtures", "tests/fixtures/java/xml_entity_markup/cases"] {
            std::fs::create_dir_all(self.0.join(relative)).unwrap();
        }
    }
}

impl Drop for FixturePathLayout {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("Remove owned fixture-path test layout");
    }
}

#[test]
fn fixture_paths_default_root_preserves_compiled_workspace_layout() {
    let actual = fixture_path_under(None, "crates/core/tests/fixtures/color-stream");
    assert_eq!(actual, std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/color-stream"));
    assert_eq!(fixture_path_under(None, "tests/fixtures/java/xml_entity_markup/cases"),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap()
            .join("tests/fixtures/java/xml_entity_markup/cases"));
}

#[test]
fn fixture_paths_relocated_data_reads_both_fixture_roots() {
    let layout = FixturePathLayout::new();
    layout.complete();
    for (relative, bytes) in [
        ("crates/core/tests/fixtures/relocated-core-marker.bin", b"core-data".as_slice()),
        ("tests/fixtures/java/xml_entity_markup/cases/relocated-workspace-marker.bin", b"workspace-data".as_slice()),
    ] {
        std::fs::write(layout.0.join(relative), bytes).unwrap();
        let relocated = fixture_path_under(Some(&layout.0), relative);
        assert_eq!(std::fs::read(&relocated).unwrap(), bytes);
        assert_ne!(relocated, fixture_path_under(None, relative));
    }
}

#[test]
fn fixture_paths_invalid_explicit_roots_never_fall_back() {
    use std::path::Path;
    let layout = FixturePathLayout::new();
    let missing = layout.0.join("missing");
    let file = layout.0.join("file");
    std::fs::write(&file, b"not a directory").unwrap();
    for root in [Path::new(""), Path::new("relative"), &missing, &file, &layout.0] {
        assert!(std::panic::catch_unwind(|| fixture_path_under(Some(root), "crates/core")).is_err());
    }
    layout.complete();
    for relative in ["", "/absolute", "../escape", "crates/../escape"] {
        assert!(std::panic::catch_unwind(|| fixture_path_under(Some(&layout.0), relative)).is_err());
    }
    #[cfg(unix)]
    {
        let link = layout.0.join("link");
        std::os::unix::fs::symlink(&layout.0, &link).unwrap();
        assert!(std::panic::catch_unwind(|| fixture_path_under(Some(&link), "crates/core")).is_err());
        let data = layout.0.join("tests/fixtures/java/xml_entity_markup/cases");
        std::fs::remove_dir(&data).unwrap();
        std::os::unix::fs::symlink(layout.0.join("crates/core/tests/fixtures"), &data).unwrap();
        assert!(std::panic::catch_unwind(|| fixture_path_under(Some(&layout.0), "crates/core")).is_err());
    }
}
