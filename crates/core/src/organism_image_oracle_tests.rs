// SPDX-License-Identifier: GPL-2.0-only

//! Direct original getImage replay; observed pixels and poststates never construct actors

use super::death_oracle_tests::{
    apply_reference_input, compare_genomes, statistics_schemas, validate, validate_genome,
    world_schema,
};
use super::maintenance_oracle_tests::world_views;
use super::*;
use crate::organism::{GenomeRef, OrganismRef};
use crate::statistics::tests::{assert_observation, observed_field, observed_getter};
use crate::world::organism_image::image_from_organism;

const ORIGINAL_TAGS: [(&str, usize); 20] = [
    ("C", 39),
    ("U", 5733),
    ("I", 453),
    ("G", 3276),
    ("N", 5640),
    ("A", 39),
    ("P", 78),
    ("O", 12012),
    ("H", 234),
    ("W", 780),
    ("S", 4056),
    ("T", 3822),
    ("Z", 78),
    ("R", 78),
    ("K", 39),
    ("B", 39),
    ("PIX", 2962),
    ("PNG", 39),
    ("F", 39),
    ("E", 39),
];

const ALPHA_TAGS: [(&str, usize); 20] = [
    ("C", 5),
    ("U", 735),
    ("I", 60),
    ("G", 420),
    ("N", 600),
    ("A", 5),
    ("P", 10),
    ("O", 1540),
    ("H", 30),
    ("W", 100),
    ("S", 520),
    ("T", 490),
    ("Z", 10),
    ("R", 10),
    ("K", 5),
    ("B", 5),
    ("PIX", 405),
    ("PNG", 5),
    ("F", 5),
    ("E", 5),
];

type Row<'a> = Vec<&'a str>;

fn input_kind(target: &str) -> &'static str {
    match target {
        "genome:g0:construction" => "String",
        "world:_O2" | "world:_CO2" | "world:_CH4" => "double",
        "actor:victim:bounds" => "Rectangle",
        "actor:victim:_centerX"
        | "actor:victim:_centerY"
        | "actor:victim:_framesColor"
        | "actor:victim:_growthRatio" => "int",
        "actor:victim:x1" | "actor:victim:y1" | "actor:victim:x2" | "actor:victim:y2" => "int[]",
        "actor:victim:_segColor" => "Color[]",
        "actor:victim:_haseyes" | "actor:victim:alive" => "boolean",
        "actor:victim:_color" => "Color",
        "actor:victim:_theta" | "actor:victim:_lastTheta" => "double",
        _ => panic!("Undeclared image input {target}"),
    }
}

