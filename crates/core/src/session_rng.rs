// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

#[derive(Debug, PartialEq)]
pub struct NewGameError {
    pub error: WorldError,
    pub entered_genesis: bool,
}

impl fmt::Display for NewGameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { self.error.fmt(formatter) }
}
impl std::error::Error for NewGameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { Some(&self.error) }
}

/// A failed start returns the same session, including any completed genesis draws
#[derive(Debug, PartialEq)]
pub struct WorldStartError {
    pub error: WorldError,
    pub session: SessionRef,
    pub entered_genesis: bool,
}
impl fmt::Display for WorldStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { self.error.fmt(formatter) }
}
impl std::error::Error for WorldStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { Some(&self.error) }
}

impl World {
    /// Validate before replacing config or shared session settings, then retain genesis prefixes
    pub fn new_game(
        &self,
        width: i32,
        height: i32,
        initial_attempts: u32,
        parameters: WorldParameters,
    ) -> Result<(), NewGameError> {
        let config = WorldConfig { width, height, initial_attempts, ..self.config.get() };
        Self::validate_initialization(config, &parameters)
            .map_err(|error| NewGameError { error, entered_genesis: false })?;
        self.config.set(config);
        self.session.0.parameters.replace(parameters);
        self.genesis().map_err(|error| NewGameError { error, entered_genesis: true })
    }

    /// Retained escaped roots and the stopped session continue using one actual authority
    pub fn into_session(self) -> SessionRef { self.session() }

    /// Explicit independent-session compatibility constructor
    pub fn with_random(
        config: WorldConfig,
        parameters: WorldParameters,
        random: JavaRandom,
    ) -> Result<Self, WorldStartError> {
        Self::with_session(config, SessionRef::new(parameters, random))
    }

