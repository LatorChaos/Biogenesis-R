use biogenesis_core::{
    Body, Gene, Genome, GenomeModifiers, GenomePreview, GenomeProperties, GeometryError,
    RotatedSegment,
};
use std::collections::BTreeMap;

const SOURCE: &str = include_str!("fixtures/preview_branches.tsv");

#[derive(Default)]
struct Case {
    name: String,
    symmetry: i32,
    mirror: i32,
    width: i32,
    height: i32,
    count: usize,
    genes: Vec<biogenesis_core::GeneRef>,
    results: BTreeMap<String, bool>,
    exceptions: BTreeMap<String, String>,
    transform: Option<[u64; 4]>,
    lines: Vec<(RotatedSegment, u32)>,
    random: BTreeMap<String, Vec<String>>,
    world: BTreeMap<String, Vec<String>>,
    purity: bool,
}

fn bits(value: &str) -> u64 {
    u64::from_str_radix(value, 16).unwrap()
}

fn cases() -> Vec<Case> {
    let mut result = Vec::new();
    let mut current = None;
    let mut tags = BTreeMap::<String, usize>::new();
    for line in SOURCE.lines() {
        let row: Vec<_> = line.split('\t').collect();
        *tags.entry(row[0].to_owned()).or_default() += 1;
        if row[0] == "C" {
            assert_eq!(row.len(), 7);
            assert!(current.is_none());
            current = Some(Case {
                name: row[1].to_owned(),
                symmetry: row[2].parse().unwrap(),
                mirror: row[3].parse().unwrap(),
                width: row[4].parse().unwrap(),
                height: row[5].parse().unwrap(),
                count: row[6].parse().unwrap(),
                ..Case::default()
            });
            continue;
        }
        let case = current.as_mut().expect("Row outside a case");
        match row[0] {
            "G" => {
                assert_eq!(row.len(), 5);
                case.genes.push(
                    Gene::new(
                        f64::from_bits(bits(row[1])),
                        f64::from_bits(bits(row[2])),
                        Some(u32::from_str_radix(row[3], 16).unwrap()),
                        row[4].parse().unwrap(),
                        [0; 36],
                    )
                    .shared(),
                );
            }
            "B" => {
                assert_eq!(row.len(), 3);
                assert!(["preview", "paste"].contains(&row[1]));
                assert!(
                    case.results
                        .insert(row[1].to_owned(), row[2].parse().unwrap())
                        .is_none()
                );
            }
            "X" => {
                assert_eq!(row.len(), 3);
                assert!(["preview", "paste"].contains(&row[1]));
                assert_eq!(row[2], "java.lang.ArrayIndexOutOfBoundsException");
                assert!(
                    case.exceptions
                        .insert(row[1].to_owned(), row[2].to_owned())
                        .is_none()
                );
            }
            "T" => {
                assert_eq!(row.len(), 5);
                assert!(
                    case.transform
                        .replace([bits(row[1]), bits(row[2]), bits(row[3]), bits(row[4])])
                        .is_none()
                );
            }
            "L" => {
                assert_eq!(row.len(), 6);
                case.lines.push((
                    RotatedSegment {
                        start: [row[1].parse().unwrap(), row[2].parse().unwrap()],
                        end: [row[3].parse().unwrap(), row[4].parse().unwrap()],
                    },
                    u32::from_str_radix(row[5], 16).unwrap(),
                ));
            }
            "A" => {
                assert_eq!(row, ["A", "preview_genotype_unchanged", "true"]);
                assert!(!case.purity);
                case.purity = true;
            }
            "R" => {
                assert_eq!(row.len(), 5);
                assert!(["before-preview", "after-preview", "after-paste"].contains(&row[1]));
                row[2].parse::<i64>().unwrap();
                assert!((0..10000).contains(&row[3].parse::<i32>().unwrap()));
                bits(row[4]);
                assert!(
                    case.random
                        .insert(
                            row[1].to_owned(),
                            row[2..].iter().map(|v| v.to_string()).collect()
                        )
                        .is_none()
                );
            }
            "W" => {
                assert_eq!(row.len(), 8);
                assert!(["before-paste", "after-paste"].contains(&row[1]));
                for value in &row[2..5] {
                    bits(value);
                }
                row[5].parse::<i32>().unwrap();
                row[6].parse::<i32>().unwrap();
                assert!(
                    case.world
                        .insert(
                            row[1].to_owned(),
                            row[2..].iter().map(|v| v.to_string()).collect()
                        )
                        .is_none()
                );
            }
            "E" => {
                assert_eq!(row.len(), 1);
                assert_eq!(case.genes.len(), case.count);
                assert!(case.purity && case.transform.is_some());
                assert_eq!(case.random.len(), 3);
                assert_eq!(case.world.len(), 2);
                // These are checked source-only purity and placement observations
                assert_eq!(case.random["before-preview"], case.random["after-preview"]);
                assert_eq!(
                    case.world["before-paste"],
                    [
                        "0000000000000000",
                        "0000000000000000",
                        "0000000000000000",
                        "0",
                        "0",
                        "~",
                    ]
                );
                assert_eq!(case.world["after-paste"][4], "1");
                if case.results["preview"] {
                    assert!(!case.exceptions.contains_key("preview"));
                    assert!(case.results.contains_key("paste"));
                    assert_eq!(case.lines.len(), case.count * case.symmetry as usize);
                } else {
                    assert_eq!(case.exceptions.len(), 2);
                    assert!(case.lines.is_empty());
                    assert!(!case.results.contains_key("paste"));
                }
                result.push(current.take().unwrap());
            }
            tag => panic!("Unknown source tag {tag}"),
        }
    }
    assert!(current.is_none());
    let expected = [
        ("C", 31),
        ("G", 218),
        ("R", 93),
        ("B", 58),
        ("T", 31),
        ("L", 1314),
        ("A", 31),
        ("W", 62),
        ("E", 31),
        ("X", 8),
    ]
    .into_iter()
    .map(|(tag, count)| (tag.to_owned(), count))
    .collect();
    assert_eq!(tags, expected);
    assert_eq!(tags.values().sum::<usize>(), 1877);
    assert_eq!(result.len(), 31);
    assert_eq!(result.iter().filter(|c| c.results["preview"]).count(), 27);
    assert_eq!(
        result
            .iter()
            .filter(|c| c.results.get("paste") == Some(&true))
            .count(),
        4
    );
    assert_eq!(
        result
            .iter()
            .filter(|c| c.results.get("paste") == Some(&false))
            .count(),
        23
    );
    result
}

