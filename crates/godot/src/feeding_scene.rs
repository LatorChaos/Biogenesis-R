// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{FeedingReadout, FeedingStamp, FeedingSubject};

#[derive(Default)]
pub(super) struct FeedingSceneState {
    readout: Option<FeedingReadout>,
    epoch: i64,
}

impl FeedingSceneState {
    pub(super) fn clear(&mut self) {
        self.readout = None;
        self.epoch = self.epoch.saturating_add(1);
    }

    fn ensure(&mut self, world: Option<&World>) {
        let Some(world) = world else {
            if self.readout.is_some() {
                self.clear();
            }
            return;
        };
        if self
            .readout
            .as_ref()
            .is_none_or(|readout| !readout.is_current_for(world))
        {
            self.epoch = self.epoch.saturating_add(1);
            self.readout = Some(world.observe_feeding());
        }
    }

    pub(super) fn begin(&mut self, world: Option<&World>) {
        self.ensure(world);
        if let Some(readout) = &self.readout {
            readout.begin_batch();
        }
    }

    fn snapshot(&mut self, world: Option<&World>) -> Option<FeedingStamp> {
        self.ensure(world);
        world.and_then(|world| self.readout.as_ref()?.snapshot(world))
    }
}

fn subject_dictionary(subject: FeedingSubject) -> VarDictionary {
    let mut output = VarDictionary::new();
    output.set("id", subject.id);
    output.set(
        "center",
        &PackedFloat64Array::from(subject.center.as_slice()),
    );
    output.set(
        "bounds",
        &PackedInt32Array::from(
            [
                subject.bounds.x,
                subject.bounds.y,
                subject.bounds.width,
                subject.bounds.height,
            ]
            .as_slice(),
        ),
    );
    output
}

impl BgrSimulation {
    pub(super) fn begin_feeding_batch(&mut self) {
        self.feeding_scene.begin(self.world.as_ref());
    }

    pub(super) fn feeding_dictionary(&mut self) -> VarDictionary {
        let stamp = self.feeding_scene.snapshot(self.world.as_ref());
        let mut event = VarDictionary::new();
        if let Some(stamp) = stamp {
            event.set("batch", i64::try_from(stamp.batch).unwrap_or(i64::MAX));
            event.set(
                "tick_at_transfer",
                i64::try_from(stamp.tick).unwrap_or(i64::MAX),
            );
            event.set("predator", &subject_dictionary(stamp.predator));
            event.set("prey", &subject_dictionary(stamp.prey));
            event.set("gross_energy", stamp.gross_energy);
            event.set("predator_alive_at_transfer", true);
            event.set("prey_alive_at_transfer", true);
        }
        let mut output = VarDictionary::new();
        output.set("available", self.world.is_some());
        output.set(
            "reason",
            if self.world.is_some() {
                ""
            } else {
                "world_not_ready"
            },
        );
        output.set("epoch", self.feeding_scene.epoch);
        output.set("scope", "latest_frontend_advance_batch");
        output.set("energy_unit", "gross_source_energy_before_costs");
        output.set("event", &event);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batches_and_world_replacement_change_only_presentation_state() {
        let mut world = World::new(WorldConfig {
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap();
        let before = world.clone();
        let random = world.random_snapshot();
        let mut state = FeedingSceneState::default();
        state.begin(Some(&world));
        let old = state.readout.clone().unwrap();
        let epoch = state.epoch;
        assert!(state.snapshot(Some(&world)).is_none());
        assert_eq!(world, before);
        assert_eq!(world.random_snapshot(), random);
        world.reset().unwrap();
        assert!(state.snapshot(Some(&world)).is_none());
        assert!(state.epoch > epoch);
        assert!(!old.is_current_for(&world));
        state.clear();
        assert!(state.readout.is_none());
        assert!(state.snapshot(None).is_none());
    }
}
