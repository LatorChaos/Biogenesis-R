// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use biogenesis_core::{
    JavaRandom, OrganismAction, OrganismHandle, OrganismSnapshot, SessionRef, World, WorldConfig,
    WorldError, WorldParameters,
};
use godot::builtin::{
    PackedByteArray, PackedFloat64Array, PackedInt32Array, PackedInt64Array, VarDictionary,
};
use godot::prelude::*;
use std::time::{Duration, Instant};

mod canvas_lines;
mod image_export;
mod laboratory;
mod settings;
mod statistics;
mod living_scene;
mod feeding_scene;
mod number_format;
mod preferences;
mod automatic_backup;
mod recovery_store;
mod world_file;
mod network_io;
mod network;
use laboratory::BgrGenomeDraft;

const MAX_COMMANDS: usize = 256;
const MAX_TICKS: i64 = 4096;
const MAX_BUDGET_MICROS: i64 = 50_000;

struct DetachedSession {
    seed: i64,
    session: SessionRef,
}

#[derive(Debug)]
struct NewGameFailure {
    message: String,
    entered_genesis: bool,
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct BgrSimulation {
    world: Option<World>,
    detached_session: Option<DetachedSession>,
    selected: Option<OrganismHandle>,
    inspector: Option<InspectorReadout>,
    tracked: Option<OrganismHandle>,
    clipboard: Option<biogenesis_core::GenomeHandle>,
    preview_genome: Option<Option<biogenesis_core::GenomeHandle>>,
    settings: Option<biogenesis_core::SettingsState>,
    settings_applying: bool,
    settings_presentation: Option<VarDictionary>,
    statistics_session: Option<statistics::StatisticsSession>,
    living_scene: living_scene::LivingSceneState,
    feeding_scene: feeding_scene::FeedingSceneState,
    preferences: preferences::PreferenceSession,
    automatic_backup: automatic_backup::AutomaticBackup,
    network: network::Network,
}

#[godot_api]
impl BgrSimulation {
    #[func]
    fn start_session(
        &mut self,
        seed: i64,
        width: i64,
        height: i64,
        attempts: i64,
    ) -> VarDictionary {
        let error = self
            .start_session_world(seed, width, height, attempts)
            .err();
        self.response(error.as_deref())
    }

    #[func]
    fn new_game(&mut self, width: i64, height: i64, attempts: i64) -> VarDictionary {
        let outcome = self.new_game_world(width, height, attempts);
        let started = outcome
            .as_ref()
            .map_or_else(|failure| failure.entered_genesis, |_| true);
        let error = outcome.err().map(|failure| failure.message);
        let mut response = self.response(error.as_deref());
        response.set("new_game_started", started);
        response
    }

    #[func]
    fn release_world(&mut self) -> VarDictionary {
        self.release_world_state();
        let error = self.network_cleanup_error();
        self.response(error.as_deref())
    }

    #[func]
    fn configure(&mut self, seed: i64, width: i64, height: i64, attempts: i64) -> VarDictionary {
        if self.automatic_backup.source_pending { return self.response(Some("Finish the pending Source Backup Save As action first")); }
        self.automatic_backup.finish_pending();
        let config = match configuration(seed, width, height, attempts) {
            Ok(config) => config,
            Err(message) => return self.response(Some(&message)),
        };
        let parameters = self.settings_parameters();
        match World::with_random(config, parameters, JavaRandom::new(config.seed)) {
            Ok(world) => {
                self.record_applied_config(config);
                self.network_world_ready(&world);
                self.living_scene.clear();
                self.feeding_scene.clear();
                self.world = Some(world);
                self.statistics_session = None;
                self.detached_session = None;
                self.selected = None;
                self.tracked = None;
                self.automatic_backup_check();
                self.response(None)
            }
            Err(failure) => {
                if failure.entered_genesis {
                    self.record_applied_config(config);
                }
                self.response(Some(&failure.error.to_string()))
            }
        }
    }

    #[func]
    fn apply_commands(&mut self, commands: PackedInt32Array) -> VarDictionary {
        if let Err(message) = validate_commands(commands.as_slice()) {
            return self.response(Some(message));
        }
        if self.automatic_backup.source_pending && commands.as_slice().iter().any(|command| *command != 0) {
            let mut output = self.response(None);
            output.set("commands_completed", 0_i64);
            return output;
        }
        if commands.as_slice().contains(&2) { self.begin_feeding_batch(); }
        let policy = self.automatic_policy();
        let Some(world) = self.world.as_mut() else {
            return self.response(Some("Initialize a world first"));
        };
        let tracked = self.tracked.clone();
        let mut tracking_path = Vec::new();
        let mut completed = 0_i64;
        for command in commands.as_slice() {
            match command {
                0 => world.pause(),
                1 => world.resume(),
                2 => match world.single_step() {
                    Ok(true) => {
                        recalculate_inspector(world, self.selected.as_ref(), &mut self.inspector);
                        record_tracking(tracked.as_ref(), &mut tracking_path);
                    },
                    Ok(false) => {
                        return self.response_with_tracking(
                            Some("Pause before advancing one tick"),
                            &tracking_path,
                        );
                    }
                    Err(error) => {
                        return self
                            .response_with_tracking(Some(&error.to_string()), &tracking_path);
                    }
                },
                3 => {
                    if let Err(error) = world.reset() {
                        return self
                            .response_with_tracking(Some(&error.to_string()), &tracking_path);
                    }
                    self.living_scene.clear();
                    self.feeding_scene.clear();
                    self.feeding_scene.begin(Some(world));
                    self.selected = None;
                    self.tracked = None;
                    let (width, height) = world.dimensions();
                    settings::record_applied(
                        &mut self.settings,
                        WorldConfig {
                            seed: world.seed(),
                            width,
                            height,
                            initial_attempts: world.initial_attempts(),
                        },
                    );
                }
                4 => {
                    if let Err(error) = world.kill_all() {
                        return self.response_with_tracking(Some(&error.to_string()), &tracking_path);
                    }
                },
                5 => {
                    if let Err(error) = world.disperse_all() {
                        return self.response_with_tracking(Some(&error.to_string()), &tracking_path);
                    }
                },
                6 => world.add_carbon_dioxide(1000.0),
                7 => world.decrease_carbon_dioxide(1000.0),
                8 => world.add_methane(1000.0),
                9 => world.decrease_methane(1000.0),
                _ => unreachable!("command batch was validated"),
            }
            completed += 1;
            self.automatic_backup.observe(world, policy);
            if self.automatic_backup.source_pending { break; }
        }
        let mut output = self.response_with_tracking(None, &tracking_path);
        output.set("commands_completed", completed);
        output
    }

