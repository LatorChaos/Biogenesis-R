use super::*;
use crate::{Gene, GeneRef, GenomeProperties};
use std::io::Write;

fn handle(genes: &[GeneRef]) -> GenomeHandle {
    GenomeHandle::from_owned(Genome::from_parts(
        genes,
        GenomeProperties {
            symmetry: 1,
            ..GenomeProperties::default()
        },
    ))
}

#[test]
fn preview_budget_and_geometry_errors_precede_allocation() {
    let source = handle(&[Gene::new(12.0, 0.0, Some(0xff00ff00), 0, [0; 36]).shared()]);
    let required = 30000 + std::mem::size_of::<RotatedSegment>() + std::mem::size_of::<u32>();
    assert_eq!(
        source.preview_image(100, 100, required - 1),
        Err(PreviewImageError::Image(OrganismImageError::BudgetExceeded {
            required,
            budget: required - 1,
        }))
    );
    assert_eq!(source.preview_image(100, 100, required).unwrap().rgb.len(), 30000);
    let empty = handle(&[]).preview_image(100, 100, 30000).unwrap();
    assert_eq!((empty.width, empty.height), (100, 100));
    assert!(empty.rgb.iter().all(|n| *n == 0));
    assert_eq!(
        source.preview_image(0, 100, usize::MAX),
        Err(PreviewImageError::Geometry(GeometryError::InvalidViewport))
    );
    assert_eq!(
        source.preview_image(i32::MAX, i32::MAX, usize::MAX),
        Err(PreviewImageError::Image(OrganismImageError::SizeOverflow))
    );
    let invalid = handle(&[Gene::new(12.0, 0.0, None, 0, [0; 36]).shared()]);
    assert_eq!(
        invalid.preview_image(100, 100, 0),
        Err(PreviewImageError::Geometry(GeometryError::MissingColor { gene: 0 }))
    );
}

#[test]
fn preview_reads_live_shared_genes_without_mutating_world_or_rng() {
    let gene = Gene::new(12.0, 0.0, Some(0xff00ff00), 0, [0; 36]).shared();
    let source = handle(&[Rc::clone(&gene)]);
    let retained = source.clone();
    let world = World::new(WorldConfig {
        seed: 827364,
        initial_attempts: 0,
        ..WorldConfig::default()
    }).unwrap();
    let actor = world.paste_genome_handle(&source, [100, 100]).unwrap().unwrap();
    let before = world.clone();
    let first = source.preview_image(100, 100, usize::MAX).unwrap();
    let frozen = first.rgb.clone();
    assert_eq!(world, before);
    assert_eq!(world.copy_genome(&actor), Some(Some(retained.clone())));
    gene.borrow_mut().set_length(24.0);
    let changed = world.clone();
    let second = retained.preview_image(100, 100, usize::MAX).unwrap();
    assert_ne!(first.rgb, second.rgb);
    assert_eq!(first.rgb, frozen);
    assert_eq!(world, changed);
    assert_eq!(world.copy_genome(&actor), Some(Some(source)));
}

#[test]
fn preview_composites_overlapping_genes_in_forward_order() {
    let source = handle(&[
        Gene::new(12.0, 0.0, Some(0xffff0000), 0, [0; 36]).shared(),
        Gene::new(12.0, 0.0, Some(0x8000ff00), 0, [0; 36]).shared(),
    ]);
    let image = source.preview_image(100, 100, usize::MAX).unwrap();
    let center = (50 * 100 + 50) * 3;
    assert_eq!(&image.rgb[center..center + 3], &[127, 128, 0]);
}

#[test]
fn opaque_identity_scale_wraps_integer_translation_before_clipping() {
    let preview = GenomePreview {
        segments: vec![RotatedSegment { start: [i32::MAX, 0], end: [0, 0] }],
        colors: vec![0xff00ff00],
        translation: [1, 0],
        scale: 1.0,
    };
    let image = render(&preview, 3, 1, usize::MAX).unwrap();
    assert_eq!(image.rgb.as_slice(), &[0, 255, 0, 0, 255, 0, 0, 0, 0]);
}

#[test]
#[ignore = "Required external Java primitive RGB comparison with owned output directory"]
fn export_fixed_preview_primitive_framebuffers() {
    let output = std::path::PathBuf::from(
        std::env::var_os("BGR_PREVIEW_PRIMITIVE_OUTPUT").expect("Owned output directory required"),
    );
    assert!(output.is_absolute());
    for ancestor in output.ancestors() {
        if let Ok(metadata) = std::fs::symlink_metadata(ancestor) {
            assert!(!metadata.file_type().is_symlink());
        }
    }
    std::fs::create_dir(&output).unwrap();
    let mut observations = String::new();
    let rows = include_str!("../tests/fixtures/preview_raster_inputs.tsv");
    let mut names = std::collections::BTreeSet::new();
    for line in rows.lines() {
        let row: Vec<&str> = line.split('\t').collect();
        assert_eq!(row.len(), 8);
        assert!(row[0].bytes().all(|n| n.is_ascii_lowercase() || n.is_ascii_digit() || n == b'-'));
        assert!(names.insert(row[0]));
        let number = |index: usize| row[index].parse::<i32>().unwrap();
        let numerator = number(5);
        let denominator = number(6);
        assert!(numerator > 0 && numerator <= denominator && denominator <= 16);
        let preview = GenomePreview {
            segments: vec![RotatedSegment {
                start: [number(1), number(2)],
                end: [number(3), number(4)],
            }],
            colors: vec![u32::from_str_radix(row[7], 16).unwrap()],
            translation: [50, 50],
            scale: f64::from(numerator) / f64::from(denominator),
        };
        let image = render(&preview, 100, 100, 64 * 1024 * 1024).unwrap();
        assert_eq!((image.width, image.height, image.rgb.len()), (100, 100, 30000));
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true)
            .open(output.join(format!("{}.rgb", row[0]))).unwrap();
        file.write_all(&image.rgb).unwrap();
        observations.push_str(&format!("{}\t100\t100\t30000\n", row[0]));
    }
    assert_eq!(names.len(), 180);
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true)
        .open(output.join("observations.tsv")).unwrap();
    file.write_all(observations.as_bytes()).unwrap();
    println!("PREVIEW_PRIMITIVES PASS cases=180 images=180 bytes_per_image=30000");
}
