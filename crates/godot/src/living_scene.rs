// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{LivingOrganismSummary, OrganismImage, RecordKind, WorldHandle};
use godot::builtin::VarArray;

const MAX_SUBJECTS: usize = 8;
const PREVIEW_PIXELS: usize = 100 * 100 * 3;
const PREVIEW_BUDGET: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
enum Metric {
    Offspring,
    Kills,
    Age,
    Mass,
    Infections,
}

const METRICS: [Metric; 5] = [
    Metric::Offspring,
    Metric::Kills,
    Metric::Age,
    Metric::Mass,
    Metric::Infections,
];

impl Metric {
    fn name(self) -> &'static str {
        match self {
            Self::Offspring => "offspring",
            Self::Kills => "kills",
            Self::Age => "age",
            Self::Mass => "mass",
            Self::Infections => "infections",
        }
    }

    fn unit(self) -> &'static str {
        match self {
            Self::Age => "biological_ticks",
            Self::Mass => "source_mass",
            _ => "count",
        }
    }

    fn integer(self, actor: &LivingOrganismSummary) -> i64 {
        i64::from(match self {
            Self::Offspring => actor.children,
            Self::Kills => actor.kills,
            Self::Age => actor.age_ticks,
            Self::Infections => actor.infections,
            Self::Mass => unreachable!("Mass is a real-valued metric"),
        })
    }

    fn eligible(self, actor: &LivingOrganismSummary) -> bool {
        match self {
            Self::Offspring | Self::Kills | Self::Infections => self.integer(actor) > 0,
            Self::Age | Self::Mass => true,
        }
    }

    fn exceeds(self, actor: &LivingOrganismSummary, previous: &LivingOrganismSummary) -> bool {
        match self {
            Self::Mass => actor.mass > previous.mass,
            _ => self.integer(actor) > self.integer(previous),
        }
    }
}

struct TokenSlot {
    token: i64,
    handle: OrganismHandle,
}

#[derive(Default)]
pub(super) struct LivingSceneState {
    owner: Option<WorldHandle>,
    slots: Vec<TokenSlot>,
    next_token: i64,
    epoch: i64,
    last_tick: Option<u64>,
}

struct Subject {
    token: i64,
    actor: LivingOrganismSummary,
    preview: Option<Result<OrganismImage, String>>,
}

struct ScenePacket {
    epoch: i64,
    world_ready: bool,
    tick: i64,
    world_time: i64,
    subjects: Vec<Subject>,
    records: [Option<usize>; 5],
    follow: Option<usize>,
    fallback: Option<usize>,
}

impl LivingSceneState {
    #[cfg(test)]
    pub(super) fn cache_for_test(&mut self, world: &World) {
        self.collect(Some(world), 0, false).unwrap();
        assert!(self.owner.is_some() && !self.slots.is_empty());
    }

    #[cfg(test)]
    pub(super) fn is_empty_for_test(&self) -> bool {
        self.owner.is_none() && self.slots.is_empty() && self.last_tick.is_none()
    }

    pub(super) fn clear(&mut self) {
        self.owner = None;
        self.slots.clear();
        self.last_tick = None;
        self.epoch = self.epoch.saturating_add(1);
    }