    #[func]
    fn advance(&mut self, ticks: i64) -> VarDictionary {
        if !(0..=MAX_TICKS).contains(&ticks) {
            return self.response(Some("Tick batch must contain between 0 and 4096 ticks"));
        }
        if self.automatic_backup.source_pending { return self.response(None); }
        self.begin_feeding_batch();
        let policy = self.automatic_policy();
        let checkpoint_barrier = std::cell::Cell::new(false);
        let automatic = &mut self.automatic_backup;
        let selected = self.selected.clone();
        let inspector = &mut self.inspector;
        let Some(world) = self.world.as_mut() else {
            return self.response(Some("Initialize a world first"));
        };
        let tracked = self.tracked.clone();
        let mut tracking_path = Vec::new();
        let result = advance_world(
            world,
            ticks,
            || checkpoint_barrier.get(),
            |world| {
                recalculate_inspector(world, selected.as_ref(), inspector);
                record_tracking(tracked.as_ref(), &mut tracking_path);
                automatic.observe(world, policy);
                checkpoint_barrier.set(automatic.source_pending);
            },
        );
        match result {
            Ok(()) => self.response_with_tracking(None, &tracking_path),
            Err(error) => self.response_with_tracking(Some(&error.to_string()), &tracking_path),
        }
    }

    /// Yield between complete ticks; a single tick and snapshot packing can exceed the requested budget
    #[func]
    fn advance_budgeted(&mut self, ticks: i64, budget_micros: i64) -> VarDictionary {
        if !(0..=MAX_TICKS).contains(&ticks) {
            return self.response(Some("Tick batch must contain between 0 and 4096 ticks"));
        }
        if !(1..=MAX_BUDGET_MICROS).contains(&budget_micros) {
            return self.response(Some(
                "Simulation budget must contain between 1 and 50000 microseconds",
            ));
        }
        if self.automatic_backup.source_pending { return self.response(None); }
        self.begin_feeding_batch();
        let policy = self.automatic_policy();
        let checkpoint_barrier = std::cell::Cell::new(false);
        let automatic = &mut self.automatic_backup;
        let selected = self.selected.clone();
        let inspector = &mut self.inspector;
        let Some(world) = self.world.as_mut() else {
            return self.response(Some("Initialize a world first"));
        };
        let budget = Duration::from_micros(budget_micros as u64);
        let started = Instant::now();
        let tracked = self.tracked.clone();
        let mut tracking_path = Vec::new();
        let result = advance_world(
            world,
            ticks,
            || checkpoint_barrier.get() || started.elapsed() >= budget,
            |world| {
                recalculate_inspector(world, selected.as_ref(), inspector);
                record_tracking(tracked.as_ref(), &mut tracking_path);
                automatic.observe_background(world, policy);
                checkpoint_barrier.set(automatic.source_pending);
            },
        );
        match result {
            Ok(()) => self.response_with_tracking(None, &tracking_path),
            Err(error) => self.response_with_tracking(Some(&error.to_string()), &tracking_path),
        }
    }

    #[func]
    fn snapshot(&mut self) -> VarDictionary {
        if self.world.is_none() {
            return self.response(Some("Initialize a world first"));
        }
        self.response(None)
    }

    #[func]
    fn select_at(&mut self, x: f64, y: f64, preserve_empty: bool) -> VarDictionary {
        let point = match selection_position(x, y) {
            Ok(point) => point,
            Err(message) => return self.response(Some(message)),
        };
        match self.select_position_state(point, preserve_empty) {
            Ok(hit) => {
                let mut response = self.response(None);
                response.set("selection_hit", hit);
                response
            }
            Err(error) => self.response(Some(&error)),
        }
    }

    #[func]
    fn organism_action(&mut self, name: GString) -> VarDictionary {
        let action = match selected_action(&name.to_string()) {
            Ok(action) => action,
            Err(message) => return self.response(Some(message)),
        };
        let Some(handle) = self.selected.clone() else {
            return self.response(Some("Select an organism first"));
        };
        let Some(world) = self.world.as_mut() else {
            return self.response(Some("Initialize a world first"));
        };
        if handle.owner_world().inspect_handle(&handle).is_none() {
            self.selected = None;
            return self.response(Some("The selected organism has left this world"));
        }
        match handle.act_in_visible_world(world, action) {
            Ok(applied) => {
                let mut response = self.response(None);
                response.set("action_applied", applied);
                response
            }
            Err(error) => {
                world.pause();
                let mut response = self.response(Some(&error.to_string()));
                response.set("runtime_error", true);
                response
            }
        }
    }

    #[func]
    fn track_selection(&mut self) -> VarDictionary {
        let Some(handle) = self.selected.clone() else {
            return self.response(Some("Select a living organism to track"));
        };
        let alive = handle.owner_world().tracking_center(&handle).is_some();
        if !alive {
            return self.response(Some("Select a living organism to track"));
        }
        self.tracked = if self.tracked.as_ref() == Some(&handle) {
            None
        } else {
            Some(handle)
        };
        self.response(None)
    }

    #[func]
    fn open_laboratory(&self, source: GString) -> VarDictionary {
        self.open_draft(&source.to_string())
    }

    #[func]
    fn laboratory_snapshot(
        &self,
        draft: Gd<BgrGenomeDraft>,
        width: i64,
        height: i64,
    ) -> VarDictionary {
        self.read_draft(draft, width, height)
    }

    #[func]
    fn laboratory_set_property(
        &self,
        mut draft: Gd<BgrGenomeDraft>,
        field: GString,
        value: Variant,
    ) -> VarDictionary {
        let Some(world) = self.world.as_ref() else {
            let mut result = VarDictionary::new();
            result.set("ok", false);
            result.set("error", "Initialize a world first");
            return result;
        };
        let parameters = world.genetics_parameters();
        draft.bind_mut().set_property_value(
            field,
            value,
            [parameters.min_mutation_rate, parameters.max_mutation_rate],
        )
    }

    #[func]
    fn commit_laboratory(&mut self, draft: Gd<BgrGenomeDraft>) -> VarDictionary {
        self.commit_draft(draft)
    }

    #[func]
    fn copy_selection(&mut self) -> VarDictionary {
        self.copy_genome()
    }

    #[func]
    fn copy_preview(&mut self) -> VarDictionary {
        self.copy_preview_state();
        self.response(None)
    }

    #[func]
    fn paste_clipboard(&mut self, x: f64, y: f64) -> VarDictionary {
        self.place_genome(x, y, false, None)
    }

    #[func]
    fn random_organism(&mut self, x: f64, y: f64) -> VarDictionary {
        self.place_genome(x, y, true, None)
    }

    #[func]
    fn import_organism(&mut self, path: GString, x: f64, y: f64) -> VarDictionary {
        self.place_genome(x, y, false, Some(std::path::Path::new(&path.to_string())))
    }

    #[func]
    fn export_selection(&self, path: GString) -> VarDictionary {
        self.export_genome(std::path::Path::new(&path.to_string()))
    }

    #[func]
    fn prepare_selection_image(&self) -> VarDictionary {
        self.selected_image()
    }

    #[func]
    fn genotype_destination(&self, path: GString) -> VarDictionary {
        laboratory::destination(std::path::Path::new(&path.to_string()))
    }