fn genome(case: &Case) -> Genome {
    Genome::from_parts(
        &case.genes,
        GenomeProperties {
            symmetry: case.symmetry,
            mirror: case.mirror,
            mutation_rate: 0,
            activity: 0,
            modifiers: GenomeModifiers {
                cream: 1,
                fallow: 1,
                spore: 1,
                adapt_spore: 1,
                ..GenomeModifiers::default()
            },
        },
    )
}

#[test]
fn original_preview_branch_calls_match_exact_lines_and_transforms() {
    let mut lines = 0;
    let mut successful = 0;
    let mut typed_errors = 0;
    for case in cases() {
        // Only original C/G inputs construct this genotype
        let genome = genome(&case);
        let before = genome.clone();
        let actual = GenomePreview::new(&genome, case.width, case.height);
        assert_eq!(genome, before, "{} preview mutates genotype", case.name);
        if !case.results["preview"] {
            assert_eq!(
                actual,
                Err(GeometryError::InvalidBranch { gene: 1 }),
                "{}",
                case.name
            );
            typed_errors += 1;
            continue;
        }
        let preview = actual.unwrap_or_else(|error| panic!("{}: {error}", case.name));
        assert_eq!(preview.segments.len(), case.lines.len(), "{}", case.name);
        assert_eq!(preview.colors.len(), case.lines.len(), "{}", case.name);
        for (index, (segment, argb)) in case.lines.iter().enumerate() {
            assert_eq!(
                &preview.segments[index], segment,
                "{} segment {index}",
                case.name
            );
            assert_eq!(preview.colors[index], *argb, "{} ARGB {index}", case.name);
            lines += 1;
        }
        let transform = [
            f64::from(preview.translation[0]).to_bits(),
            f64::from(preview.translation[1]).to_bits(),
            preview.scale.to_bits(),
            preview.scale.to_bits(),
        ];
        assert_eq!(Some(transform), case.transform, "{} transform", case.name);
        successful += 1;
    }
    assert_eq!((successful, typed_errors, lines), (27, 4, 1314));
}

#[test]
fn preview_acceptance_keeps_body_predecessor_restrictions() {
    let mut rejected = 0;
    for case in cases() {
        let genome = genome(&case);
        let invalid = case.genes.iter().enumerate().find_map(|(index, gene)| {
            let branch = gene.borrow().branch();
            (index > 0 && (branch < -1 || i64::from(branch) > index as i64)).then_some(index)
        });
        let body = Body::at_growth(&genome, 16);
        if let Some(index) = invalid {
            assert_eq!(
                body,
                Err(GeometryError::InvalidBranch { gene: index }),
                "{}",
                case.name
            );
            rejected += 1;
        } else {
            assert!(body.is_ok(), "{}: {body:?}", case.name);
        }
    }
    assert_eq!(rejected, 25);
}

#[test]
fn preview_preserves_scalar_color_and_viewport_validation() {
    let shared = Gene::new(2.5, 0.1, Some(0xff008000), 0, [0; 36]).shared();
    let genome = Genome::from_parts(
        &[shared.clone()],
        GenomeProperties {
            symmetry: 1,
            ..GenomeProperties::default()
        },
    );
    shared.borrow_mut().set_color(None);
    assert_eq!(
        GenomePreview::new(&genome, 30, 30),
        Err(GeometryError::MissingColor { gene: 0 })
    );
    shared.borrow_mut().set_color(Some(0x80203040));
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        shared.borrow_mut().set_length(value);
        assert_eq!(
            GenomePreview::new(&genome, 30, 30),
            Err(GeometryError::NonfiniteGene { gene: 0 })
        );
        shared.borrow_mut().set_length(2.5);
        shared.borrow_mut().set_theta(value);
        assert_eq!(
            GenomePreview::new(&genome, 30, 30),
            Err(GeometryError::NonfiniteGene { gene: 0 })
        );
    }
    shared.borrow_mut().set_theta(0.1);
    assert_eq!(
        GenomePreview::new(&genome, 0, 30),
        Err(GeometryError::InvalidViewport)
    );
    assert_eq!(
        GenomePreview::new(&genome, 30, -1),
        Err(GeometryError::InvalidViewport)
    );
    let invalid = Genome::from_parts(
        &[shared],
        GenomeProperties {
            symmetry: 9,
            ..GenomeProperties::default()
        },
    );
    assert_eq!(
        GenomePreview::new(&invalid, 30, 30),
        Err(GeometryError::InvalidSymmetry)
    );
}
