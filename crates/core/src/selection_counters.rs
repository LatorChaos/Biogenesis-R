// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Selection counter updates follow synchronous World callbacks, including partial failures

use super::*;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrganismCounters {
    pub children: i32,
    pub kills: i32,
    pub infections: i32,
}

impl OrganismCounters {
    fn from_actor(actor: &Organism) -> Self {
        Self {
            children: actor.state.total_children,
            kills: actor.state.total_kills,
            infections: actor.state.total_infected,
        }
    }
}

#[derive(Debug)]
pub(super) struct CounterState {
    actor: OrganismHandle,
    values: Cell<OrganismCounters>,
}

/// One identity-retaining toolbar readout; dropping it removes the weak subscription
#[derive(Clone, Debug)]
pub struct CounterReadout(Rc<CounterState>);

impl CounterReadout {
    pub fn snapshot(&self) -> OrganismCounters {
        self.0.values.get()
    }
}

impl World {
    /// Replace this World's counter subscriber only after a valid selection
    pub fn observe_organism_counters(&self, actor: &OrganismHandle) -> Option<CounterReadout> {
        if !self.contains_handle(actor) {
            return None;
        }
        let values = OrganismCounters::from_actor(&actor.0.borrow());
        let state = Rc::new(CounterState { actor: actor.clone(), values: Cell::new(values) });
        self.counter_observer.replace(Rc::downgrade(&state));
        Some(CounterReadout(state))
    }

    fn notify_counter(&self, actor: &Organism, update: impl FnOnce(&mut OrganismCounters)) {
        let Some(state) = self.counter_observer.borrow().upgrade() else { return; };
        // Compare stable RefCell payload addresses without reborrowing the active actor
        if !std::ptr::eq(state.actor.0.as_ptr(), actor) {
            return;
        }
        let mut values = state.values.get();
        update(&mut values);
        state.values.set(values);
    }

    pub(super) fn notify_children(&self, actor: &Organism) {
        self.notify_counter(actor, |values| values.children = actor.state.total_children);
    }

    pub(super) fn notify_kills(&self, actor: &Organism) {
        self.notify_counter(actor, |values| values.kills = actor.state.total_kills);
    }