    #[func]
    fn abort_tracking(&mut self) -> VarDictionary {
        self.tracked = None;
        self.response(None)
    }
}

impl BgrSimulation {
    fn response_with_tracking(&mut self, error: Option<&str>, path: &[i32]) -> VarDictionary {
        let mut response = self.response(error);
        response.set("tracking_path", &PackedInt32Array::from(path));
        response
    }

    fn select_position_state(&mut self, point: [i32; 2], preserve_empty: bool) -> Result<bool, String> {
        let world = self.world.as_ref().ok_or("Initialize a world first")?;
        let selected = world.pick_organism(point).map_err(|error| error.to_string())?;
        let hit = selected.is_some();
        if hit || !preserve_empty {
            self.selected = selected;
            self.inspector = None;
        }
        Ok(hit)
    }

    fn copy_preview_state(&mut self) {
        if let Some(Some(genome)) = self.preview_genome.clone() {
            self.clipboard = Some(genome);
        }
    }

    fn response(&mut self, error: Option<&str>) -> VarDictionary {
        self.automatic_backup.poll_pending();
        let feeding = self.feeding_dictionary();
        debug_assert!(self.world.is_none() || self.detached_session.is_none());
        let selected_owner = self.selected.as_ref().map(OrganismHandle::owner_world);
        let selected = selected_owner.as_ref().and_then(|world| {
            self.selected
                .as_ref()
                .and_then(|handle| world.inspect_handle(handle))
        });
        if selected.is_none() {
            self.selected = None;
        }
        self.preview_genome = selected_owner.as_ref().and_then(|world| {
            self.selected
                .as_ref()
                .and_then(|actor| world.inspect_genome(actor))
        });
        let tracked = self.tracked.as_ref()
            .and_then(|handle| handle.owner_world().tracking_center(handle));
        if tracked.is_none() {
            self.tracked = None;
        }
        let mut output = VarDictionary::new();
        output.set("automatic_backup", &self.automatic_status());
        output.set("feeding", &feeding);
        output.set("ok", error.is_none());
        output.set("world_ready", self.world.is_some());
        output.set(
            "session_ready",
            self.world.is_some() || self.detached_session.is_some(),
        );
        if let Some(detached) = &self.detached_session {
            output.set("seed", detached.seed);
        }
        output.set("clipboard_ready", self.clipboard.is_some());
        output.set("statistics_open", self.statistics_session.is_some());
        output.set("error", error.unwrap_or(""));
        output.set("tracking_path", &PackedInt32Array::new());
        output.set("selected", &selection_fields(selected.as_ref(), &mut self.inspector, selected_owner.as_ref(), self.selected.as_ref()));
        let preview = if selected.is_some() {
            self.preview_genome
                .as_ref().and_then(Option::as_ref)
                .map(|genome| genome.preview_image(100, 100, 64 * 1024 * 1024))
        } else {
            None
        };
        match preview {
            Some(Ok(preview)) => {
                output.set("genotype_preview", &statistics::image_fields(preview));
                output.set("genotype_preview_error", "");
            }
            Some(Err(error)) => {
                output.set("genotype_preview", &VarDictionary::new());
                output.set("genotype_preview_error", error.to_string());
            }
            None => {
                output.set("genotype_preview", &VarDictionary::new());
                output.set("genotype_preview_error", if selected.as_ref().is_some_and(|actor| actor.genotype.is_none()) { "Primary genotype unavailable" } else { "" });
            }
        }
        output.set("tracking", tracked.is_some());
        output.set(
            "selected_is_tracked",
            self.selected.is_some() && self.selected == self.tracked,
        );
        let target: Vec<f64> = tracked
            .as_ref()
            .map(|center| center.map(f64::from).to_vec())
            .unwrap_or_default();
        output.set(
            "tracking_center",
            &PackedFloat64Array::from(target.as_slice()),
        );
        if let Some(world) = &self.world {
            let segments = match world.render_segments_from_color(self.network_draw_paint(world)) {
                Ok(segments) => segments,
                Err(failure) => {
                    output.set("ok", false);
                    output.set("draw_error", failure.to_string());
                    if error.is_none() {
                        output.set("error", failure.to_string());
                    }
                    match failure {
                        WorldError::Drawing { commands, paint, .. }
                        | WorldError::DrawingList { commands, paint, .. } => {
                            output.set("draw_paint", i64::from(paint));
                            commands.into_iter().filter_map(|command| match command {
                                biogenesis_core::DrawCommand::Line(line) => Some(line),
                                _ => None,
                            }).collect()
                        }
                        _ => Vec::new(),
                    }
                }
            };
            let mut points = Vec::with_capacity(segments.len() * 4);
            let mut colors = Vec::with_capacity(segments.len());
            let mut alphas = Vec::with_capacity(segments.len());
            let mut identities = Vec::with_capacity(segments.len());
            for segment in segments {
                points.extend_from_slice(&[segment.x1, segment.y1, segment.x2, segment.y2]);
                colors.push(segment.rgb as i32);
                alphas.push(segment.alpha);
                identities.push(segment.organism_id);
            }
            let (width, height) = world.dimensions();
            let tick =
                i64::try_from(world.tick()).expect("world tick exceeds bridge integer range");
            let population = i64::from(world.population());
            output.set("tick", tick);
            output.set("world_time", world.time());
            output.set("seed", world.seed());
            output.set("initial_attempts", i64::from(world.initial_attempts()));
            output.set("paused", world.paused());
            output.set("width", f64::from(width));
            output.set("height", f64::from(height));
            output.set("population", population);
            output.set("oxygen", world.oxygen());
            output.set("carbon_dioxide", world.carbon_dioxide());
            output.set("methane", world.methane());
            output.set("max_velocity", world.physics_parameters().max_velocity);
            output.set("segments", &PackedFloat64Array::from(points.as_slice()));
            output.set("colors", &PackedInt32Array::from(colors.as_slice()));
            output.set("alphas", &PackedByteArray::from(alphas.as_slice()));
            output.set(
                "organism_ids",
                &PackedInt64Array::from(identities.as_slice()),
            );
        }
        output
    }
}

impl BgrSimulation {
    fn start_session_world(
        &mut self,
        seed: i64,
        width: i64,
        height: i64,
        attempts: i64,
    ) -> Result<(), String> {
        if self.world.is_some() {
            return Err(
                "Release the current world before starting another session window".to_owned(),
            );
        }
        let detached = self
            .detached_session
            .take()
            .unwrap_or_else(|| DetachedSession {
                seed,
                session: SessionRef::new(WorldParameters::default(), JavaRandom::new(seed)),
            });
        let config = match configuration(detached.seed, width, height, attempts) {
            Ok(config) => config,
            Err(error) => {
                self.detached_session = Some(detached);
                return Err(error);
            }
        };
        let state = self
            .settings
            .get_or_insert_with(|| biogenesis_core::SettingsState::new("en".to_owned()));
        detached.session.apply_settings_parameters(state);
        match World::with_session(config, detached.session) {
            Ok(world) => {
                self.record_applied_config(config);
                self.network_world_ready(&world);
                self.world = Some(world);
                self.statistics_session = None;
                self.automatic_backup_check();
                Ok(())
            }
            Err(failure) => {
                if failure.entered_genesis {
                    self.record_applied_config(config);
                }
                self.detached_session = Some(DetachedSession {
                    seed: detached.seed,
                    session: failure.session,
                });
                Err(failure.error.to_string())
            }
        }
    }