    fn collect(
        &mut self,
        world: Option<&World>,
        follow_token: i64,
        previews: bool,
    ) -> Result<ScenePacket, String> {
        if follow_token < 0 {
            return Err("Follow token cannot be negative".into());
        }
        let Some(world) = world else {
            if self.owner.is_some() || !self.slots.is_empty() {
                self.clear();
            }
            return Ok(ScenePacket {
                epoch: self.epoch,
                world_ready: false,
                tick: 0,
                world_time: 0,
                subjects: Vec::new(),
                records: [None; 5],
                follow: None,
                fallback: None,
            });
        };
        let owner = world.handle();
        let same_owner = self.owner.as_ref() == Some(&owner)
            && self.last_tick.is_none_or(|tick| world.tick() >= tick);
        let previous = if same_owner {
            self.slots.as_slice()
        } else {
            &[]
        };
        let requested = previous
            .iter()
            .find(|slot| slot.token == follow_token)
            .map(|slot| slot.handle.clone());
        let mut winners: [Option<LivingOrganismSummary>; 5] = std::array::from_fn(|_| None);
        let mut first = None;
        let mut following = None;
        let mut invalid = None;
        world
            .visit_living_organisms(previews, |actor| {
                if !actor.mass.is_finite()
                    || !actor.energy.is_finite()
                    || actor
                        .center
                        .iter()
                        .chain(actor.velocity.iter())
                        .any(|value| !value.is_finite())
                {
                    invalid.get_or_insert_with(|| {
                        format!(
                            "Living organism {} has a nonfinite presentation value",
                            actor.id
                        )
                    });
                    return;
                }
                if first.is_none() {
                    first = Some(actor.clone());
                }
                if requested.as_ref() == Some(&actor.handle) {
                    following = Some(actor.clone());
                }
                for (index, metric) in METRICS.iter().copied().enumerate() {
                    if metric.eligible(&actor)
                        && winners[index]
                            .as_ref()
                            .is_none_or(|old| metric.exceeds(&actor, old))
                    {
                        winners[index] = Some(actor.clone());
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        if let Some(error) = invalid {
            return Err(error);
        }
        let mut actors = Vec::with_capacity(MAX_SUBJECTS);
        let mut insert = |actor: &LivingOrganismSummary| -> usize {
            if let Some(index) = actors
                .iter()
                .position(|old: &LivingOrganismSummary| old.handle == actor.handle)
            {
                index
            } else {
                actors.push(actor.clone());
                actors.len() - 1
            }
        };
        let records = std::array::from_fn(|index| winners[index].as_ref().map(&mut insert));
        let follow = following.as_ref().map(&mut insert);
        let fallback = first.as_ref().map(&mut insert);
        if actors.len() > MAX_SUBJECTS {
            return Err("Living presentation exceeds eight unique subjects".into());
        }
        let preview_budget = PREVIEW_BUDGET - actors.len().saturating_sub(1) * PREVIEW_PIXELS;
        let mut next_token = self.next_token;
        let mut subjects = Vec::with_capacity(actors.len());
        let mut slots = Vec::with_capacity(actors.len());
        for actor in actors {
            let token = match previous.iter().find(|slot| slot.handle == actor.handle) {
                Some(slot) => slot.token,
                None => {
                    next_token = next_token
                        .checked_add(1)
                        .ok_or("Presentation token space exhausted")?;
                    next_token
                }
            };
            let preview = previews.then(|| {
                actor
                    .preview_genome
                    .as_ref()
                    .ok_or_else(|| "Primary genotype unavailable".to_owned())
                    .and_then(|genome| {
                        genome
                            .preview_image(100, 100, preview_budget)
                            .map_err(|error| error.to_string())
                    })
            });
            slots.push(TokenSlot {
                token,
                handle: actor.handle.clone(),
            });
            subjects.push(Subject {
                token,
                actor,
                preview,
            });
        }
        let tick =
            i64::try_from(world.tick()).map_err(|_| "World tick exceeds bridge integer range")?;
        if !same_owner {
            self.epoch = self.epoch.saturating_add(1);
        }
        self.owner = Some(owner);
        self.slots = slots;
        self.next_token = next_token;
        self.last_tick = Some(world.tick());
        Ok(ScenePacket {
            epoch: self.epoch,
            world_ready: true,
            tick,
            world_time: world.time(),
            subjects,
            records,
            follow,
            fallback,
        })
    }
}

impl ScenePacket {
    fn dictionary(self) -> VarDictionary {
        let mut subjects = VarArray::new();
        for subject in &self.subjects {
            let actor = &subject.actor;
            let mut item = VarDictionary::new();
            item.set("token", subject.token);
            item.set("id", actor.id);
            item.set("alive", true);
            item.set("center", &PackedFloat64Array::from(actor.center.as_slice()));
            item.set(
                "bounds",
                &PackedInt32Array::from(
                    [
                        actor.bounds.x,
                        actor.bounds.y,
                        actor.bounds.width,
                        actor.bounds.height,
                    ]
                    .as_slice(),
                ),
            );
            item.set(
                "velocity",
                &PackedFloat64Array::from(actor.velocity.as_slice()),
            );
            item.set("age_ticks", actor.age_ticks);
            item.set("age_world_units", actor.age_ticks >> 8);
            item.set("energy", actor.energy);
            item.set("mass", actor.mass);
            item.set("children", actor.children);
            item.set("kills", actor.kills);
            item.set("infections", actor.infections);
            item.set("consumer", actor.consumer);
            item.set("killer", actor.killer);
            item.set(
                "contact_tint_argb",
                &actor
                    .temporary_color
                    .map(|value| i64::from(value).to_variant())
                    .unwrap_or_else(Variant::nil),
            );
            item.set("contact_tint_frames", actor.temporary_color_frames);
            item.set("genotype_available", actor.genotype_available);
            let mut preview = VarDictionary::new();
            preview.set("requested", subject.preview.is_some());
            preview.set("error", "");
            preview.set("image", &VarDictionary::new());
            if let Some(image) = &subject.preview {
                match image {
                    Ok(image) => {
                        let mut fields = VarDictionary::new();
                        fields.set("width", i64::from(image.width));
                        fields.set("height", i64::from(image.height));
                        fields.set("rgb", &PackedByteArray::from(image.rgb.as_slice()));
                        preview.set("image", &fields);
                    }
                    Err(error) => preview.set("error", error.as_str()),
                };
            }
            item.set("preview", &preview);
            subjects.push(&item.to_variant());
        }
        let mut records = VarArray::new();
        for (index, metric) in METRICS.iter().copied().enumerate() {
            let subject = self.records[index].map(|index| &self.subjects[index]);
            let mut record = VarDictionary::new();
            record.set("kind", metric.name());
            record.set("unit", metric.unit());
            record.set("present", subject.is_some());
            record.set("subject_token", subject.map_or(0, |subject| subject.token));
            match metric {
                Metric::Mass => {
                    record.set("value", subject.map_or(0.0, |subject| subject.actor.mass))
                }
                _ => record.set(
                    "value",
                    subject.map_or(0, |subject| metric.integer(&subject.actor)),
                ),
            };
            records.push(&record.to_variant());
        }
        let mut units = VarDictionary::new();
        units.set("center", "world_coordinate");
        units.set("bounds", "world_coordinate");
        units.set("velocity", "world_coordinate_per_biological_tick");
        units.set("energy", "source_energy");
        units.set("mass", "source_mass");
        units.set("age_ticks", "biological_ticks");
        units.set("age_world_units", "256_biological_ticks");
        let mut output = VarDictionary::new();
        output.set("schema", 1);
        output.set("ok", true);
        output.set("error", "");
        output.set("world_ready", self.world_ready);
        output.set("presentation_epoch", self.epoch);
        output.set("tick", self.tick);
        output.set("world_time", self.world_time);
        output.set("subjects", &subjects);
        output.set("records", &records);
        output.set(
            "follow_token",
            self.follow.map_or(0, |index| self.subjects[index].token),
        );
        output.set(
            "fallback_token",
            self.fallback.map_or(0, |index| self.subjects[index].token),
        );
        output.set("units", &units);
        output
    }
}

#[godot_api(secondary)]
impl BgrSimulation {
    #[func]
    fn living_scene_snapshot(
        &mut self,
        follow_token: i64,
        include_previews: bool,
    ) -> VarDictionary {
        match self.collect_living_scene(follow_token, include_previews) {
            Ok(packet) => {
                let mut output = packet.dictionary();
                output.set("feeding", &self.feeding_dictionary());
                output
            },
            Err(error) => {
                let mut output = VarDictionary::new();
                output.set("schema", 1);
                output.set("ok", false);
                output.set("error", error.as_str());
                output.set("world_ready", self.world.is_some());
                output
            }
        }
    }

    #[func]
    fn living_scene_snapshot_with_records(
        &mut self,
        follow_token: i64,
        include_live_previews: bool,
    ) -> VarDictionary {
        let mut output = self.living_scene_snapshot(follow_token, include_live_previews);
        if !output.at("ok").to::<bool>() {
            return output;
        }

        let mut historical = VarArray::new();
        if let Some(world) = self.world.as_ref() {
            let records = world.all_time_records();
            let live_images = if include_live_previews { MAX_SUBJECTS } else { 0 };
            // Reserve the other retained RGB8 images while rasterizing one record
            let budget = PREVIEW_BUDGET - (live_images + records.len() - 1) * PREVIEW_PIXELS;
            for record in records {
                let metric = match record.kind {
                    RecordKind::HistoricalChildren => Metric::Offspring,
                    RecordKind::HistoricalKills => Metric::Kills,
                    RecordKind::HistoricalInfections => Metric::Infections,
                    _ => unreachable!("All-time records contain only historical record kinds"),
                };
                let mut item = VarDictionary::new();
                item.set("kind", metric.name());
                item.set("count", record.count);
                item.set("time", record.time.expect("Historical record time is retained"));
                item.set("present", record.genome.is_some());
                let mut preview = VarDictionary::new();
                preview.set("requested", record.genome.is_some());
                preview.set("error", "");
                preview.set("image", &VarDictionary::new());
                if let Some(genome) = record.genome {
                    match genome.preview_image(100, 100, budget) {
                        Ok(image) => preview.set("image", &statistics::image_fields(image)),
                        Err(error) => preview.set("error", error.to_string()),
                    };
                }
                item.set("preview", &preview);
                historical.push(&item.to_variant());
            }
        }
        output.set("all_time_records", &historical);
        output
    }
}

impl BgrSimulation {
    fn collect_living_scene(
        &mut self,
        follow_token: i64,
        previews: bool,
    ) -> Result<ScenePacket, String> {
        self.living_scene
            .collect(self.world.as_ref(), follow_token, previews)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biogenesis_core::{ColorKind, Gene, Genome, GenomeHandle, GenomeProperties};

    fn world() -> World {
        World::new(WorldConfig {
            seed: 827364,
            width: 1200,
            height: 800,
            initial_attempts: 0,
        })
        .unwrap()
    }

    fn paste(world: &World, position: [i32; 2], genome: &GenomeHandle) -> OrganismHandle {
        world
            .paste_genome_handle(genome, position)
            .unwrap()
            .unwrap()
    }

    fn genome() -> GenomeHandle {
        GenomeHandle::from_owned(Genome::from_parts(
            &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                mutation_rate: 0,
                ..GenomeProperties::default()
            },
        ))
    }

    #[test]
    fn empty_world_has_no_invented_champions_or_follow_target() {
        let world = world();
        let mut state = LivingSceneState::default();
        let packet = state.collect(Some(&world), 0, false).unwrap();
        assert!(packet.world_ready && packet.subjects.is_empty());
        assert_eq!(packet.records, [None; 5]);
        assert!(packet.follow.is_none() && packet.fallback.is_none());
        assert!(state.collect(None, 0, false).unwrap().subjects.is_empty());
    }

    #[test]
    fn stable_ties_use_first_living_membership_and_deduplicate_record_subjects() {
        let world = world();
        let code = genome();
        let first = paste(&world, [200, 200], &code);
        paste(&world, [500, 400], &code);
        let mut state = LivingSceneState::default();
        let packet = state.collect(Some(&world), 0, false).unwrap();
        assert_eq!(packet.records[0], None);
        assert_eq!(packet.records[1], None);
        assert_eq!(packet.records[4], None);
        let age = packet.records[2].unwrap();
        let mass = packet.records[3].unwrap();
        assert_eq!(age, mass);
        assert_eq!(packet.subjects[age].actor.handle, first);
        assert_eq!(packet.subjects.len(), 1);
    }

    #[test]
    fn dead_and_removed_tokens_do_not_follow_retained_actors() {
        let mut world = world();
        let first = paste(&world, [200, 200], &genome());
        let mut state = LivingSceneState::default();
        let token = state.collect(Some(&world), 0, false).unwrap().subjects[0].token;
        world.act_on_organism(&first, OrganismAction::Kill).unwrap();
        let dead = state.collect(Some(&world), token, false).unwrap();
        assert!(dead.follow.is_none() && dead.subjects.is_empty() && state.slots.is_empty());
        paste(&world, [500, 400], &genome());
        let live = state.collect(Some(&world), 0, false).unwrap();
        let live_token = live.subjects[0].token;
        world.reset().unwrap();
        let reset = state.collect(Some(&world), live_token, false).unwrap();
        assert!(reset.follow.is_none());
        assert!(
            reset
                .subjects
                .iter()
                .all(|subject| subject.token != live_token)
        );
    }

    #[test]
    fn world_replacement_rejects_colliding_numeric_ids_and_old_tokens() {
        let world = world();
        paste(&world, [200, 200], &genome());
        let mut state = LivingSceneState::default();
        let first = state.collect(Some(&world), 0, false).unwrap();
        let copied = world.clone();
        let next = state
            .collect(Some(&copied), first.subjects[0].token, false)
            .unwrap();
        assert_eq!(next.subjects[0].actor.id, first.subjects[0].actor.id);
        assert_ne!(next.subjects[0].token, first.subjects[0].token);
        assert!(next.follow.is_none());
        assert!(next.epoch > first.epoch);
    }

    #[test]
    fn requests_and_previews_preserve_rng_save_bytes_and_inspector_ownership() {
        let mut simulation = crate::tests::session();
        let world = world();
        paste(&world, [200, 200], &genome());
        simulation.world = Some(world);
        crate::tests::select_first(&mut simulation);
        let world = simulation.world.as_ref().unwrap();
        let limits = biogenesis_core::object_stream::Limits::default();
        let before = world
            .encode_legacy_file_bytes(limits, limits.allocated_bytes)
            .unwrap();
        let random = world.random_snapshot();
        let selected = simulation.selected.clone();
        let tracked = simulation.tracked.clone();
        let clipboard = simulation.clipboard.clone();
        let counters = simulation.inspector.as_ref().unwrap().counters.clone();
        let shown = counters.snapshot();
        for previews in [false, true, false] {
            let packet = simulation.collect_living_scene(0, previews).unwrap();
            assert!(packet.subjects.len() <= MAX_SUBJECTS);
        }
        let world = simulation.world.as_ref().unwrap();
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(
            world
                .encode_legacy_file_bytes(limits, limits.allocated_bytes)
                .unwrap(),
            before
        );
        assert_eq!(simulation.selected, selected);
        assert_eq!(simulation.tracked, tracked);
        assert_eq!(simulation.clipboard, clipboard);
        assert_eq!(counters.snapshot(), shown);
        world
            .act_on_organism(selected.as_ref().unwrap(), OrganismAction::Reproduce)
            .unwrap();
        assert!(counters.snapshot().children > shown.children);
    }
}
