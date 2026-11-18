// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{ColorKind, Gene, Genome, JavaRandom, source_palette_color};

#[test]
fn distinct_equal_colors_and_null_keep_separate_model_meanings() {
    let first = JavaColor::from_argb(0x7f12_3456);
    let equal = JavaColor::from_argb(first.argb());
    assert_eq!(first, equal);
    assert!(!first.same_instance(&equal));
    assert_eq!(first.alpha_bits(), 0);
    assert!(first.rgb_components().is_none());
    assert!(first.components().is_none());
    assert!(first.color_space().is_none());
    let mut gene = Gene::default();
    assert!(gene.color_handle().is_none());
    gene.set_color(Some(0));
    assert_eq!(gene.color(), Some(0));
    assert!(gene.color_handle().is_some());
    assert_eq!(ColorKind::from_argb(gene.color().unwrap()), ColorKind::NoColor);
    gene.set_color(None);
    assert!(gene.color_handle().is_none());
}

#[test]
fn immutable_caches_keep_raw_bits_aliases_and_distinct_empty_owners() {
    let bits = vec![0x8000_0000, 0x7f80_0000, 0xffa0_0001, 0x7fc0_1234];
    let cache = JavaFloatArray::from_bits(bits.clone());
    let color = JavaColor::from_parts(ColorData {
        argb: 0x7f12_3456,
        alpha_bits: 0xffa0_0001,
        space: None,
        rgb: Some(cache.clone()),
        components: Some(cache.clone()),
    });
    let mut gene = Gene::new_with_color(4.0, 0.0, Some(color.clone()), -1, [0; 36]);
    let copied = gene.clone();
    assert!(copied.color_handle().unwrap().same_instance(&color));
    gene.set_color(Some(color.argb()));
    assert!(!gene.color_handle().unwrap().same_instance(&color));
    assert_eq!(gene, copied);
    drop(gene);
    drop(color);
    drop(cache);
    let retained = copied.color_handle().unwrap();
    assert_eq!(retained.alpha_bits(), 0xffa0_0001);
    assert_eq!(retained.rgb_components().unwrap().bits(), bits);
    assert!(retained.rgb_components().unwrap().same_instance(retained.components().unwrap()));
    let empty = JavaFloatArray::from_bits(Vec::new());
    let distinct = JavaFloatArray::from_bits(Vec::new());
    assert!(!empty.same_instance(&distinct));
    assert!(empty.same_instance(&empty.clone()));
}

#[test]
fn palette_randomization_and_xml_bind_fixed_holders_without_merging_fresh_values() {
    let palette = source_palette_color(0).unwrap();
    let fresh = JavaColor::from_argb(palette.argb());
    assert_eq!(palette, fresh);
    assert!(!palette.same_instance(&fresh));
    let mut random = JavaRandom::new(823);
    let mut expected = random.clone();
    expected.bounded(1);
    let mut weights = [0; 46];
    weights[1] = 1;
    let mut gene = Gene::default();
    gene.randomize_color(&mut random, &weights).unwrap();
    assert_eq!(random, expected);
    assert!(gene.color_handle().unwrap().same_instance(&palette));
    let genome = Genome::random(&mut JavaRandom::new(52));
    genome.gene_ref(0).unwrap().borrow_mut().set_color_handle(Some(palette.clone()));
    let bytes = crate::encode_genome_xml(&genome).unwrap();
    let parsed = crate::parse_genome_xml(&bytes).unwrap();
    assert!(parsed.gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&palette));
    assert!(!parsed.gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&fresh));
    assert!(source_palette_color(64).is_none());
}

#[test]
fn named_profiles_are_fixed_while_equal_spaces_remain_distinct() {
    let profile = JavaIccProfile::builtin(NamedIccProfile::Srgb);
    assert!(profile.same_instance(&JavaIccProfile::builtin(NamedIccProfile::Srgb)));
    assert!(!profile.same_instance(&JavaIccProfile::builtin(NamedIccProfile::LinearRgb)));
    let cache = JavaFloatArray::from_bits(vec![0x8000_0000]);
    let data = IccSpaceData {
        num_components: i32::MIN,
        space_type: -17,
        need_scale_init: false,
        diff_min_max: Some(cache.clone()),
        inv_diff_min_max: Some(cache),
        max_val: None,
        min_val: None,
        profile: Some(profile),
    };
    let first = JavaIccColorSpace::from_parts(data.clone());
    let distinct = JavaIccColorSpace::from_parts(data);
    assert!(!first.same_instance(&distinct));
    assert!(first.profile().unwrap().same_instance(distinct.profile().unwrap()));
    assert!(first.diff_min_max().unwrap().same_instance(first.inv_diff_min_max().unwrap()));
    assert_eq!(first.num_components(), i32::MIN);
    assert_eq!(first.space_type(), -17);
    assert!(!first.need_scale_init());
}