    fn new_game_world(
        &mut self,
        width: i64,
        height: i64,
        attempts: i64,
    ) -> Result<(), NewGameFailure> {
        if self.automatic_backup.source_pending {
            return Err(NewGameFailure { message: "Finish the pending Source Backup Save As action first".into(), entered_genesis: false });
        }
        self.automatic_backup.finish_pending();
        let Some(world) = self.world.as_mut() else {
            return Err(NewGameFailure {
                message: "Initialize a world first".to_owned(),
                entered_genesis: false,
            });
        };
        let config = configuration(world.seed(), width, height, attempts).map_err(|message| {
            NewGameFailure {
                message,
                entered_genesis: false,
            }
        })?;
        let parameters = world.parameters();
        let result = world.new_game(
            config.width,
            config.height,
            config.initial_attempts,
            parameters,
        );
        let entered_genesis = result
            .as_ref()
            .map_or_else(|failure| failure.entered_genesis, |_| true);
        if entered_genesis {
            self.living_scene.clear();
            self.feeding_scene.clear();
            self.tracked = None;
            self.selected = None;
            self.inspector = None;
            self.preview_genome = None;
        }
        if result.is_ok() {
            world.pause();
        }
        if entered_genesis {
            self.record_applied_config(config);
        }
        if result.is_ok() { self.automatic_backup_check(); }
        result.map_err(|failure| NewGameFailure {
            message: failure.error.to_string(),
            entered_genesis: failure.entered_genesis,
        })
    }

    fn release_world_state(&mut self) {
        self.living_scene.clear();
        self.feeding_scene.clear();
        self.release_network();
        self.automatic_backup.end_world();
        self.statistics_session = None;
        if let Some(world) = self.world.take() {
            self.detached_session = Some(DetachedSession {
                seed: world.seed(),
                session: world.into_session(),
            });
        }
        self.tracked = None;
        self.selected = None;
        self.inspector = None;
        self.clipboard = None;
        self.preview_genome = None;
    }
}

/// Source InfoToolbar retains selection-fixed fields between explicit selections
struct InspectorReadout {
    genotype: Option<InspectorGenotype>,
    reproduce_energy: i32,
    maximum_age: i32,
    energy: f64,
    age: i32,
    mass: f64,
    counters: biogenesis_core::CounterReadout,
}

struct InspectorGenotype {
    generation: i32,
    mutation_rate: i32,
    clade: String,
}

impl InspectorReadout {
    fn new(actor: &OrganismSnapshot, counters: biogenesis_core::CounterReadout) -> Self {
        Self {
            genotype: actor.genotype.as_ref().map(|genotype| InspectorGenotype {
                generation: genotype.generation(),
                mutation_rate: genotype.mutation_rate(),
                clade: selection_clade(genotype),
            }),
            reproduce_energy: actor.state.reproduction.energy,
            maximum_age: actor.state.reproduction.maximum_age,
            energy: actor.state.energy,
            age: actor.state.age >> 8,
            mass: actor.body.mass,
            counters,
        }
    }

    fn recalculate(&mut self, actor: &OrganismSnapshot) {
        self.energy = actor.state.energy;
        self.age = actor.state.age >> 8;
        self.mass = actor.body.mass;
    }
}

fn selection_clade(genotype: &biogenesis_core::Genome) -> String {
    genotype.clade_id()
        .map(biogenesis_core::JavaText::display_utf8_lossy)
        .unwrap_or_else(|| "-1".to_owned())
}

fn recalculate_inspector(
    world: &World,
    selected: Option<&OrganismHandle>,
    readout: &mut Option<InspectorReadout>,
) {
    // Source tests nFrames++ modulo20 before resetting the counter at256
    if world.frame().wrapping_sub(1) % 20 != 0 {
        return;
    }
    let Some(readout) = readout.as_mut() else {
        return;
    };
    let Some(actor) = selected.and_then(|handle| handle.owner_world().inspect_handle(handle)) else {
        return;
    };
    readout.recalculate(&actor);
}

fn selection_fields(
    actor: Option<&OrganismSnapshot>,
    readout: &mut Option<InspectorReadout>,
    world: Option<&World>,
    handle: Option<&OrganismHandle>,
) -> VarDictionary {
    let mut fields = VarDictionary::new();
    let Some(actor) = actor else {
        *readout = None;
        return fields;
    };
    if readout.as_ref().is_none_or(|shown| shown.genotype.is_some() != actor.genotype.is_some()) {
        let counters = world.expect("A selected member has a World")
            .observe_organism_counters(handle.expect("A selected member has a handle"))
            .expect("A selected member can subscribe to its World counters");
        *readout = Some(InspectorReadout::new(actor, counters));
    }
    let shown = readout.as_ref().expect("Selected readout was initialized");
    let counters = shown.counters.snapshot();
    let mut display = VarDictionary::new();
    for (key, value, visible) in [
        ("id", actor.id, actor.id),
        ("age", i64::from(actor.state.age >> 8), i64::from(shown.age)),
        ("maximum_age", i64::from(actor.state.reproduction.maximum_age), i64::from(shown.maximum_age)),
        ("reproduce_energy", i64::from(actor.state.reproduction.energy), i64::from(shown.reproduce_energy)),
        ("children", i64::from(actor.state.total_children), i64::from(counters.children)),
        ("kills", i64::from(actor.state.total_kills), i64::from(counters.kills)),
        ("infections", i64::from(actor.state.total_infected), i64::from(counters.infections)),
    ] {
        fields.set(key, value);
        display.set(key, number_format::integer(visible));
    }
    for (key, value, visible) in [
        ("energy", actor.state.energy, shown.energy),
        ("mass", actor.body.mass, shown.mass),
    ] {
        fields.set(key, value);
        display.set(key, number_format::real(visible));
    }
    fields.set("genotype_available", actor.genotype.is_some());
    if let Some(genotype) = actor.genotype.as_ref() {
        let shown_genotype = shown.genotype.as_ref().expect("Inspector genotype availability was synchronized");
        for (key, value, visible) in [
            ("generation", genotype.generation(), shown_genotype.generation),
            ("mutation_rate", genotype.mutation_rate(), shown_genotype.mutation_rate),
        ] {
            fields.set(key, value);
            display.set(key, number_format::integer(i64::from(visible)));
        }
        fields.set("clade", selection_clade(genotype));
        display.set("clade", shown_genotype.clade.as_str());
    }
    fields.set("display", &display);
    fields.set("age_ticks", actor.state.age);
    fields.set("alive", actor.state.alive);
    fields.set("infected", actor.infected_genotype.is_some());
    fields.set(
        "bounds",
        &PackedInt32Array::from(
            [
                actor.state.bounds.get().x,
                actor.state.bounds.get().y,
                actor.state.bounds.get().width,
                actor.state.bounds.get().height,
            ]
            .as_slice(),
        ),
    );
    fields
}

fn selection_position(x: f64, y: f64) -> Result<[i32; 2], &'static str> {
    let valid = |value: f64| {
        value.is_finite()
            && value.trunc() >= f64::from(i32::MIN)
            && value.trunc() <= f64::from(i32::MAX)
    };
    if !valid(x) || !valid(y) {
        return Err("Selection coordinates must fit signed 32-bit integers");
    }
    Ok([x as i32, y as i32])
}

