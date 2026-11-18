use crate::genetics::{Gene, GenomeProperties};

fn decoded(value: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(value, 16).unwrap())
}

fn close(actual: f64, expected: f64, absolute: f64, relative: f64) {
    if expected.is_nan() {
        assert!(actual.is_nan());
    } else if expected.is_infinite() {
        assert_eq!(actual, expected);
    } else {
        assert!(
            (actual - expected).abs() <= absolute + relative * expected.abs(),
            "actual={actual:?}, expected={expected:?}"
        );
    }
}

#[test]
fn production_java_geometry_matches_all_recorded_states() {
    let mut rows = include_str!("../fixtures/geometry.tsv").lines();
    let mut cases = 0;
    let mut polar = 0;
    let mut segment_count = 0;
    let mut math_cases = 0;
    let mut rounding = 0;
    while let Some(line) = rows.next() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields[0] == "A" {
            let y = decoded(fields[1]);
            let x = decoded(fields[2]);
            for (actual, expected) in [
                (java_math::atan(y), decoded(fields[3])),
                (java_math::atan2(y, x), decoded(fields[4])),
            ] {
                if expected.is_nan() {
                    assert!(actual.is_nan());
                } else {
                    assert_eq!(
                        actual.to_bits(),
                        expected.to_bits(),
                        "atan/atan2 case {math_cases}"
                    );
                }
            }
            math_cases += 1;
            continue;
        }
        if fields[0] == "Q" {
            assert_eq!(
                java_round(decoded(fields[1])),
                fields[2].parse::<i32>().unwrap()
            );
            rounding += 1;
            continue;
        }
        if fields[0] == "V" || fields[0] == "U" {
            let mut vector = PolarVector {
                modulus: decoded(fields[1]),
                theta: decoded(fields[2]),
            };
            if fields[0] == "V" {
                vector.invert_x();
            } else {
                vector.invert_y();
            }
            vector.add_angle(0.375);
            if fields[0] == "V" {
                vector.invert_y();
            } else {
                vector.invert_x();
            }
            close(vector.modulus, decoded(fields[3]), 1e-12, 1e-12);
            close(vector.theta, decoded(fields[4]), 1e-12, 1e-12);
            close(vector.cartesian()[0], decoded(fields[5]), 1e-12, 1e-12);
            close(vector.cartesian()[1], decoded(fields[6]), 1e-12, 1e-12);
            polar += 1;
            continue;
        }
        assert_eq!(fields[0], "C");
        let symmetry: i32 = fields[2].parse().unwrap();
        let mirror: i32 = fields[3].parse().unwrap();
        let growth: i32 = fields[4].parse().unwrap();
        let flags: i32 = fields[7].parse().unwrap();
        let eyes = EyeState {
            enabled: fields[5] == "1",
            good_vision: fields[6] == "1",
            consumer: flags & 1 != 0,
            fungus: flags & 2 != 0,
            infectious: flags & 4 != 0,
        };
        let update: i32 = fields[8].parse().unwrap();
        let angle = decoded(fields[9]);
        let center = [decoded(fields[10]), decoded(fields[11])];
        let viewport = [fields[12].parse().unwrap(), fields[13].parse().unwrap()];
        let count: usize = fields[14].parse().unwrap();
        let mut genes = Vec::new();
        for _ in 0..count {
            let row: Vec<_> = rows.next().unwrap().split('\t').collect();
            assert_eq!(row[0], "G");
            let reactions: Vec<i32> = row[5..]
                .iter()
                .map(|number| number.parse().unwrap())
                .collect();
            genes.push(
                Gene::new(
                    decoded(row[1]),
                    decoded(row[2]),
                    Some(row[3].parse().unwrap()),
                    row[4].parse().unwrap(),
                    reactions.try_into().unwrap(),
                )
                .shared(),
            );
        }
        let genome = Genome::from_parts(
            &genes,
            GenomeProperties {
                symmetry,
                mirror,
                ..GenomeProperties::default()
            },
        );
        let row: Vec<_> = rows.next().unwrap().split('\t').collect();
        assert_eq!(row[0], "F");
        let colors: Vec<u32> = row[1..]
            .iter()
            .map(|color| color.parse().unwrap())
            .collect();
        let body = Body::from_phenotype(&genome, growth, &colors, eyes, update).unwrap();
        let row: Vec<_> = rows.next().unwrap().split('\t').collect();
        assert_eq!(row[0], "B");
        assert_eq!(
            body.size().unwrap(),
            IntRect {
                x: row[1].parse().unwrap(),
                y: row[2].parse().unwrap(),
                width: row[3].parse().unwrap(),
                height: row[4].parse().unwrap()
            }
        );
        assert_eq!(
            (body.size().unwrap().width, body.size().unwrap().height),
            (row[3].parse::<i32>().unwrap(), row[4].parse::<i32>().unwrap())
        );
        close(body.mass, decoded(row[5]), 1e-12, 1e-12);
        close(body.inertia, decoded(row[6]), 1e-9, 1e-12);
        assert_eq!(body.eyes.enabled, row[7] == "1");
        assert_eq!(body.eyes.good_vision, row[8] == "1");
        assert_eq!(body.update_effects, row[9].parse::<i32>().unwrap());
        let rotated = body.rotated(angle, center).unwrap();
        let row: Vec<_> = rows.next().unwrap().split('\t').collect();
        assert_eq!(row[0], "W");
        assert_eq!(
            rotated.bounds,
            IntRect {
                x: row[1].parse().unwrap(),
                y: row[2].parse().unwrap(),
                width: row[3].parse().unwrap(),
                height: row[4].parse().unwrap()
            },
            "Rotated bounds case {cases}"
        );
        assert_eq!(
            rotated.center,
            [row[5].parse().unwrap(), row[6].parse().unwrap()]
        );
        let preview = GenomePreview::new(&genome, viewport[0], viewport[1]).unwrap();
        let row: Vec<_> = rows.next().unwrap().split('\t').collect();
        assert_eq!(row[0], "T");
        assert_eq!(
            preview.translation,
            [decoded(row[1]) as i32, decoded(row[2]) as i32]
        );
        close(preview.scale, decoded(row[3]), 1e-15, 1e-12);
        assert_eq!(body.forward_count(), count * symmetry as usize);
        for index in 0..body.forward_count() {
            let row: Vec<_> = rows.next().unwrap().split('\t').collect();
            assert_eq!(row[0], "S");
            let segment = &body.segment(index).unwrap();
            assert_eq!(
                segment.start,
                [row[1].parse().unwrap(), row[2].parse().unwrap()],
                "Case {cases}, segment {index}"
            );
            assert_eq!(
                segment.end,
                [row[3].parse().unwrap(), row[4].parse().unwrap()],
                "Case {cases}, segment {index}"
            );
            assert_eq!(body.color_argb(index).unwrap(), row[5].parse::<u32>().unwrap());
            assert_eq!(body.color_argb(index).unwrap() & 0xffffff, row[5].parse::<u32>().unwrap() & 0xffffff);
            close(segment.physical_length, decoded(row[6]), 1e-12, 1e-12);
            close(segment.start_radius, decoded(row[7]), 1e-12, 1e-12);
            close(segment.end_radius, decoded(row[8]), 1e-12, 1e-12);
            assert_eq!(
                rotated.segments[index].start,
                [row[9].parse().unwrap(), row[10].parse().unwrap()]
            );
            assert_eq!(
                rotated.segments[index].end,
                [row[11].parse().unwrap(), row[12].parse().unwrap()]
            );
            assert_eq!(
                preview.segments[index].start,
                [row[13].parse().unwrap(), row[14].parse().unwrap()]
            );
            assert_eq!(
                preview.segments[index].end,
                [row[15].parse().unwrap(), row[16].parse().unwrap()]
            );
            assert_eq!(
                preview.colors[index],
                row[17].parse::<i32>().unwrap() as u32
            );
            let reactions: Vec<i32> = row[18..]
                .iter()
                .map(|number| number.parse().unwrap())
                .collect();
            assert_eq!(segment.reactions, reactions.as_slice());
            segment_count += 1;
        }
        assert_eq!(rows.next().unwrap(), "E");
        cases += 1;
    }
    assert_eq!(polar, 18);
    assert_eq!(math_cases, 3169);
    assert_eq!(rounding, 10);
    assert_eq!(cases, 912);
    assert_eq!(segment_count, 34560);
}