    pub(super) fn notify_infections(&self, actor: &Organism) {
        self.notify_counter(actor, |values| values.infections = actor.state.total_infected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::infection::{Infector, InfectionKind};

    fn empty_world() -> World {
        World::new(WorldConfig { width: 7000, height: 7000, initial_attempts: 0, ..WorldConfig::default() }).unwrap()
    }

    fn actor(world: &World, position: [i32; 2]) -> OrganismHandle {
        let genome = GenomeHandle::from_owned(Genome::from_parts(
            &[crate::Gene::new(12.0, 0.0, crate::ColorKind::Green.argb(), 0, [0; 36]).shared()],
            crate::GenomeProperties { symmetry: 1, mutation_rate: 0, ..crate::GenomeProperties::default() },
        ));
        world.paste_genome_handle(&genome, position).unwrap().unwrap()
    }

    fn check(stage: &str, actor: &OrganismHandle, shown: &CounterReadout) {
        let row = include_str!("../tests/fixtures/inspector_events.tsv").lines()
            .map(|line| line.split('\t').collect::<Vec<_>>())
            .find(|row| row[0] == stage).expect("Frozen actual Java stage");
        let numbers: Vec<i32> = row[1..].iter().map(|value| value.parse().unwrap()).collect();
        let raw = OrganismCounters::from_actor(&actor.0.borrow());
        let display = shown.snapshot();
        assert_eq!([raw.children, raw.kills, raw.infections], numbers[..3], "{stage}: actual model");
        assert_eq!([display.children, display.kills, display.infections], numbers[3..], "{stage}: source toolbar");
    }

    #[test]
    fn selection_counter_notifications_follow_actual_gui_event_order() {
        let world = empty_world();
        let peer = actor(&world, [1000, 1000]);
        let selected = actor(&world, [2000, 2000]);
        let victim = actor(&world, [3000, 3000]);
        let mut shown = world.observe_organism_counters(&selected).unwrap();
        check("initial", &selected, &shown);
        selected.0.borrow_mut().state.total_children = 17;
        check("raw-children-before-callback", &selected, &shown);
        world.add_organism(Rc::clone(&peer.0), Some(&selected.0.borrow())).unwrap();
        check("birth-callback", &selected, &shown);
        peer.0.borrow_mut().state.total_children = 91;
        world.add_organism(Rc::clone(&victim.0), Some(&peer.0.borrow())).unwrap();
        check("unrelated-birth", &selected, &shown);
        for (stage, kind) in [("infection-direct", InfectionKind::Direct), ("infection-fruit", InfectionKind::Fruit), ("infection-weak-fruit", InfectionKind::WeakFruit)] {
            world.infect_from(&mut victim.0.borrow_mut(), Infector::Other(&mut selected.0.borrow_mut()), kind).unwrap();
            check(stage, &selected, &shown);
        }
        world.infect_from(&mut selected.0.borrow_mut(), Infector::Recipient, InfectionKind::Direct).unwrap();
        check("infection-self", &selected, &shown);
        for stage in ["death", "death-repeated"] {
            world.die(&mut victim.0.borrow_mut(), Some(super::super::death::Killer::Other(&mut selected.0.borrow_mut()))).unwrap();
            check(stage, &selected, &shown);
        }
        {
            let mut actor = selected.0.borrow_mut();
            actor.state.total_children = i32::MAX;
            actor.state.total_kills = i32::MAX;
            actor.state.total_infected = i32::MAX;
        }
        shown = world.observe_organism_counters(&selected).unwrap();
        check("wrapping-selected", &selected, &shown);
        selected.0.borrow_mut().state.total_children = i32::MIN;
        world.add_organism(Rc::clone(&peer.0), Some(&selected.0.borrow())).unwrap();
        check("birth-wrapped", &selected, &shown);
        world.die(&mut victim.0.borrow_mut(), Some(super::super::death::Killer::Other(&mut selected.0.borrow_mut()))).unwrap();
        check("kills-wrapped", &selected, &shown);
        world.infect_from(&mut victim.0.borrow_mut(), Infector::Other(&mut selected.0.borrow_mut()), InfectionKind::Direct).unwrap();
        check("infections-wrapped", &selected, &shown);
        let association = victim.0.borrow().state.world.clone();
        victim.0.borrow_mut().state.world = Default::default();
        assert!(world.infect_from(&mut victim.0.borrow_mut(), Infector::Other(&mut selected.0.borrow_mut()), InfectionKind::Direct).is_err());
        check("infection-before-notification-failure", &selected, &shown);
        victim.0.borrow_mut().state.world = association;
        shown = world.observe_organism_counters(&selected).unwrap();
        check("reselected-after-failure", &selected, &shown);
    }

    #[test]
    fn counter_subscription_is_weak_single_and_owned_by_the_notifying_world() {
        let world = empty_world();
        let first = actor(&world, [1000, 1000]);
        let second = actor(&world, [2000, 2000]);
        let old = world.observe_organism_counters(&first).unwrap();
        let current = world.observe_organism_counters(&second).unwrap();
        first.0.borrow_mut().state.total_children = 7;
        world.notify_children(&first.0.borrow());
        assert_eq!(old.snapshot().children, 0);
        second.0.borrow_mut().state.total_children = 11;
        let foreign = empty_world();
        assert!(foreign.observe_organism_counters(&second).is_none());
        foreign.notify_children(&second.0.borrow());
        assert_eq!(current.snapshot().children, 0);
        world.notify_children(&second.0.borrow());
        assert_eq!(current.snapshot().children, 11);
        assert!(world.clone().counter_observer.borrow().upgrade().is_none());
        drop(current);
        assert!(world.counter_observer.borrow().upgrade().is_none());
        world.notify_children(&second.0.borrow_mut());
    }

    #[test]
    fn shared_actor_counters_follow_the_callback_world_and_keep_completed_events() {
        let local = empty_world();
        let foreign = empty_world();
        let donor = actor(&local, [1000, 1000]);
        let local_recipient = actor(&local, [2000, 2000]);
        let foreign_recipient = actor(&foreign, [3000, 3000]);
        foreign.add_organism(Rc::clone(&donor.0), None).unwrap();
        let local_view = local.observe_organism_counters(&donor).unwrap();
        let foreign_view = foreign.observe_organism_counters(&donor).unwrap();
        foreign.infect_from(&mut foreign_recipient.0.borrow_mut(), Infector::Other(&mut donor.0.borrow_mut()), InfectionKind::Direct).unwrap();
        assert_eq!(foreign_view.snapshot().infections, 1);
        assert_eq!(local_view.snapshot().infections, 0);
        local.infect_from(&mut local_recipient.0.borrow_mut(), Infector::Other(&mut donor.0.borrow_mut()), InfectionKind::Direct).unwrap();
        assert_eq!(local_view.snapshot().infections, 2);
        assert_eq!(foreign_view.snapshot().infections, 1);
        local_recipient.0.borrow_mut().state.world = Default::default();
        assert!(local.infect_from(&mut local_recipient.0.borrow_mut(), Infector::Other(&mut donor.0.borrow_mut()), InfectionKind::Direct).is_err());
        assert_eq!(donor.0.borrow().state.total_infected, 3);
        assert_eq!(local_view.snapshot().infections, 2);
        assert_eq!(foreign_view.snapshot().infections, 1);
    }
}