fn selected_action(name: &str) -> Result<OrganismAction, &'static str> {
    match name {
        "feed" => Ok(OrganismAction::Feed),
        "weaken" => Ok(OrganismAction::Weaken),
        "kill" => Ok(OrganismAction::Kill),
        "revive" => Ok(OrganismAction::Revive),
        "disperse" => Ok(OrganismAction::Disperse),
        "reproduce" => Ok(OrganismAction::Reproduce),
        "rejuvenate" => Ok(OrganismAction::Rejuvenate),
        _ => Err("Unknown organism action"),
    }
}

fn record_tracking(tracked: Option<&OrganismHandle>, path: &mut Vec<i32>) {
    if let Some(center) = tracked.and_then(|handle| handle.owner_world().tracking_center(handle)) {
        path.extend_from_slice(&center);
    }
}

fn advance_world(
    world: &mut World,
    ticks: i64,
    mut should_yield: impl FnMut() -> bool,
    mut after_tick: impl FnMut(&World),
) -> Result<(), WorldError> {
    for _ in 0..ticks {
        if !world.step()? {
            break;
        }
        after_tick(world);
        if should_yield() {
            break;
        }
    }
    Ok(())
}

fn configuration(seed: i64, width: i64, height: i64, attempts: i64) -> Result<WorldConfig, String> {
    let dimension = |value, name| {
        i32::try_from(value)
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| format!("{name} must be a positive 32-bit integer"))
    };
    Ok(WorldConfig {
        seed,
        width: dimension(width, "Width")?,
        height: dimension(height, "Height")?,
        initial_attempts: u32::try_from(attempts)
            .map_err(|_| "Initial attempts must be a nonnegative 32-bit integer".to_owned())?,
    })
}

fn validate_commands(commands: &[i32]) -> Result<(), &'static str> {
    if commands.len() > MAX_COMMANDS {
        return Err("Command batch exceeds 256 actions");
    }
    if commands.iter().any(|command| !(0..=9).contains(command)) {
        return Err("Unknown simulation command");
    }
    Ok(())
}

struct BiogenesisExtension;