#[test]
fn malformed_geometry_fails_explicitly_without_a_gene_count_cap() {
    let gene = Gene::new(2.5, 0.0, Some(0xffff0000), -1, [0; 36]).shared();
    let properties = GenomeProperties {
        symmetry: 1,
        ..GenomeProperties::default()
    };
    let genome = Genome::from_parts(&[gene.clone()], properties);
    for growth in [0, 17, i32::from(u8::MAX)] {
        assert_eq!(
            Body::at_growth(&genome, growth),
            Err(GeometryError::InvalidGrowth)
        );
    }
    assert!(
        Body::at_growth(&Genome::from_parts(&[], properties), 1)
            .unwrap()
            .segment_values().unwrap()
            .is_empty()
    );
    for symmetry in [-1, 0, 9] {
        let invalid = Genome::from_parts(
            &[gene.clone()],
            GenomeProperties {
                symmetry,
                ..properties
            },
        );
        assert_eq!(
            Body::at_growth(&invalid, 1),
            Err(GeometryError::InvalidSymmetry)
        );
    }
    gene.borrow_mut().set_color(None);
    assert_eq!(
        Body::at_growth(&genome, 1),
        Err(GeometryError::MissingColor { gene: 0 })
    );
    gene.borrow_mut().set_color(Some(0xffff0000));
    for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        gene.borrow_mut().set_length(number);
        assert_eq!(
            Body::at_growth(&genome, 1),
            Err(GeometryError::NonfiniteGene { gene: 0 })
        );
        gene.borrow_mut().set_length(2.5);
        gene.borrow_mut().set_theta(number);
        assert_eq!(
            Body::at_growth(&genome, 1),
            Err(GeometryError::NonfiniteGene { gene: 0 })
        );
        gene.borrow_mut().set_theta(0.0);
    }
    gene.borrow_mut().set_length(2.5);
    for branch in [-2, 2, i32::MAX] {
        let invalid = Gene::new(2.5, 0.0, Some(0xffff0000), branch, [0; 36]).shared();
        let branched = Genome::from_parts(&[gene.clone(), invalid], properties);
        assert_eq!(
            Body::at_growth(&branched, 1),
            Err(GeometryError::InvalidBranch { gene: 1 })
        );
    }
    gene.borrow_mut().set_branch(i32::MAX);
    assert!(Body::at_growth(&genome, 1).is_ok());
    gene.borrow_mut().set_branch(-1);
    let body = Body::at_growth(&genome, 1).unwrap();
    assert_eq!(
        body.rotated(f64::NAN, [0.0, 0.0]),
        Err(GeometryError::NonfiniteTransform)
    );
    assert_eq!(
        body.rotated(0.0, [f64::INFINITY, 0.0]),
        Err(GeometryError::NonfiniteTransform)
    );
    assert_eq!(
        Body::from_phenotype(&genome, 1, &[], EyeState::default(), 0),
        Err(GeometryError::PhenotypeLength)
    );
    assert_eq!(
        Body::from_segment_state(&genome, 1, &[0xffff0000], &[], EyeState::default(), 0),
        Err(GeometryError::ReactionLength)
    );
    assert_eq!(
        GenomePreview::new(&genome, 0, 30),
        Err(GeometryError::InvalidViewport)
    );
}