    pub fn with_session(config: WorldConfig, session: SessionRef) -> Result<Self, WorldStartError> {
        if let Err(error) = Self::validate_initialization(config, &session.parameters()) {
            return Err(WorldStartError { error, session, entered_genesis: false });
        }
        let world = Self::empty_in_session(config, session);
        if let Err(error) = world.genesis() {
            return Err(WorldStartError { error, session: world.into_session(), entered_genesis: true });
        }
        Ok(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, Gene, GenomeProperties};
    use std::num::NonZeroUsize;

    fn config(attempts: u32) -> WorldConfig {
        WorldConfig {
            seed: 827364,
            width: 7000,
            height: 7000,
            initial_attempts: attempts,
        }
    }

    fn green() -> Genome {
        Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), -1, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        )
    }

    #[test]
    fn new_game_uses_current_settings_and_continuing_rng_without_changing_pause() {
        let world = World::new(config(0)).unwrap();
        world.session.0.random.borrow_mut().next_i64();
        world.session.0.random.borrow_mut().next_bool();
        world.frame.set(19);
        world.tick.set(19);
        world.resume();
        let random = world.random_snapshot();
        let mut parameters = WorldParameters::default();
        parameters.atmosphere.initial_oxygen = 123.0;
        parameters.atmosphere.initial_carbon_dioxide = 10000.0;
        parameters.atmosphere.initial_methane = 17.0;
        parameters.initial_energy = 41.0;
        let expected_config = WorldConfig {
            width: 2000,
            height: 3000,
            initial_attempts: 32,
            ..config(0)
        };
        let expected = World::with_random(expected_config, parameters.clone(), random).unwrap();
        expected.resume();

        world.new_game(2000, 3000, 32, parameters).unwrap();

        assert_eq!(world.config.get(), expected_config);
        assert_eq!(world.parameters(), expected.parameters());
        assert_eq!(world.random_snapshot(), expected.random_snapshot());
        assert_eq!(world, expected);
        assert_eq!(world.render_segments().unwrap(), expected.render_segments().unwrap());
        assert_eq!(world.population.get(), expected.population.get());
        assert_eq!(world.oxygen.get().to_bits(), expected.oxygen.get().to_bits());
        assert_eq!(
            world.carbon_dioxide.get().to_bits(),
            expected.carbon_dioxide.get().to_bits()
        );
        assert_eq!(world.methane.get().to_bits(), expected.methane.get().to_bits());
        assert_eq!((world.frame(), world.tick(), world.time()), (0, 0, 0));
        assert_eq!(world.statistics_ref().borrow().get_min_population(), 32);
        assert!(!world.paused());
    }

    #[test]
    fn new_game_retains_frozen_objects_corridor_order_queues_and_traveller_aliases() {
        let mut parameters = WorldParameters::default();
        parameters.outgoing = Some(OutgoingConfig {
            user_name: "session-test".into(),
            pending_per_connection: NonZeroUsize::new(2).unwrap(),
        });
        let world = World::with_parameters(config(0), parameters.clone()).unwrap();
        world.paste_genome(green(), [1000, 1000]).unwrap().unwrap();
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        let genome = Rc::clone(actor.borrow().genome.as_ref().unwrap());
        let old_bounds = actor.borrow().state.bounds.get();
        world.rebuild_spatial();
        let incoming = world.new_incoming_corridor();
        incoming.borrow_mut().receive(Some(Rc::clone(&genome)));
        world.add_incoming_corridor(Rc::clone(&incoming));
        world.add_incoming_corridor(Rc::clone(&incoming));
        let connection = world.new_connection_core(51).unwrap();
        let outgoing = world.new_outgoing_corridor(Rc::clone(&connection));
        world.add_outgoing_corridor(Rc::clone(&outgoing));
        assert!(
            world
                .send_outgoing(&outgoing, &actor, &mut actor.borrow_mut())
                .unwrap()
        );
        let random = world.random_snapshot();
        let spatial = world.spatial.borrow().clone();
        let incoming_bounds = incoming.borrow().bounds;
        let outgoing_bounds = outgoing.borrow().bounds;

        world.new_game(100, 100, 0, parameters).unwrap();

        assert_eq!(world.random_snapshot(), random);
        assert!(world.organisms.borrow().is_empty() && world.population.get() == 0);
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &actor));
        assert_eq!(*world.spatial.borrow(), spatial);
        assert_eq!(world.spatial.borrow().query(old_bounds), [0]);
        assert!(world.corridors_exist.get());
        assert_eq!(world.incoming_corridors.borrow().len(), 2);
        assert!(
            world.incoming_corridors.borrow().iter()
                .all(|item| Rc::ptr_eq(item, &incoming))
        );
        assert!(Rc::ptr_eq(&world.outgoing_corridors.borrow()[0], &outgoing));
        assert_eq!(incoming.borrow().bounds, incoming_bounds);
        assert_eq!(outgoing.borrow().bounds, outgoing_bounds);
        assert!(Rc::ptr_eq(
            outgoing.borrow().traveller.as_ref().unwrap(),
            &actor
        ));
        assert!(Rc::ptr_eq(&outgoing.borrow().connection, &connection));
        assert!(Rc::ptr_eq(
            incoming.borrow().pending[0].as_ref().unwrap(),
            &genome
        ));
        assert!(Rc::ptr_eq(
            connection
                .borrow()
                .pending
                .entries()
                .next()
                .unwrap()
                .genome(),
            &genome
        ));
        genome
            .borrow_mut()
            .set_clade_id(Some("retained".to_owned()));
        assert_eq!(
            world.frame_objects.borrow()[0].borrow().genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
            Some("retained")
        );
        world.incoming_corridors.borrow_mut().clear();
        world.outgoing_corridors.borrow_mut().clear();
        world
            .new_game(100, 100, 0, world.parameters().clone())
            .unwrap();
        assert!(world.corridors_exist.get());
    }

    #[test]
    fn invalid_new_game_preserves_the_live_graph_parameters_and_rng() {
        let world = World::new(config(0)).unwrap();
        world.paste_genome(green(), [1000, 1000]).unwrap().unwrap();
        world.rebuild_spatial();
        let actor = Rc::clone(&world.organisms.borrow()[0]);
        let incoming = world.new_incoming_corridor();
        world.add_incoming_corridor(Rc::clone(&incoming));
        world.resume();
        let before = world.clone();
        let mut invalid = world.parameters().clone();
        invalid.atmosphere.initial_oxygen = f64::NAN;
        for (width, attempts, parameters) in [
            (0, 0, world.parameters().clone()),
            (7000, MAX_INITIAL_ATTEMPTS + 1, world.parameters().clone()),
            (7000, 0, invalid),
        ] {
            let failure = world
                .new_game(width, 7000, attempts, parameters)
                .unwrap_err();
            assert!(!failure.entered_genesis);
            assert_eq!(world, before);
            assert!(Rc::ptr_eq(&world.organisms.borrow()[0], &actor));
            assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &actor));
            assert!(Rc::ptr_eq(&world.incoming_corridors.borrow()[0], &incoming));
        }
    }

    #[test]
    fn new_game_accepts_source_infinite_atmosphere_without_advancing_rng() {
        let world = World::new(config(0)).unwrap();
        world.session.0.random.borrow_mut().next_i64();
        world.frame.set(19);
        world.tick.set(19);
        world.resume();
        let random = world.random_snapshot();
        let mut parameters = world.parameters().clone();
        parameters.atmosphere.initial_oxygen = f64::INFINITY;
        parameters.atmosphere.initial_carbon_dioxide = f64::INFINITY;
        parameters.atmosphere.initial_methane = f64::INFINITY;

        world.new_game(7000, 7000, 0, parameters.clone()).unwrap();

        assert_eq!(world.parameters(), parameters);
        assert_eq!(world.oxygen.get(), f64::INFINITY);
        assert_eq!(world.carbon_dioxide.get(), f64::INFINITY);
        assert_eq!(world.methane.get(), f64::INFINITY);
        assert_eq!((world.frame(), world.tick(), world.time()), (0, 0, 0));
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(world.population(), 0);
        assert!(!world.paused());
    }

    #[test]
    fn stopped_session_moves_the_stream_and_releases_world_only_actor_roots() {
        let world = World::new(config(0)).unwrap();
        world.paste_genome(green(), [1000, 1000]).unwrap().unwrap();
        world.rebuild_spatial();
        let actor = Rc::downgrade(&world.organisms.borrow()[0]);
        world.session.0.random.borrow_mut().next_i64();
        let expected = world.random_snapshot();

        let random = world.into_session().random_snapshot();

        assert!(actor.upgrade().is_none());
        assert_eq!(random, expected);
        let resumed = World::with_random(config(0), WorldParameters::default(), random).unwrap();
        assert_eq!(resumed.random_snapshot(), expected);
        assert_eq!((resumed.tick(), resumed.population()), (0, 0));
        assert!(resumed.organisms.borrow().is_empty() && resumed.frame_objects.borrow().is_empty());
        assert!(resumed.incoming_corridors.borrow().is_empty() && resumed.outgoing_corridors.borrow().is_empty());
        assert!(!resumed.corridors_exist.get() && resumed.paused());
    }

    #[test]
    fn invalid_session_start_returns_the_unchanged_owned_rng_for_retry() {
        let mut random = JavaRandom::new(-827364);
        random.next_i64();
        random.next_bool();
        let expected = random.clone();
        let invalid_config = WorldConfig {
            width: 0,
            ..config(0)
        };

        let failure =
            World::with_random(invalid_config, WorldParameters::default(), random).unwrap_err();

        assert!(!failure.entered_genesis);
        assert_eq!(failure.error, WorldError::InvalidDimensions);
        assert_eq!(failure.session.random_snapshot(), expected);
        let world =
            World::with_random(config(0), WorldParameters::default(), failure.session.random_snapshot()).unwrap();
        assert_eq!(world.random_snapshot(), expected);
        assert_ne!(world.random_snapshot(), JavaRandom::new(world.seed()));
    }

    #[test]
    fn source_attempt_domain_preserves_genesis_and_validation_rng_boundaries() {
        let parameters = WorldParameters::default();
        let mut random = JavaRandom::new(config(0).seed);
        random.next_i64();
        let small = WorldConfig {
            width: 1,
            height: 1,
            ..config(1)
        };
        let expected = World::with_random(small, parameters.clone(), random.clone()).unwrap_err();
        assert!(expected.entered_genesis);
        assert!(matches!(&expected.error, WorldError::BodyDoesNotFit { .. }));
        assert_ne!(expected.session.random_snapshot(), random);

        for attempts in [12_501, i32::MAX as u32] {
            let failure = World::with_random(
                WorldConfig { initial_attempts: attempts, ..small },
                parameters.clone(),
                random.clone(),
            )
            .unwrap_err();
            assert!(failure.entered_genesis);
            assert_eq!(failure.error, expected.error);
            assert_eq!(failure.session.random_snapshot(), expected.session.random_snapshot());
        }

        let failure = World::with_random(
            WorldConfig { initial_attempts: i32::MAX as u32 + 1, ..small },
            parameters,
            random.clone(),
        )
        .unwrap_err();
        assert!(!failure.entered_genesis);
        assert_eq!(failure.error, WorldError::InitialAttemptsExceeded { maximum: i32::MAX as u32 });
        assert_eq!(failure.session.random_snapshot(), random);
    }

    #[test]
    fn construction_failures_retain_actual_rng_progress_and_new_game_partial_state() {
        let small = WorldConfig {
            width: 1,
            height: 1,
            initial_attempts: 1,
            ..config(0)
        };
        let parameters = WorldParameters::default();
        let mut random = JavaRandom::new(config(0).seed);
        random.next_i64();
        let expected = World::empty_validated(small, parameters.clone(), random.clone());
        let expected_error = expected.genesis().unwrap_err();
        assert!(matches!(expected_error, WorldError::BodyDoesNotFit { .. }));
        assert_ne!(expected.random_snapshot(), random);

        let failure = World::with_random(small, parameters.clone(), random.clone()).unwrap_err();

        assert!(failure.entered_genesis);
        assert_eq!(failure.error, expected_error);
        assert_eq!(failure.session.random_snapshot(), expected.random_snapshot());
        let world = World::new(config(0)).unwrap();
        world.session.0.random.replace(random);
        world.resume();
        let failure = world.new_game(1, 1, 1, parameters).unwrap_err();
        assert!(failure.entered_genesis);
        assert_eq!(failure.error, expected_error);
        assert_eq!(world.config.get(), small);
        assert_eq!(world.random_snapshot(), expected.random_snapshot());
        assert_eq!(world.oxygen.get().to_bits(), expected.oxygen.get().to_bits());
        assert_eq!(
            world.carbon_dioxide.get().to_bits(),
            expected.carbon_dioxide.get().to_bits()
        );
        assert_eq!(world.methane.get().to_bits(), expected.methane.get().to_bits());
        assert!(!world.paused());
    }

    #[test]
    fn genetic_parameter_failure_after_genesis_reports_actual_partial_mutation() {
        let mut parameters = WorldParameters::default();
        parameters.genetics.initial_complexity = i32::MAX;
        parameters.genetics.validate().unwrap();
        let world = World::with_parameters(config(0), parameters.clone()).unwrap();
        world.frame.set(19);
        world.tick.set(19);
        world.resume();
        let random_before = world.random_snapshot();
        parameters.atmosphere.initial_oxygen = 123.0;

        // Seed827364 rejects its wrapped negative gene count before allocating
        let failure = world.new_game(7000, 7000, 1, parameters).unwrap_err();

        assert!(failure.entered_genesis);
        assert!(matches!(failure.error, WorldError::InvalidParameters(_)));
        assert_eq!((world.frame(), world.tick()), (0, 0));
        assert_eq!(world.initial_attempts(), 1);
        assert_eq!(world.oxygen.get().to_bits(), 123.0_f64.to_bits());
        assert_ne!(world.random_snapshot(), random_before);
        assert!(!world.paused());
    }

    #[test]
    fn explicit_reset_still_replays_the_configured_seed_after_session_continuation() {
        let configured = config(32);
        let expected = World::new(configured).unwrap();
        let mut random = JavaRandom::new(configured.seed);
        random.next_i64();
        let mut world = World::with_random(configured, WorldParameters::default(), random).unwrap();
        assert_ne!(world.random_snapshot(), expected.random_snapshot());
        world
            .new_game(
                configured.width,
                configured.height,
                configured.initial_attempts,
                WorldParameters::default(),
            )
            .unwrap();
        assert_ne!(world.random_snapshot(), expected.random_snapshot());

        world.reset().unwrap();

        assert_eq!(world, expected);
        assert!(world.paused());
    }
}