#[gdextension(entry_symbol = bgr_init)]
unsafe impl ExtensionLibrary for BiogenesisExtension {}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn session() -> BgrSimulation {
        BgrSimulation {
            world: None,
            detached_session: None,
            selected: None,
            inspector: None,
            tracked: None,
            clipboard: None,
            preview_genome: None,
            settings: None,
            settings_applying: false,
            settings_presentation: None,
            statistics_session: None,
            living_scene: living_scene::LivingSceneState::default(),
            feeding_scene: feeding_scene::FeedingSceneState::default(),
            preferences: preferences::PreferenceSession::default(),
            automatic_backup: automatic_backup::AutomaticBackup::default(),
            network: network::Network::default(),
        }
    }

    pub(crate) fn select_first(session: &mut BgrSimulation) {
        let world = session.world.as_ref().unwrap();
        let id = world.render_segments().unwrap().first().unwrap().organism_id;
        let actor = world.inspect(id).unwrap().unwrap();
        let bounds = actor.state.bounds.get();
        let handle = world
            .pick_organism([bounds.x + bounds.width / 2, bounds.y + bounds.height / 2])
            .unwrap().unwrap();
        session.clipboard = world.copy_genome(&handle).flatten();
        session.preview_genome = Some(session.clipboard.clone());
        session.inspector = Some(InspectorReadout::new(&actor, world.observe_organism_counters(&handle).unwrap()));
        session.selected = Some(handle.clone());
        session.tracked = Some(handle);
    }

    #[test]
    fn preview_copy_preserves_empty_panels_and_copies_exact_identity() {
        let expected: std::collections::HashMap<_, _> =
            include_str!("../tests/fixtures/selected_preview_copy.tsv")
                .lines().map(|line| line.split_once('\t').unwrap()).collect();
        assert_eq!(expected.len(), 8);
        let mut session = session();
        let first = biogenesis_core::GenomeHandle::from_owned(biogenesis_core::Genome::from_parts(
            &[], biogenesis_core::GenomeProperties::default(),
        ));
        let second = biogenesis_core::GenomeHandle::from_owned(biogenesis_core::Genome::from_parts(
            &[], biogenesis_core::GenomeProperties::default(),
        ));
        let identity = |value: &Option<biogenesis_core::GenomeHandle>| match value {
            None => "null",
            Some(value) if value == &first => "first",
            Some(value) if value == &second => "second",
            Some(_) => panic!("Unexpected clipboard identity"),
        };
        session.clipboard = Some(first.clone());
        session.preview_genome = None;
        session.copy_preview_state();
        assert_eq!(identity(&session.clipboard), "first");
        session.preview_genome = Some(None);
        session.copy_preview_state();
        assert_eq!(identity(&session.clipboard), expected["empty-panel-preserves"]);
        session.preview_genome = Some(Some(second.clone()));
        session.copy_preview_state();
        assert_eq!(identity(&session.clipboard), expected["present-panel-copies-exact"]);
        session.preview_genome = Some(None);
        session.copy_preview_state();
        assert_eq!(identity(&session.clipboard), expected["cleared-panel-preserves"]);
        session.clipboard = None;
        session.copy_preview_state();
        assert_eq!(identity(&session.clipboard), expected["empty-panel-empty-clipboard"]);
    }

    #[test]
    fn session_restart_continues_stream_and_explicit_reset_replays_provenance() {
        let mut session = session();
        session.start_session_world(827364, 7000, 7000, 32).unwrap();
        let first = session.world.as_ref().unwrap().render_segments().unwrap();
        session.new_game_world(7000, 7000, 32).unwrap();
        let next = session.world.as_ref().unwrap().render_segments().unwrap();
        assert_ne!(first, next);
        session.world.as_mut().unwrap().reset().unwrap();
        assert_eq!(session.world.as_ref().unwrap().render_segments().unwrap(), first);

        session.release_world_state();
        assert!(session.world.is_none() && session.detached_session.is_some());
        session.start_session_world(-7, 7000, 7000, 32).unwrap();

        assert!(session.detached_session.is_none());
        let world = session.world.as_ref().unwrap();
        assert_eq!(world.seed(), 827364);
        assert_eq!(world.render_segments().unwrap(), next);
    }

    #[test]
    fn session_failed_starts_return_unchanged_or_actually_advanced_streams() {
        let mut session = session();
        assert!(session.start_session_world(827364, 0, 7000, 32).is_err());
        assert!(session.world.is_none());
        assert_eq!(
            session
                .settings
                .as_ref()
                .map(|state| state.genesis())
                .unwrap_or_default(),
            biogenesis_core::GenesisSettings::default()
        );
        let detached = session.detached_session.as_ref().unwrap();
        assert_eq!(detached.seed, 827364);
        assert_eq!(detached.session.random_snapshot(), JavaRandom::new(827364));
        let before = detached.session.random_snapshot();
        assert!(session.start_session_world(1, 1, 1, 1).is_err());
        assert!(session.world.is_none());
        let detached = session.detached_session.as_ref().unwrap();
        assert_eq!(detached.seed, 827364);
        assert_ne!(detached.session.random_snapshot(), before);
        let remaining = detached.session.random_snapshot();
        assert_eq!(
            session.settings.as_ref().unwrap().genesis(),
            biogenesis_core::GenesisSettings {
                width: 1,
                height: 1,
                initial_attempts: 1
            }
        );
        session.release_world_state();
        assert_eq!(
            session.detached_session.as_ref().unwrap().session.random_snapshot(),
            remaining
        );
        session.start_session_world(2, 7000, 7000, 0).unwrap();
        let world = session.world.take().unwrap();
        assert_eq!(world.seed(), 827364);
        assert_eq!(world.into_session().random_snapshot(), remaining);
    }

    #[test]
    fn session_new_game_validation_retains_handles_and_partial_failure_clears_them() {
        let mut session = session();
        session.start_session_world(827364, 7000, 7000, 32).unwrap();
        select_first(&mut session);
        let selected = session.selected.clone();
        let clipboard = session.clipboard.clone();
        session.world.as_mut().unwrap().resume();
        let before = session.world.as_ref().unwrap().render_segments().unwrap();

        let failure = session
            .new_game_world(i64::from(i32::MAX) + 1, 7000, 32)
            .unwrap_err();
        assert!(!failure.entered_genesis);
        assert!(session.selected == selected && session.tracked == selected);
        assert_eq!(session.world.as_ref().unwrap().render_segments().unwrap(), before);
        assert!(!session.world.as_ref().unwrap().paused());

        let failure = session.new_game_world(1, 1, 1).unwrap_err();
        assert!(failure.entered_genesis);
        assert!(
            session.selected.is_none()
                && session.tracked.is_none()
                && session.preview_genome.is_none()
        );
        assert!(session.clipboard == clipboard);
        let world = session.world.as_ref().unwrap();
        assert_eq!(world.dimensions(), (1, 1));
        assert_eq!(world.initial_attempts(), 1);
        assert!(!world.paused());
    }

    #[test]
    fn session_release_drops_world_and_all_frontend_reference_owners() {
        let mut session = session();
        session.start_session_world(827364, 7000, 7000, 32).unwrap();
        select_first(&mut session);
        assert!(session.selected.is_some() && session.tracked.is_some());
        assert!(session.clipboard.is_some() && session.preview_genome.is_some());

        session.release_world_state();

        assert!(session.world.is_none() && session.detached_session.is_some());
        assert!(session.selected.is_none() && session.tracked.is_none());
        assert!(session.clipboard.is_none() && session.preview_genome.is_none());
        session.release_world_state();
        assert!(session.detached_session.is_some());
    }

    #[test]
    fn session_genesis_parameter_error_reports_mutation_phase() {
        let mut parameters = WorldParameters::default();
        parameters.genetics.initial_complexity = i32::MAX;
        parameters.genetics.validate().unwrap();
        let mut session = session();
        session.world = Some(
            World::with_parameters(
                WorldConfig {
                    seed: 827364,
                    width: 7000,
                    height: 7000,
                    initial_attempts: 0,
                },
                parameters,
            )
            .unwrap(),
        );
        let world = session.world.as_mut().unwrap();
        assert!(world.single_step().unwrap());
        world.resume();

        // This seed rejects the wrapped negative gene count before allocation
        let failure = session.new_game_world(7000, 7000, 1).unwrap_err();

        assert!(failure.entered_genesis);
        let world = session.world.take().unwrap();
        assert_eq!(world.tick(), 0);
        assert_eq!(world.initial_attempts(), 1);
        assert!(!world.paused());
        assert_ne!(world.into_session().random_snapshot(), JavaRandom::new(827364));
    }

    #[test]
    fn settings_survive_stopped_session_and_supply_actual_future_genesis() {
        use biogenesis_core::{SettingKey, SettingsDraft, SettingsState};
        let mut session = session();
        session.start_session_world(827364, 7000, 7000, 0).unwrap();
        let mut state = SettingsState::new("en".to_owned());
        let mut draft = SettingsDraft::new(&state);
        draft
            .set_editor(SettingKey::BlueEnergyConsumption, "0.125".to_owned())
            .unwrap();
        draft
            .set_editor(SettingKey::InitialO2, "1234".to_owned())
            .unwrap();
        draft
            .ok(&mut state, |_| panic!("Locale unchanged"))
            .unwrap();
        session
            .world
            .as_mut()
            .unwrap()
            .apply_settings_parameters(&state);
        let before_random = session.world.as_ref().unwrap().clone().random_snapshot();
        let escaped_session = session.world.as_ref().unwrap().session();
        session.settings = Some(state.clone());
        session.release_world_state();
        assert_eq!(session.settings.as_ref(), Some(&state));
        let mut draft = SettingsDraft::new(&state);
        draft
            .set_editor(SettingKey::DrainSubsDivisor, "17".to_owned())
            .unwrap();
        draft
            .ok(&mut state, |_| panic!("Locale unchanged"))
            .unwrap();
        session.apply_runtime_settings(&state);
        session.settings = Some(state);
        assert_eq!(escaped_session.parameters().atmosphere.drain_divisor, 17);
        assert_eq!(escaped_session.random_snapshot(), before_random);
        assert_eq!(
            session
                .detached_session
                .as_ref()
                .unwrap()
                .session
                .parameters()
                .atmosphere
                .drain_divisor,
            17
        );
        session.start_session_world(999, 7000, 7000, 0).unwrap();
        let world = session.world.as_ref().unwrap();
        assert_eq!(world.seed(), 827364);
        assert_eq!(world.clone().random_snapshot(), before_random);
        assert_eq!(world.oxygen(), 1234.0);
        assert_eq!(world.parameters().lifecycle.blue_energy_consumption, 0.125);
    }

    #[test]
    fn projected_partial_bounds_preserve_bridge_owners_before_rejected_new_game() {
        use biogenesis_core::{SettingKey, SettingsDraft, SettingsState};
        let mut session = session();
        session.start_session_world(827364, 7000, 7000, 1).unwrap();
        select_first(&mut session);
        let mut state = SettingsState::new("en".to_owned());
        for (max, min) in [("7", "7"), ("3", "6")] {
            let mut draft = SettingsDraft::new(&state);
            draft
                .set_editor(SettingKey::MaxMutationRate, max.to_owned())
                .unwrap();
            draft
                .set_editor(SettingKey::MinMutationRate, min.to_owned())
                .unwrap();
            draft
                .ok(&mut state, |_| panic!("Locale unchanged"))
                .unwrap();
        }
        session
            .world
            .as_mut()
            .unwrap()
            .apply_settings_parameters(&state);
        session.settings = Some(state);
        let pending = session.settings.as_ref().unwrap().genesis();
        let before = session.world.as_ref().unwrap().clone();
        let selected = session.selected.clone();
        let tracked = session.tracked.clone();
        let clipboard = session.clipboard.clone();
        let failure = session.new_game_world(1234, 2345, 0).unwrap_err();
        assert!(!failure.entered_genesis);
        assert_eq!(session.settings.as_ref().unwrap().genesis(), pending);
        assert_eq!(session.world.as_ref().unwrap(), &before);
        assert_eq!(session.selected, selected);
        assert_eq!(session.tracked, tracked);
        assert_eq!(session.clipboard, clipboard);
        assert_eq!(
            session
                .settings
                .as_ref()
                .unwrap()
                .parameters()
                .genetics
                .min_mutation_rate,
            7
        );
    }

    #[test]
    fn yielded_batches_preserve_exact_completed_ticks() {
        let mut world = World::new(configuration(827364, 1200, 800, 96).unwrap()).unwrap();
        world.resume();
        let expected = world.clone();
        let mut completed = 0;
        advance_world(
            &mut world,
            16,
            || {
                completed += 1;
                completed == 4
            },
            |_| {},
        )
        .unwrap();
        assert_eq!(completed, 4);
        for _ in 0..4 {
            assert!(expected.step().unwrap());
        }
        assert_eq!(world, expected);
        advance_world(&mut world, 12, || false, |_| {}).unwrap();
        for _ in 0..12 {
            assert!(expected.step().unwrap());
        }
        assert_eq!(world, expected);
    }

    #[test]
    fn paused_and_empty_batches_never_consult_the_deadline() {
        let mut world = World::new(configuration(827364, 1200, 800, 96).unwrap()).unwrap();
        let before = world.clone();
        advance_world(
            &mut world,
            16,
            || panic!("Paused world cannot complete a tick"),
            |_| panic!("Paused world cannot move the tracking camera"),
        )
        .unwrap();
        assert_eq!(world, before);
        world.resume();
        let before = world.clone();
        advance_world(
            &mut world,
            0,
            || panic!("Empty batch cannot complete a tick"),
            |_| panic!("Empty batch cannot move the tracking camera"),
        )
        .unwrap();
        assert_eq!(world, before);
    }

    #[test]
    fn positional_selection_preserves_empty_policy_and_successful_handle_identity() {
        let mut simulation = session();
        simulation.start_session_world(827364, 1200, 800, 8).unwrap();
        select_first(&mut simulation);
        let selected = simulation.selected.clone();
        let clipboard = simulation.clipboard.clone();
        let world = simulation.world.as_ref().unwrap();
        let owner = world.handle();
        let random = world.random_snapshot();
        let bounds = world.inspect_handle(selected.as_ref().unwrap()).unwrap().state.bounds.get();
        let point = [bounds.x + bounds.width / 2, bounds.y + bounds.height / 2];
        assert!(!simulation.select_position_state([i32::MAX, i32::MAX], true).unwrap());
        assert_eq!(simulation.selected, selected);
        assert!(!simulation.select_position_state([i32::MAX, i32::MAX], false).unwrap());
        assert!(simulation.selected.is_none());
        assert!(simulation.select_position_state(point, false).unwrap());
        assert_eq!(simulation.selected, selected);
        assert_eq!(simulation.clipboard, clipboard);
        let world = simulation.world.as_ref().unwrap();
        assert!(world.handle().same_owner(&owner));
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn positional_selection_propagates_reached_membership_error_without_model_mutation() {
        let mut simulation = session();
        simulation.start_session_world(827364, 1200, 800, 0).unwrap();
        let limits = biogenesis_core::object_stream::Limits::default();
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../core/tests/fixtures/shared_world_owners/member_wrapper_backing-call_0.ser");
        let world = simulation.world.as_ref().unwrap()
            .load_legacy_file(&fixture, limits, limits.allocated_bytes).unwrap();
        let actor = world.inspect(0).unwrap().unwrap();
        let bounds = actor.state.bounds.get();
        let point = [bounds.x + bounds.width / 2, bounds.y + bounds.height / 2];
        let selected = world.pick_organism(point).unwrap().unwrap();
        let miss = [i32::MAX, i32::MAX];
        let original_error = world.pick_organism(miss).unwrap_err().to_string();
        let owner = world.handle();
        let random = world.random_snapshot();
        let clock = (world.tick(), world.time(), world.paused());
        let gases = [world.oxygen().to_bits(), world.carbon_dioxide().to_bits(), world.methane().to_bits()];
        simulation.world = Some(world);
        simulation.selected = Some(selected.clone());
        simulation.tracked = Some(selected.clone());
        for preserve_empty in [false, true] {
            let error = simulation.select_position_state(miss, preserve_empty).unwrap_err();
            assert_eq!(error, original_error);
            assert_eq!(simulation.selected.as_ref(), Some(&selected));
            assert_eq!(simulation.tracked.as_ref(), Some(&selected));
            let world = simulation.world.as_ref().unwrap();
            assert!(world.handle().same_owner(&owner));
            assert_eq!(world.random_snapshot(), random);
            assert_eq!((world.tick(), world.time(), world.paused()), clock);
            assert_eq!([world.oxygen().to_bits(), world.carbon_dioxide().to_bits(), world.methane().to_bits()], gases);
        }
    }

    #[test]
    fn tracking_records_every_completed_tick_and_rejects_foreign_or_dead_handles() {
        let mut world = World::new(configuration(827364, 1200, 800, 96).unwrap()).unwrap();
        let bounds = world.inspect(0).unwrap().unwrap().state.bounds.get();
        let position = [bounds.x, bounds.y];
        let tracked = world.pick_organism(position).unwrap().unwrap();
        let expected = world.clone();
        let expected_handle = expected.pick_organism(position).unwrap().unwrap();
        assert!(expected.tracking_center(&tracked).is_none());
        world.resume();
        expected.resume();
        let mut path = Vec::new();
        advance_world(
            &mut world,
            8,
            || false,
            |_| {
                record_tracking(Some(&tracked), &mut path);
            },
        )
        .unwrap();
        let mut expected_path = Vec::new();
        for _ in 0..8 {
            expected.step().unwrap();
            let actor = expected.inspect_handle(&expected_handle).unwrap();
            if actor.state.alive {
                expected_path.extend_from_slice(&actor.state.integer_center);
            }
        }
        assert_eq!(path, expected_path);
        assert_eq!(world, expected);
        world
            .act_on_organism(&tracked, OrganismAction::Kill)
            .unwrap();
        assert!(world.tracking_center(&tracked).is_none());
        let before = path.clone();
        record_tracking(Some(&tracked), &mut path);
        assert_eq!(path, before);
    }

    #[test]
    fn configuration_rejects_integer_wrap_and_zero_dimensions() {
        for (width, height, attempts) in [
            (0, 400, 32),
            (-1, 400, 32),
            (400, 0, 32),
            (400, 400, -1),
            (i64::MAX, 400, 32),
            (i64::from(i32::MAX) + 1, 400, 32),
            (400, i64::from(i32::MAX) + 1, 32),
        ] {
            assert!(configuration(827364, width, height, attempts).is_err());
        }
        let config = configuration(i64::MIN, 512, 400, 0).unwrap();
        assert_eq!(config.seed, i64::MIN);
        assert_eq!(config.initial_attempts, 0);
        let maximum = configuration(0, i64::from(i32::MAX), 1, 0).unwrap();
        assert_eq!(maximum.width, i32::MAX);
    }

    #[test]
    fn command_validation_preserves_distinct_actions_and_rejects_bad_batches() {
        assert!(validate_commands(&[0, 2, 2, 1, 0, 3]).is_ok());
        assert!(validate_commands(&[]).is_ok());
        assert!(validate_commands(&[4, 5, 6, 7, 8, 9]).is_ok());
        assert!(validate_commands(&[0, 10, 1]).is_err());
        assert!(validate_commands(&[-1]).is_err());
        assert!(validate_commands(&vec![0; MAX_COMMANDS + 1]).is_err());
    }

    #[test]
    fn pointer_coordinates_keep_java_truncation_without_wrap_or_nonfinite_inputs() {
        assert_eq!(selection_position(-0.9, -1.9).unwrap(), [0, -1]);
        assert_eq!(
            selection_position(f64::from(i32::MIN), f64::from(i32::MAX)).unwrap(),
            [i32::MIN, i32::MAX]
        );
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            2147483648.0,
            -2147483649.0,
        ] {
            assert!(selection_position(value, 0.0).is_err());
            assert!(selection_position(0.0, value).is_err());
        }
        assert!(selected_action("feed").is_ok());
        assert!(selected_action("Feed").is_err());
    }

    #[test]
    fn statistics_formatting_preserves_ties_grouping_signed_zero_and_large_integers() {
        for (bits, expected) in [
            (0x0_u64, "0"),
            (0x8000000000000000_u64, "-0"),
            (0x4002000000000000_u64, "2.2"),
            (0x4006000000000000_u64, "2.8"),
            (0x408f3fae147ae148_u64, "1,000"),
            (0xc0c81ca000000000_u64, "-12,345.2"),
            (0x402819999999999a_u64, "12.1"),
            (0x7ff8000000000000_u64, "NaN"),
            (0x7ff0000000000000_u64, "\u{221e}"),
            (0xfff0000000000000_u64, "-\u{221e}"),
        ] {
            assert_eq!(number_format::for_tag("en-US").real(f64::from_bits(bits)), expected);
        }
        for (value, expected) in [
            (0, "0"),
            (-1, "-1"),
            (1234567, "1,234,567"),
            (i64::MIN, "-9,223,372,036,854,775,808"),
            (i64::MAX, "9,223,372,036,854,775,807"),
        ] {
            assert_eq!(number_format::for_tag("en-US").integer(value), expected);
        }
    }

    #[test]
    fn statistics_session_keeps_same_world_owner_and_releases_it_on_stop() {
        let mut session = session();
        session.start_session_world(827364, 1200, 800, 0).unwrap();
        session.open_statistics_state().unwrap();
        let retained = session.statistics_session.as_ref().unwrap().handle.clone();
        session.new_game_world(1200, 800, 0).unwrap();
        assert_eq!(session.statistics_session.as_ref().unwrap().handle, retained);
        assert_ne!(session.world.as_ref().unwrap().open_statistics(), retained);
        session.update_statistics_state().unwrap();
        assert_eq!(session.statistics_session.as_ref().unwrap().handle, retained);
        session.release_world_state();
        assert!(session.statistics_session.is_none());
        assert!(session.world.is_none() && session.detached_session.is_some());
    }

    #[test]
    fn statistics_record_copy_preserves_identity_and_null_or_rejected_copy_retains_clipboard() {
        use biogenesis_core::{ColorKind, Gene, Genome, GenomeHandle, GenomeProperties};

        let mut session = session();
        session.start_session_world(827364, 1200, 800, 0).unwrap();
        let genotype = GenomeHandle::from_owned(Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties { symmetry: 1, mutation_rate: 0, ..GenomeProperties::default() },
        ));
        let parent = session.world.as_mut().unwrap().paste_genome_handle(&genotype, [600, 400])
            .unwrap().unwrap();
        session.clipboard = Some(genotype.clone());
        session.open_statistics_state().unwrap();
        session.copy_statistics_record(0).unwrap();
        assert_eq!(session.clipboard, Some(genotype.clone()));
        let world = session.world.as_mut().unwrap();
        assert!(world.act_on_organism(&parent, OrganismAction::Feed).unwrap());
        assert!(world.act_on_organism(&parent, OrganismAction::Reproduce).unwrap());
        assert!(world.inspect_handle(&parent).unwrap().state.total_children > 0);
        session.update_statistics_state().unwrap();
        session.clipboard = None;
        session.copy_statistics_record(0).unwrap();
        assert_eq!(session.clipboard, Some(genotype.clone()));
        session.copy_statistics_record(2).unwrap();
        assert_eq!(session.clipboard, Some(genotype.clone()));
        assert!(session.copy_statistics_record(6).is_err());
        assert_eq!(session.clipboard, Some(genotype));
    }

    #[test]
    fn inspector_recalculation_preserves_selection_fixed_fields() {
        let mut session = session();
        session.start_session_world(827364, 1200, 800, 16).unwrap();
        select_first(&mut session);
        let mut actor = session.world.as_ref().unwrap()
            .inspect_handle(session.selected.as_ref().unwrap()).unwrap();
        let mut shown = InspectorReadout::new(&actor, session.world.as_ref().unwrap()
            .observe_organism_counters(session.selected.as_ref().unwrap()).unwrap());
        let original = (
            shown.reproduce_energy,
            shown.maximum_age,
            shown.genotype.as_ref().map(|g| (g.generation, g.mutation_rate, g.clade.clone())),
        );
        actor.state.energy = -0.0;
        actor.state.age = -257;
        actor.body.mass = 1234.5;
        actor.state.reproduction.energy = i32::MIN;
        actor.state.reproduction.maximum_age = i32::MAX;
        actor.genotype = Some(biogenesis_core::Genome::from_parts(
            &[], biogenesis_core::GenomeProperties::default(),
        ));
        shown.recalculate(&actor);
        assert_eq!(shown.energy.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(shown.age, -2);
        assert_eq!(shown.mass, 1234.5);
        assert_eq!(
            (shown.reproduce_energy, shown.maximum_age,
             shown.genotype.as_ref().map(|g| (g.generation, g.mutation_rate, g.clade.clone()))),
            original,
        );
    }

}
