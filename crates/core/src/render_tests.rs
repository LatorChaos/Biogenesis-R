// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{ColorKind, Gene, GenomeProperties, RotatedSegment};

fn world() -> World {
    let world = World::new(WorldConfig {
        initial_attempts: 0,
        ..Default::default()
    })
    .unwrap();
    let colors = [
        ColorKind::Green.argb().unwrap(),
        ColorKind::Eye.argb().unwrap(),
        0x1200_1122,
    ];
    let genes = colors.map(|color| Gene::new(12.0, 0.0, Some(color), -1, [0; 36]).shared());
    world
        .paste_genome(
            Genome::from_parts(
                &genes,
                GenomeProperties {
                    symmetry: 1,
                    ..Default::default()
                },
            ),
            [1000, 1000],
        )
        .unwrap()
        .unwrap();
    for (index, color) in colors.into_iter().enumerate() {
        world.organisms.borrow()[0].borrow().body.set_color_argb(index, color).unwrap();
    }
    world
}

#[test]
fn rendering_reads_cached_endpoints_with_java_wrapping_without_changing_state() {
    let world = world();
    {
        let actor_ref = Rc::clone(&world.organisms.borrow()[0]);
        let mut actor = actor_ref.borrow_mut();
        actor.state.integer_center = [i32::MAX, i32::MIN];
        actor.body.set_rotated_segment(0, RotatedSegment {
            start: [1, -1],
            end: [-2, 3],
        }).unwrap();
        actor.state.angle = 1234.5;
    }
    let before = world.clone();
    let lines = world.render_segments().unwrap();
    assert_eq!(
        [lines[0].x1, lines[0].y1, lines[0].x2, lines[0].y2],
        [-2147483648.0, 2147483647.0, 2147483645.0, -2147483645.0]
    );
    assert_eq!(world, before);
}

#[test]
fn rendering_preserves_tints_eye_exemptions_dead_brown_and_full_argb() {
    let world = world();
    let eye = ColorKind::Eye.argb().unwrap();
    let green = ColorKind::Green.argb().unwrap();
    let brown = ColorKind::Brown.argb().unwrap();
    let tint = 0x34ab_cd98;
    for (alive, eyes, frames, expected) in [
        (true, true, 10, [tint, eye, tint]),
        (false, true, 10, [tint, eye, tint]),
        (false, false, 10, [tint; 3]),
        (false, false, 0, [brown; 3]),
        (false, true, 0, [green, eye, 0x1200_1122]),
        (true, false, 0, [green, eye, 0x1200_1122]),
        (true, true, -1, [green, eye, 0x1200_1122]),
    ] {
        {
            let actor_ref = Rc::clone(&world.organisms.borrow()[0]);
            let mut actor = actor_ref.borrow_mut();
            actor.state.alive = alive;
            actor.body.eyes.enabled = eyes;
            actor.set_display_color(Some(crate::JavaColor::from_argb(tint)));
            actor.state.temporary_color_frames = frames;
        }
        let before = world.clone();
        let actual: Vec<_> = world
            .render_segments().unwrap()
            .iter()
            .map(|line| (u32::from(line.alpha) << 24) | line.rgb)
            .collect();
        assert_eq!(
            actual, expected,
            "alive={alive} eyes={eyes} frames={frames}"
        );
        assert_eq!(world, before);
    }
}

#[test]
fn null_tints_retain_graphics_color_between_segments_and_organisms() {
    let world = world();
    let code = Rc::clone(world.organisms.borrow()[0].borrow().genome.as_ref().unwrap());
    let actor = world
        .construct_pasted_shared(code, [3500, 3500])
        .unwrap()
        .unwrap();
    world.add_organism(Rc::new(RefCell::new(actor)), None);
    for (index, actor) in world.organisms.borrow().iter().enumerate() {
        let mut actor = actor.borrow_mut();
        actor.state.temporary_color = None;
        actor.state.temporary_color_frames = 1;
        actor.body.eyes.enabled = index == 0;
    }
    let before = world.clone();
    let colors: Vec<_> = world
        .render_segments().unwrap()
        .iter()
        .map(|line| (u32::from(line.alpha) << 24) | line.rgb)
        .collect();
    let eye = ColorKind::Eye.argb().unwrap();
    assert_eq!(colors, [0xffff_ffff, eye, eye, eye, eye, eye]);
    assert_eq!(world, before);
}

#[test]
fn real_java_construction_and_bounded_random_match_exactly() {
    let mut current: Option<(World, Vec<RenderSegment>)> = None;
    let mut worlds = 0;
    let mut segments = 0;
    let mut random_cases = 0;
    for (line_index, line) in include_str!("../tests/fixtures/construction.tsv")
        .lines()
        .enumerate()
    {
        let fields: Vec<_> = line.split('\t').collect();
        match fields[0] {
            "Q" => {}
            "R" => {
                assert_eq!(fields.len(), 24);
                let mut random = JavaRandom::new(fields[1].parse().unwrap());
                let bound = fields[2].parse().unwrap();
                for expected in &fields[3..23] {
                    assert_eq!(
                        random.next_bounded(bound).unwrap(),
                        expected.parse::<i32>().unwrap()
                    );
                }
                assert_eq!(random.next_i64(), fields[23].parse::<i64>().unwrap());
                random_cases += 1;
            }
            "W" => {
                assert!(current.is_none());
                let world = World::new(WorldConfig {
                    seed: fields[1].parse().unwrap(),
                    width: fields[2].parse().unwrap(),
                    height: fields[3].parse().unwrap(),
                    initial_attempts: fields[4].parse().unwrap(),
                })
                .unwrap();
                assert_eq!(world.population(), fields[5].parse::<i32>().unwrap());
                assert_eq!(world.oxygen(), fields[6].parse::<f64>().unwrap());
                assert_eq!(world.carbon_dioxide(), fields[7].parse::<f64>().unwrap());
                current = Some((world, Vec::new()));
            }
            "S" => {
                let (_, expected) = current.as_mut().unwrap();
                expected.push(RenderSegment {
                    alpha: 255,
                    organism_id: fields[1].parse().unwrap(),
                    x1: fields[2].parse().unwrap(),
                    y1: fields[3].parse().unwrap(),
                    x2: fields[4].parse().unwrap(),
                    y2: fields[5].parse().unwrap(),
                    rgb: fields[6].parse().unwrap(),
                });
                segments += 1;
            }
            "E" => {
                let (world, expected) = current.take().unwrap();
                for actor in world.organisms.borrow().clone() {
                    crate::movement::calculate_bounds(&mut actor.borrow_mut(), true).unwrap();
                }
                assert_eq!(
                    world.render_segments().unwrap(),
                    expected,
                    "Construction ending at fixture line {}",
                    line_index + 1
                );
                worlds += 1;
            }
            _ => panic!("Unknown oracle record"),
        }
    }
    assert!(current.is_none());
    assert_eq!(worlds, 5);
    assert_eq!(random_cases, 8);
    assert_eq!(segments, 4403);
}