#[test]
fn rebuild_preserves_mutated_reactions_and_colors_from_java() {
    let mut lines = include_str!("../fixtures/geometry-rebuild.tsv").lines();
    let fields: Vec<_> = lines.next().unwrap().split('\t').collect();
    assert_eq!(fields[0], "C");
    let count: usize = fields[1].parse().unwrap();
    let symmetry: i32 = fields[2].parse().unwrap();
    let mirror: i32 = fields[3].parse().unwrap();
    let initial_growth: i32 = fields[4].parse().unwrap();
    let rebuilt_growth: i32 = fields[5].parse().unwrap();
    let mut genes = Vec::new();
    for _ in 0..count {
        let row: Vec<_> = lines.next().unwrap().split('\t').collect();
        assert_eq!(row[0], "G");
        let reactions: Vec<i32> = row[5..]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();
        genes.push(
            Gene::new(
                decoded(row[1]),
                decoded(row[2]),
                Some(row[3].parse().unwrap()),
                row[4].parse().unwrap(),
                reactions.try_into().unwrap(),
            )
            .shared(),
        );
    }
    let genome = Genome::from_parts(
        &genes,
        GenomeProperties {
            symmetry,
            mirror,
            ..GenomeProperties::default()
        },
    );
    let mut original = Body::at_growth(&genome, initial_growth).unwrap();
    for color_index in 0..original.forward_count() {
        let row: Vec<_> = lines.next().unwrap().split('\t').collect();
        assert_eq!(row[0], "P");
        original.set_color_argb(color_index, row[1].parse().unwrap()).unwrap();
        let reactions: Vec<i32> = row[2..]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();
        original.set_reaction_values(color_index, reactions.try_into().unwrap()).unwrap();
    }
    let rebuilt = original
        .rebuild(
            &genome,
            rebuilt_growth,
            original.eyes,
            original.update_effects,
        )
        .unwrap();
    let row: Vec<_> = lines.next().unwrap().split('\t').collect();
    assert_eq!(row[0], "B");
    assert_eq!(
        rebuilt.size().unwrap(),
        IntRect {
            x: row[1].parse().unwrap(),
            y: row[2].parse().unwrap(),
            width: row[3].parse().unwrap(),
            height: row[4].parse().unwrap()
        }
    );
    close(rebuilt.mass, decoded(row[5]), 1e-12, 1e-12);
    close(rebuilt.inertia, decoded(row[6]), 1e-9, 1e-12);
    assert_eq!(rebuilt.update_effects, row[7].parse::<i32>().unwrap());
    for index in 0..rebuilt.forward_count() {
        let row: Vec<_> = lines.next().unwrap().split('\t').collect();
        assert_eq!(row[0], "S");
        assert_eq!(
            rebuilt.segment(index).unwrap().start,
            [row[1].parse().unwrap(), row[2].parse().unwrap()]
        );
        assert_eq!(
            rebuilt.segment(index).unwrap().end,
            [row[3].parse().unwrap(), row[4].parse().unwrap()]
        );
        assert_eq!(rebuilt.color_argb(index).unwrap(), row[5].parse::<u32>().unwrap());
        assert_eq!(rebuilt.color_argb(index).unwrap() & 0xffffff, row[5].parse::<u32>().unwrap() & 0xffffff);
        close(rebuilt.physical_length(index).unwrap(), decoded(row[6]), 1e-12, 1e-12);
        close(rebuilt.segment(index).unwrap().start_radius, decoded(row[7]), 1e-12, 1e-12);
        close(rebuilt.segment(index).unwrap().end_radius, decoded(row[8]), 1e-12, 1e-12);
        let reactions: Vec<i32> = row[9..]
            .iter()
            .map(|value| value.parse().unwrap())
            .collect();
        assert_eq!(
            rebuilt.reaction_values(index).unwrap(),
            reactions.as_slice(),
            "Rebuilt segment {index}"
        );
        assert_eq!(original.reaction_values(index).unwrap(), reactions.as_slice());
    }
    assert!(lines.next().is_none());
}
