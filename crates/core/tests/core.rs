// SPDX-License-Identifier: GPL-2.0-or-later

use biogenesis_core::{JavaRandom, World, WorldConfig, WorldError};

#[test]
fn pause_single_step_reset_and_snapshots_preserve_authority() {
    let config = WorldConfig {
        seed: 42,
        width: 128,
        height: 96,
        initial_attempts: 32,
    };
    let mut world = World::new(config).unwrap();
    let initial = world.clone();
    assert_eq!(world.seed(), 42);
    assert_eq!(world.dimensions(), (128, 96));
    assert!(world.paused());
    assert!(!world.step().unwrap());
    assert_eq!(world, initial);
    let first_snapshot = world.render_segments().unwrap();
    assert_eq!(world.render_segments().unwrap(), first_snapshot);
    assert_eq!(world, initial);
    assert!(world.single_step().unwrap());
    assert_eq!(world.tick(), 1);
    assert!(world.paused());
    world.resume();
    let running = world.clone();
    assert!(!world.single_step().unwrap());
    assert_eq!(world, running);
    assert!(world.step().unwrap());
    assert_eq!(world.tick(), 2);
    world.pause();
    assert!(!world.step().unwrap());
    world.reset().unwrap();
    assert_eq!(world, initial);
    assert_eq!(world.render_segments().unwrap(), first_snapshot);
    let other = World::new(WorldConfig { seed: 0, ..config }).unwrap();
    assert_ne!(world.render_segments().unwrap(), other.render_segments().unwrap());
}

#[test]
fn atmosphere_matches_independent_closed_form_and_conserves_carbon() {
    let world = World::new(WorldConfig {
        initial_attempts: 0,
        ..WorldConfig::default()
    })
    .unwrap();
    assert!(world.single_step().unwrap());
    assert_eq!(world.carbon_dioxide(), 499_500.0);
    assert_eq!(world.methane(), 500.0);
    assert!(world.single_step().unwrap());
    assert_eq!(world.carbon_dioxide(), 499_001.0);
    assert_eq!(world.methane(), 999.0);
    for _ in 2..1024 {
        assert!(world.single_step().unwrap());
    }
    // The two ordered reference reactions reduce to a two-state linear exchange
    let expected_carbon = 250_000.0 * (1.0 + 0.998_f64.powi(1024));
    assert!((world.carbon_dioxide() - expected_carbon).abs() < 1e-7);
    assert!((world.carbon_dioxide() + world.methane() - 500_000.0).abs() < 1e-7);
    assert_eq!(world.oxygen(), 0.0);
}

#[test]
fn invalid_inputs_fail_before_unbounded_work_or_random_draws() {
    let mut random = JavaRandom::new(42);
    for bound in [0, -1, i32::MIN] {
        assert!(random.next_bounded(bound).is_err());
    }
    assert_eq!(random, JavaRandom::new(42));
    assert_eq!(JavaRandom::new(0), JavaRandom::new(1_i64 << 48));
    for (width, height) in [(0, 32), (32, 0), (i32::MIN, 32), (32, -1)] {
        assert_eq!(
            World::new(WorldConfig {
                width,
                height,
                initial_attempts: 0,
                ..WorldConfig::default()
            }),
            Err(WorldError::InvalidDimensions)
        );
    }
    let small = WorldConfig {
        width: 1,
        height: 1,
        initial_attempts: 1,
        ..WorldConfig::default()
    };
    let maximum = i32::MAX as u32;
    assert_eq!(
        World::new(WorldConfig {
            initial_attempts: maximum + 1,
            ..small
        }),
        Err(WorldError::InitialAttemptsExceeded { maximum })
    );
    assert!(matches!(
        World::new(small),
        Err(WorldError::BodyDoesNotFit { .. })
    ));
}