fn genomes(rows: &[Row<'_>], stage: &str) -> BTreeMap<String, Stage> {
    let mut output = BTreeMap::<String, Stage>::new();
    for row in rows
        .iter()
        .filter(|row| ["G", "N"].contains(&row[0]) && row[1] == stage)
    {
        assert_eq!(row[2], "g0");
        let code = output.entry(row[2].into()).or_default();
        let inserted = if row[0] == "G" {
            assert_eq!(row.len(), 5);
            code.genome.insert(row[3].into(), row[4].into())
        } else {
            assert_eq!(row.len(), 6);
            code.genes
                .entry(row[3].parse().unwrap())
                .or_default()
                .insert(row[4].into(), row[5].into())
        };
        assert!(inserted.is_none(), "Duplicate image genome observation");
    }
    assert_eq!(output.len(), 1);
    let schema = gene_fields(&Gene::new(0.0, 0.0, None, 0, [0; 36]))
        .into_keys()
        .collect();
    validate_genome(&output["g0"], &schema);
    output
}

fn snapshot(
    rows: &[Row<'_>],
    stage: &str,
    name: &str,
    world: &World,
    actors: &BTreeMap<String, OrganismRef>,
    codes: &BTreeMap<String, GenomeRef>,
    errors: &mut Vec<String>,
) -> usize {
    let actor = actors["victim"].borrow();
    let (setting_schema, organism_schema) = primitive_reference_schemas();
    assert_eq!(setting_schema.len(), 147);
    let mut expected = Fields::new();
    let mut excluded = BTreeSet::new();
    for row in rows.iter().filter(|r| r[0] == "O" && r[1] == stage) {
        assert_eq!(row.len(), 6);
        assert_eq!(row[2], "victim");
        assert_eq!(organism_schema[row[3]], row[4]);
        if ["_world", "_visibleWorld"].contains(&row[3]) {
            assert!(excluded.insert(row[3]));
            assert_eq!(row[5], if row[3] == "_world" { "world" } else { "~" });
        } else {
            if row[4] == "GeneticCode" {
                assert!(["g0", "~"].contains(&row[5]));
            } else {
                validate(row[4], row[5]);
            }
            assert!(expected.insert(row[3].into(), row[5].into()).is_none());
        }
    }
    assert_eq!(excluded, BTreeSet::from(["_world", "_visibleWorld"]));
    assert_eq!(expected.len(), 152);
    let mut actual = organism_fields(&View::from(&*actor));
    actual.remove("_world").unwrap();
    actual.remove("_visibleWorld").unwrap();
    for (field, code) in [
        ("_geneticCode", actor.genome.as_ref()),
        ("_infectedGeneticCode", actor.infected_genome.as_ref()),
        ("_savedGeneticCode", actor.saved_genome.as_ref()),
    ] {
        let label = code.map_or("~", |code| {
            codes
                .iter()
                .find(|(_, supplied)| Rc::ptr_eq(code, supplied))
                .expect("Unregistered actual genotype")
                .0
                .as_str()
        });
        actual.insert(field.into(), label.into());
    }
    compare_fields(name, stage, "O", &actual, &expected, errors);
    let mut seen_holders = BTreeSet::new();
    for row in rows.iter().filter(|r| r[0] == "H" && r[1] == stage) {
        assert_eq!(row.len(), 5);
        assert_eq!(row[2], "victim");
        assert!(seen_holders.insert(row[3]));
        match row[3] {
            "genotype" => {
                assert_eq!(row[4], "g0");
                assert!(Rc::ptr_eq(actor.genome.as_ref().unwrap(),  &codes["g0"]));
            }
            "infected" => {
                assert_eq!(row[4], "~");
                assert!(actor.infected_genome.is_none());
            }
            "saved" => {
                assert_eq!(row[4], "~");
                assert!(actor.saved_genome.is_none());
            }
            _ => panic!("Unknown image holder"),
        }
    }
    assert_eq!(
        seen_holders,
        BTreeSet::from(["genotype", "infected", "saved"])
    );
    drop(actor);
    // This source route pastes without addOrganism; it must not alter membership or birth statistics
    assert!(world.organisms.borrow().is_empty());
    let (actual_world, _) = world_views(world, actors);
    let mut expected_world = Fields::new();
    let (stats_schema, getter_schema) = statistics_schemas();
    let mut seen_stats = BTreeSet::new();
    let mut seen_getters = BTreeSet::new();
    for row in rows
        .iter()
        .filter(|r| ["W", "S", "T"].contains(&r[0]) && r[1] == stage)
    {
        assert_eq!(row.len(), 5);
        match row[0] {
            "W" => {
                assert_eq!(world_schema()[row[2]], row[3]);
                validate(row[3], row[4]);
                assert!(
                    expected_world
                        .insert(row[2].into(), row[4].into())
                        .is_none()
                );
            }
            "S" => {
                assert_eq!(stats_schema[row[2]], row[3]);
                assert!(seen_stats.insert(row[2]));
                assert_observation(
                    observed_field(&world.statistics_ref().borrow(), row[2]),
                    row[3],
                    row[4],
                    codes,
                    name,
                );
            }
            "T" => {
                assert_eq!(getter_schema[row[2]], row[3]);
                assert!(seen_getters.insert(row[2]));
                assert_observation(
                    observed_getter(&world.statistics_ref().borrow(), row[2]),
                    row[3],
                    row[4],
                    codes,
                    name,
                );
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(
        (expected_world.len(), seen_stats.len(), seen_getters.len()),
        (10, 52, 49)
    );
    compare_fields(name, stage, "W", &actual_world, &expected_world, errors);
    let observed_codes = genomes(rows, stage);
    compare_genomes(name, stage, &observed_codes, codes, errors);
    let code_rows = observed_codes
        .values()
        .map(|code| 28 + code.genes.len() * 40)
        .sum::<usize>();
    let rng: Vec<_> = rows
        .iter()
        .filter(|r| r[0] == "R" && r[1] == stage)
        .collect();
    assert_eq!(rng.len(), 1);
    assert_eq!(rng[0].len(), 5);
    let mut suffix = world.random_snapshot();
    assert_eq!(
        suffix.next_i64(),
        rng[0][2].parse::<i64>().unwrap(),
        "{name} {stage} RNG long"
    );
    assert_eq!(
        suffix.bounded(10000),
        rng[0][3].parse::<i32>().unwrap(),
        "{name} {stage} RNG int"
    );
    assert_eq!(
        bits(suffix.next_f64()),
        rng[0][4],
        "{name} {stage} RNG double"
    );
    152 + 3 + 10 + 52 + 49 + code_rows + 1
}

fn replay(rows: &[Row<'_>], mode: &str) -> usize {
    assert_eq!(rows[0].len(), 4);
    assert_eq!(rows[0][2], mode);
    let name = rows[0][1];
    let seed: i64 = rows[0][3].parse().unwrap();
    let (setting_schema, _) = primitive_reference_schemas();
    let mut settings = Fields::new();
    for row in rows.iter().filter(|r| r[0] == "U") {
        assert_eq!(row.len(), 4);
        assert_eq!(setting_schema[row[1]], row[2]);
        validate(row[2], row[3]);
        assert!(settings.insert(row[1].into(), row[3].into()).is_none());
    }
    assert_eq!(
        settings.keys().collect::<BTreeSet<_>>(),
        setting_schema.keys().collect()
    );
    let input_codes = genomes(rows, "input");
    let mut world = empty_reference_world(&settings, seed);
    let mut actors = BTreeMap::<String, OrganismRef>::new();
    let mut codes = BTreeMap::<String, GenomeRef>::new();
    let mut errors = Vec::new();
    let mut result = None;
    let mut count = 0;
    let mut seen_snapshots = Vec::new();
    let mut pixels = 0;
    let mut image_dimensions = false;
    let mut image_png = false;
    let mut supplied_gene_refs = Vec::new();
    let mut construction = false;
    let mut source_serialization = Vec::new();
    for row in rows {
        match row[0] {
            "C" => {
                assert!(actors.is_empty() && seen_snapshots.is_empty());
            }
            "U" => {
                assert!(actors.is_empty() && seen_snapshots.is_empty());
            }
            "G" | "N" => {
                assert_eq!(row[1], seen_snapshots.last().copied().unwrap_or("input"));
            }
            "O" | "H" | "W" | "S" | "T" | "R" => {
                assert_eq!(row[1], *seen_snapshots.last().unwrap());
            }
            "I" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], "setup");
                assert_eq!(input_kind(row[2]), row[3]);
                assert!(seen_snapshots.is_empty());
                validate(row[3], row[4]);
                if row[2] == "genome:g0:construction" {
                    assert!(!construction);
                    construction = true;
                    assert_eq!(row[4], "explicit");
                }
                if ["actor:victim:_theta", "actor:victim:_lastTheta"].contains(&row[2]) {
                    assert_eq!(row[3], "double");
                    let value = decoded(row[4]);
                    let mut actor = actors["victim"].borrow_mut();
                    if row[2] == "actor:victim:_theta" {
                        actor.state.angle = value;
                    } else {
                        actor.state.last_angle = value;
                    }
                } else if row[4].is_empty() && ["int[]", "Color[]"].contains(&row[3]) {
                    assert!(row[2].starts_with("actor:victim:"));
                    let actor = actors["victim"].borrow();
                    assert!(
                        (actor.body.segment_count == 0) && actor.body.rotated_values().unwrap().is_empty()
                    );
                } else {
                    apply_reference_input(
                        name,
                        &input_codes,
                        row[2],
                        row[3],
                        row[4],
                        &mut world,
                        &actors,
                        &mut codes,
                        &mut errors,
                    );
                }
            }
            "A" => {
                assert_eq!(row.len(), 5);
                assert_eq!(&row[1..3], ["victim", "g0"]);
                let center = [row[3].parse().unwrap(), row[4].parse().unwrap()];
                let actor = world
                    .construct_pasted_shared(Rc::clone(&codes["g0"]), center)
                    .expect("Image actor paste returned typed error")
                    .expect("Image actor paste rejected");
                assert!(
                    actors
                        .insert("victim".into(), Rc::new(RefCell::new(actor)))
                        .is_none()
                );
                assert!(world.organisms.borrow().is_empty());
                supplied_gene_refs = codes["g0"].borrow().genes().unwrap().to_vec();
                for (index, gene) in supplied_gene_refs.iter().enumerate() {
                    for (other_index, other) in supplied_gene_refs.iter().enumerate() {
                        assert_eq!(Rc::ptr_eq(gene, other), index == other_index);
                    }
                }
            }
            "P" => {
                assert_eq!(row.len(), 2);
                assert_eq!(
                    row[1],
                    if seen_snapshots.is_empty() {
                        "before"
                    } else {
                        "after"
                    }
                );
                seen_snapshots.push(row[1]);
                count += snapshot(rows, row[1], name, &world, &actors, &codes, &mut errors);
            }
            "Z" => {
                assert_eq!(row.len(), 3);
                assert_eq!(row[1], *seen_snapshots.last().unwrap());
                assert_eq!(row[2].len(), 64);
                assert!(row[2].bytes().all(|b| b.is_ascii_hexdigit()));
                source_serialization.push(row[2]);
            }
            "K" => {
                assert_eq!(row.as_slice(), ["K", "image", "getImage", "victim", "~"]);
                assert_eq!(seen_snapshots, ["before"]);
                assert!(result.is_none());
                result = Some(image_from_organism(&actors["victim"].borrow(), 1_048_576).unwrap());
            }
            "B" => {
                assert_eq!(row.len(), 5);
                assert_eq!(row[1], "image");
                assert!(!image_dimensions);
                image_dimensions = true;
                let image = result.as_ref().unwrap();
                assert_eq!(image.width, row[2].parse::<u32>().unwrap());
                assert_eq!(image.height, row[3].parse::<u32>().unwrap());
                // Source TYPE_INT_RGB=1 maps to an opaque row-major RGB8 result, not a Java layout field
                assert_eq!(row[4], "1");
                assert_eq!(
                    image.rgb.len(),
                    image.width as usize * image.height as usize * 3
                );
                count += 1;
            }
            "PIX" => {
                assert_eq!(row.len(), 5);
                assert!(image_dimensions);
                let image = result.as_ref().unwrap();
                assert_eq!(row[1].parse::<usize>().unwrap(), pixels);
                assert_eq!(
                    row[2].parse::<usize>().unwrap(),
                    pixels % image.width as usize
                );
                assert_eq!(
                    row[3].parse::<usize>().unwrap(),
                    pixels / image.width as usize
                );
                let pixel = &image.rgb[pixels * 3..pixels * 3 + 3];
                assert_eq!(
                    format!("ff{:02x}{:02x}{:02x}", pixel[0], pixel[1], pixel[2]),
                    row[4],
                    "{name} pixel {pixels}"
                );
                pixels += 1;
                count += 1;
            }
            "PNG" => {
                assert_eq!(row.len(), 4);
                assert!(!image_png);
                image_png = true;
                assert_eq!(row[1], format!("{name}.png"));
                assert_eq!(row[2].len(), 64);
                assert!(row[2].bytes().all(|b| b.is_ascii_hexdigit()));
                assert!(row[3].parse::<usize>().unwrap() > 0);
            }
            "F" => {
                assert_eq!(row.as_slice(), ["F", "unchanged", "true"]);
            }
            "E" => {
                assert_eq!(row.len(), 1);
            }
            _ => panic!("Unknown image row {}", row[0]),
        }
    }
    let mut case_tags = BTreeMap::<&str, usize>::new();
    for row in rows {
        *case_tags.entry(row[0]).or_default() += 1;
    }
    for (tag, expected) in [
        ("C", 1),
        ("U", 147),
        ("A", 1),
        ("P", 2),
        ("O", 308),
        ("H", 6),
        ("W", 20),
        ("S", 104),
        ("T", 98),
        ("Z", 2),
        ("R", 2),
        ("K", 1),
        ("B", 1),
        ("PNG", 1),
        ("F", 1),
        ("E", 1),
        ("G", 84),
    ] {
        assert_eq!(case_tags[tag], expected, "{name} {tag} rows");
    }
    assert_eq!(
        case_tags.get("N").copied().unwrap_or(0),
        input_codes["g0"].genes.len() * 120
    );
    assert!(construction && image_dimensions && image_png);
    assert_eq!(seen_snapshots, ["before", "after"]);
    assert_eq!(source_serialization.len(), 2);
    assert_eq!(source_serialization[0], source_serialization[1]);
    let image = result.unwrap();
    assert_eq!(pixels, image.width as usize * image.height as usize);
    let final_code = codes["g0"].borrow();
    assert_eq!(final_code.genes().unwrap().len(), supplied_gene_refs.len());
    assert!(
        final_code
            .genes().unwrap()
            .iter()
            .zip(&supplied_gene_refs)
            .all(|(current, supplied)| Rc::ptr_eq(current, supplied))
    );
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    count
}

fn compare_fixture(
    source: &str,
    mode: &str,
    expected_tags: [(&str, usize); 20],
    expected_derived: usize,
    expected_rows: usize,
) {
    let mut tags = BTreeMap::<&str, usize>::new();
    let mut rows = Vec::new();
    let mut names = BTreeSet::new();
    let mut total = 0;
    for line in source.lines() {
        assert!(!line.is_empty());
        let row: Row<'_> = line.split('\t').collect();
        *tags.entry(row[0]).or_default() += 1;
        if row[0] == "C" {
            assert!(rows.is_empty());
            assert!(names.insert(row[1]));
        } else {
            assert!(!rows.is_empty());
        }
        let end = row[0] == "E";
        rows.push(row);
        if end {
            total += replay(&rows, mode);
            rows.clear();
        }
    }
    assert!(rows.is_empty());
    let expected: BTreeMap<_, _> = expected_tags
        .into_iter()
        .filter(|(_, count)| *count != 0)
        .collect();
    assert_eq!(tags, expected);
    assert_eq!(
        names.len(),
        expected_tags.iter().find(|(tag, _)| *tag == "C").unwrap().1
    );
    assert_eq!(total, expected_derived);
    assert_eq!(source.lines().count(), expected_rows);
    eprintln!(
        "IMAGE_ORACLE mode={mode} cases={} derived={total} raw={expected_rows} diagnostics_unpassed=8360",
        names.len()
    );
}

#[test]
fn original_cropped_organism_images_and_complete_purity_observations() {
    compare_fixture(
        include_str!("../tests/fixtures/organism_image.tsv"),
        "organism-image",
        ORIGINAL_TAGS,
        29_771,
        39_475,
    );
}

#[test]
fn original_translucent_shape_raster_and_complete_purity_observations() {
    compare_fixture(
        include_str!("../tests/fixtures/organism_image_alpha.tsv"),
        "organism-image-alpha",
        ALPHA_TAGS,
        3_760,
        4_965,
    );
}
