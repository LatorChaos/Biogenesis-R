// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Physical contact and EYE sensing from Organism.java:5863-6192

use crate::organism::primary_genome;

use super::{Organism, OrganismRef, World, WorldError};
use crate::{ColorKind, java_math::min, physics};

fn segment_line(actor: &Organism, segment: usize) -> Result<physics::Line, WorldError> {
    let points = actor.body.rotated_segment(segment)?;
    Ok(physics::Line::from_offsets(points.start, points.end, actor.state.integer_center))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EffectsState, Gene, Genome, GenomeProperties, IntRect, JavaRandom, RotatedSegment,
        WorldConfig, WorldParameters,
    };
    use std::rc::Rc;

    fn pair(
        actor_colors: &[ColorKind],
        peer_colors: &[ColorKind],
    ) -> (World, OrganismRef, OrganismRef) {
        let mut parameters = WorldParameters::default();
        parameters.genetics.min_mutation_rate = 0;
        parameters.genetics.max_mutation_rate = 0;
        parameters.genetics.meta_mutation_rate = 0;
        let world = World::with_parameters(
            WorldConfig {
                initial_attempts: 0,
                ..WorldConfig::default()
            },
            parameters,
        )
        .unwrap();
        for (colors, center) in [(actor_colors, [1000, 1000]), (peer_colors, [3500, 3500])] {
            let genes: Vec<_> = colors
                .iter()
                .map(|color| Gene::new(12.0, 0.0, color.argb(), -1, [0; 36]).shared())
                .collect();
            world
                .paste_genome(
                    Genome::from_parts(
                        &genes,
                        GenomeProperties {
                            symmetry: 1,
                            ..GenomeProperties::default()
                        },
                    ),
                    center,
                )
                .unwrap()
                .unwrap();
        }
        let actor_ref = Rc::clone(&world.organisms.borrow()[0]);
        let peer_ref = Rc::clone(&world.organisms.borrow()[1]);
        for (reference, vertical) in [(&actor_ref, false), (&peer_ref, true)] {
            let mut actor = reference.borrow_mut();
            actor.effects = EffectsState {

                ..EffectsState::default()
            };
            actor.state.parent_id = -1;
            actor.state.center = [2000.0; 2];
            actor.state.integer_center = [2000; 2];
            actor.state.bounds.set(IntRect {
                x: 1994,
                y: 1994,
                width: 13,
                height: 13,
            });
            actor.state.velocity = [0.0; 2];
            actor.state.angular_velocity = 0.0;
            actor.state.energy = 100.0;
            actor.state.active = true;
            actor.state.alive = true;
            actor.body.mass = 1.0;
            actor.body.inertia = 1.0;
            actor.body.eyes.enabled = false;
            for index in actor.body.indices() { actor.body.set_physical_length(index, 2.0).unwrap(); }
            for index in actor.body.indices() {
                let value = if vertical {
                    RotatedSegment {
                        start: [0, -5],
                        end: [0, 5],
                    }
                } else {
                    RotatedSegment {
                        start: [-5, 0],
                        end: [5, 0],
                    }
                };
                actor.body.set_rotated_segment(index, value).unwrap();
            }
        }
        world.oxygen.set(1000.0);
        world.carbon_dioxide.set(0.0);
        world.rebuild_spatial();
        (world, actor_ref, peer_ref)
    }

    fn contact(world: &mut World, actor_ref: &OrganismRef, peer_ref: &OrganismRef) -> bool {
        world
            .contact_from(
                actor_ref,
                &mut actor_ref.borrow_mut(),
                peer_ref,
                &mut peer_ref.borrow_mut(),
            )
            .unwrap()
    }

    #[test]
    fn physical_loops_select_only_the_highest_pair_and_keep_spike_ties_strict() {
        let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Spike; 2], &[ColorKind::Spike; 2]);
        for reference in [&actor_ref, &peer_ref] {
            let mut actor = reference.borrow_mut();
            actor.effects.is_spike = true;
            actor.effects.use_pre_touch_effects = true;
            for index in actor.body.indices() { actor.body.set_photo(index, -2.0).unwrap(); }
        }
        assert!(contact(&mut world, &actor_ref, &peer_ref));
        for reference in [&actor_ref, &peer_ref] {
            let actor = reference.borrow();
            assert_eq!(actor.body.photo_values().unwrap(), [-2.0, 0.0]);
            assert!(
                (actor.body.indices()).all(|index| actor.body.color_argb(index).unwrap() == ColorKind::Spike.argb().unwrap())
            );
        }
    }

    #[test]
    fn nonspike_endpoint_ties_select_a_tip_even_between_friends() {
        let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Spike], &[ColorKind::Green]);
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects.is_spike = true;
            actor.effects.use_pre_touch_effects = true;
            actor.state.parent_id = peer_ref.borrow().id as i32;
        }
        assert!(contact(&mut world, &actor_ref, &peer_ref));
        let actor = actor_ref.borrow();
        assert_eq!(
            actor.body.color_argb(0).unwrap(),
            ColorKind::SpikePoint.argb().unwrap()
        );
        assert_eq!(actor.body.photo_values().unwrap(), [-1.0]);
        assert_eq!(peer_ref.borrow().state.energy, 100.0);
    }

    #[test]
    fn matching_photo_marker_preserves_the_existing_spike_color() {
        let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Spike], &[ColorKind::Green]);
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects.is_spike = true;
            actor.effects.use_pre_touch_effects = true;
            actor.body.set_photo(0, -1.0).unwrap();
            actor.state.parent_id = peer_ref.borrow().id as i32;
        }
        assert!(contact(&mut world, &actor_ref, &peer_ref));
        assert_eq!(
            actor_ref.borrow().body.color_argb(0).unwrap(),
            ColorKind::Spike.argb().unwrap()
        );
    }

    #[test]
    fn physical_impulse_precedes_reaction_braking() {
        let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Teal], &[ColorKind::Green]);
        world.session.0.parameters.borrow_mut().physics.elasticity = 1.0;
        {
            let mut actor = actor_ref.borrow_mut();
            actor.effects.can_react = true;
            actor.body.set_reaction(0, 1, 1).unwrap();
            actor.state.velocity = [0.0, 1.0];
        }
        assert!(contact(&mut world, &actor_ref, &peer_ref));
        assert_eq!(peer_ref.borrow().state.velocity, [0.0, 1.0]);
        assert_eq!(actor_ref.borrow().state.velocity, [0.0, 0.0]);
    }

    #[test]
    fn pretouch_mimic_changes_friendship_and_the_current_biological_gate() {
        let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Dark], &[ColorKind::Red]);
        {
            let mut actor = actor_ref.borrow_mut();
            let mut properties = actor.genome.as_ref().unwrap().borrow().properties();
            properties.modifiers.generation_battle = true;
            actor.genome.as_ref().unwrap().borrow_mut().set_properties(properties);
            actor.state.parent_id = peer_ref.borrow().id as i32;
            actor.effects.use_pre_touch_effects = true;
            actor.effects.segment_pre_touch_effects = 2;
            assert!(!actor.effects.generation_battle);
        }
        assert!(contact(&mut world, &actor_ref, &peer_ref));
        let actor = actor_ref.borrow();
        assert!(actor.effects.generation_battle);
        assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::Red.argb().unwrap());
        assert_eq!(actor.body.photo_values().unwrap(), [-1.0]);
        assert!(peer_ref.borrow().state.energy < 100.0);
    }

    #[test]
    fn reverse_reaction_still_executes_after_the_first_actor_dies() {
        let (mut world, actor_ref, peer_ref) =
            pair(&[ColorKind::Gray], &[ColorKind::Teal, ColorKind::Blue]);
        world.session.0.parameters.borrow_mut().lifecycle.blue_energy_consumption = 0.125;
        world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 0.25;
        {
            let mut actor = actor_ref.borrow_mut();
            actor.state.energy = 0.5;
            actor.body.set_photo(0, -2.0).unwrap();
            let mut peer = peer_ref.borrow_mut();
            peer.effects.is_enhanced = true;
            peer.effects.can_react = true;
            peer.body.set_physical_length(0, 0.5).unwrap();
            peer.body.set_reaction(1, 31, 1).unwrap();
        }
        assert!(contact(&mut world, &actor_ref, &peer_ref));
        assert!(!actor_ref.borrow().state.alive);
        assert_eq!(peer_ref.borrow().state.energy, 99.625);
    }

    #[test]
    fn eye_visits_peer_segments_ascending_without_can_react_or_physical_return() {
        let (mut world, actor_ref, peer_ref) = pair(
            &[ColorKind::Teal, ColorKind::Eye],
            &[ColorKind::Green, ColorKind::Red],
        );
        world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.rubbing = 1.0;
        world.oxygen.set(1.5);
        world.session.0.random.replace(JavaRandom::new(1));
        {
            let mut actor = actor_ref.borrow_mut();
            actor.body.eyes.enabled = true;
            actor.effects.can_move = true;
            actor.effects.can_dodge = true;
            actor.effects.can_react = false;
            for index in actor.body.indices() { actor.body.set_physical_length(index, 0.5).unwrap(); }
            actor.body.set_reaction(1, 1, 1).unwrap();
            actor.body.set_reaction(1, 0, 2).unwrap();
            actor.state.velocity = [1.0, 0.0];
        }
        assert!(!contact(&mut world, &actor_ref, &peer_ref));
        assert_eq!(actor_ref.borrow().state.velocity, [0.2, 0.0]);
        assert_eq!(actor_ref.borrow().state.angular_velocity, 0.0);
        assert_eq!(actor_ref.borrow().state.energy, 98.5);
        assert_eq!(peer_ref.borrow().state.velocity, [0.0; 2]);
        let mut expected = JavaRandom::new(1);
        assert_eq!(expected.bounded(10), 5);
        assert_eq!(world.random_snapshot(), expected);
    }

    #[test]
    fn immobile_eye_brakes_for_every_hit_without_reversing_the_peer() {
        let (mut world, actor_ref, peer_ref) =
            pair(&[ColorKind::Eye], &[ColorKind::Green, ColorKind::Red]);
        world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.rubbing = 1.0;
        {
            let mut actor = actor_ref.borrow_mut();
            actor.body.eyes.enabled = true;
            actor.body.set_physical_length(0, 0.5).unwrap();
            actor.body.set_reaction(0, 0, 1).unwrap();
            actor.body.set_reaction(0, 1, 1).unwrap();
            actor.state.velocity = [1.0, 0.0];
            peer_ref.borrow_mut().effects.can_react = true;
        }
        let random = world.random_snapshot();
        assert!(!contact(&mut world, &actor_ref, &peer_ref));
        assert_eq!(actor_ref.borrow().state.velocity, [0.4 * 0.4, 0.0]);
        assert_eq!(actor_ref.borrow().state.energy, 98.0);
        assert_eq!(peer_ref.borrow().state.energy, 100.0);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn eye_sensing_requires_the_entry_flag_and_exact_argb_but_accepts_nan_length() {
        for (enabled, argb, length, responds) in [
            (false, ColorKind::Eye.argb().unwrap(), 0.5, false),
            (
                true,
                0x12ff_ffff & ColorKind::Eye.argb().unwrap(),
                0.5,
                false,
            ),
            (true, ColorKind::Eye.argb().unwrap(), f64::NAN, true),
        ] {
            let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Eye], &[ColorKind::Green]);
            world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 1.0;
            {
                let mut actor = actor_ref.borrow_mut();
                actor.body.eyes.enabled = enabled;
                actor.body.set_color_argb(0, argb).unwrap();
                actor.body.set_physical_length(0, length).unwrap();
                actor.body.set_reaction(0, 1, 1).unwrap();
            }
            assert!(!contact(&mut world, &actor_ref, &peer_ref));
            assert_eq!(
                actor_ref.borrow().state.energy,
                if responds { 99.0 } else { 100.0 }
            );
        }
    }

    #[test]
    fn long_eye_segments_use_the_regular_physical_path() {
        let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Eye], &[ColorKind::Green]);
        actor_ref.borrow_mut().body.eyes.enabled = true;
        assert!(contact(&mut world, &actor_ref, &peer_ref));
    }

    #[test]
    fn contact_lines_wrap_integer_offsets_before_widening() {
        let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Green], &[ColorKind::Green]);
        {
            let mut actor = actor_ref.borrow_mut();
            actor.state.integer_center = [i32::MAX, 0];
            actor.body.set_rotated_segment(0, RotatedSegment {
                start: [1, 0],
                end: [3, 0],
            }).unwrap();
            actor.state.bounds.set(IntRect {
                x: i32::MIN,
                y: -2,
                width: 4,
                height: 5,
            });
            let mut peer = peer_ref.borrow_mut();
            peer.state.integer_center = [i32::MIN + 1, 0];
            peer.body.set_rotated_segment(0, RotatedSegment {
                start: [0, -1],
                end: [0, 1],
            }).unwrap();
            peer.state.bounds.set(actor.state.bounds.get());
        }
        assert!(contact(&mut world, &actor_ref, &peer_ref));
    }

    #[test]
    fn synchronous_plague_birth_uses_both_held_contacts_and_stays_outside_frozen_membership() {
        let (mut world, actor_ref, peer_ref) = pair(&[ColorKind::Plague], &[ColorKind::Green]);
        {
            let mut actor = actor_ref.borrow_mut();
            actor.state.energy = 20.0;
            actor.effects.is_plant = true;
            actor.effects.is_infectious = true;
            actor.effects.plague_version = 1;
            actor.body.set_photo(0, -2.0).unwrap();
            let mut peer = peer_ref.borrow_mut();
            peer.effects.is_plant = true;
            peer.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
            peer.state.reproduction.infected_energy = 43;
        }
        assert!(contact(&mut world, &actor_ref, &peer_ref));
        assert_eq!(world.organisms.borrow().len(), 3);
        assert_eq!(world.frame_objects.borrow().len(), 2);
        assert_eq!(peer_ref.borrow().state.total_children, 1);
        assert_eq!(peer_ref.borrow().state.virus_children, 0);
        assert_eq!(actor_ref.borrow().state.energy, 10.0);
    }
}

fn friendly(actor: &Organism, peer: &Organism) -> Result<bool, crate::WorldError> {
    Ok(((actor.state.parent_id == peer.id as i32 || actor.id as i32 == peer.state.parent_id)
        && !actor.effects.generation_battle
        && !peer.effects.generation_battle
        || (actor.state.parent_id == peer.state.parent_id
            && actor.state.parent_id != -1
            && !actor.effects.sibling_battle
            && !peer.effects.sibling_battle)
        || (actor.effects.transfers_energy
            && peer.effects.transfers_energy
            && ((primary_genome(&actor.genome)?.borrow().modifiers().peaceful
                && primary_genome(&peer.genome)?.borrow().modifiers().peaceful)
                || (actor.effects.friend_length == peer.effects.friend_length
                    && actor.effects.friend_theta == peer.effects.friend_theta
                    && primary_genome(&actor.genome)?.borrow().modifiers().social
                    && primary_genome(&peer.genome)?.borrow().modifiers().social))))
        && actor.state.alive
        && peer.state.alive)
}

fn spike_tip(
    actor: &mut Organism,
    peer: &Organism,
    segment: usize,
    peer_segment: usize,
    distances: [f64; 2],
) -> Result<(), crate::WorldError> {
    use ColorKind::{Spike, SpikePoint};
    if actor.effects.is_spike
        && matches!(
            ColorKind::from_argb(actor.body.color_argb(segment)?),
            Spike | SpikePoint
        )
    {
        let other_spike = peer.effects.is_spike
            && matches!(
                ColorKind::from_argb(peer.body.color_argb(peer_segment)?),
                Spike | SpikePoint
            );
        let point = if other_spike {
            distances[0] < distances[1]
        } else {
            distances[0] <= distances[1]
        };
        let coefficient = if point { -1.0 } else { 0.0 };
        if actor.body.photo(segment)? != coefficient {
            actor.body.set_color(segment, if point { SpikePoint } else { Spike }.color_ref())?;
            actor.body.set_photo(segment, coefficient)?;
        }
    }
    Ok(())
    }

impl World {
    /// Apply one source contact using distinct, already held organism identities
    pub(super) fn contact_from(
        &self,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
        peer_ref: &OrganismRef,
        peer: &mut Organism,
    ) -> Result<bool, WorldError> {
        let eyes = actor.body.eyes.enabled;
        for segment in actor.body.reverse_indices() {
            if actor.body.physical_length(segment)? >= 1.0 {
                let line = segment_line(actor, segment)?;
                if !physics::rectangle_intersects_line(peer.state.bounds.get(), line) {
                    continue;
                }
                for peer_segment in peer.body.reverse_indices() {
                    if peer.body.physical_length(peer_segment)? >= 1.0 {
                        let peer_line = segment_line(peer, peer_segment)?;
                        if physics::rectangle_intersects_line(actor.state.bounds.get(), peer_line)
                            && physics::lines_intersect(line, peer_line)
                        {
                            let point = physics::intersection(line, peer_line);
                            let near_actor = min(
                                physics::point_distance_squared(point, line.start),
                                physics::point_distance_squared(point, line.end),
                            );
                            let near_peer = min(
                                physics::point_distance_squared(point, peer_line.start),
                                physics::point_distance_squared(point, peer_line.end),
                            );
                            let (impulse_line, this_organism) = if near_actor < near_peer {
                                (peer_line, false)
                            } else {
                                (line, true)
                            };
                            physics::touch_move(
                                actor,
                                peer,
                                point,
                                impulse_line,
                                this_organism,
                                &self.session.0.parameters.borrow().physics,
                            );
                            if actor.effects.use_pre_touch_effects {
                                spike_tip(
                                    actor,
                                    peer,
                                    segment,
                                    peer_segment,
                                    [near_actor, near_peer],
                                )?;
                                if actor.effects.segment_pre_touch_effects > 0 {
                                    self.pre_touch_effects(actor, peer, segment, peer_segment)?;
                                }
                            }
                            if peer.effects.use_pre_touch_effects {
                                spike_tip(
                                    peer,
                                    actor,
                                    peer_segment,
                                    segment,
                                    [near_peer, near_actor],
                                )?;
                                if peer.effects.segment_pre_touch_effects > 0 {
                                    self.pre_touch_effects(peer, actor, peer_segment, segment)?;
                                }
                            }
                            if friendly(actor, peer)? {
                                if actor.effects.can_react {
                                    self.touch_friend_reaction(actor, peer, segment, peer_segment)?;
                                }
                                if actor.effects.use_friend_effects > 0 {
                                    self.touch_friend_effects(actor, peer, segment, peer_segment)?;
                                }
                                if peer.effects.can_react {
                                    self.touch_friend_reaction(peer, actor, peer_segment, segment)?;
                                }
                                if peer.effects.use_friend_effects > 0 {
                                    self.touch_friend_effects(peer, actor, peer_segment, segment)?;
                                }
                            } else {
                                if actor.effects.can_react {
                                    self.touch_reaction(actor, peer, segment, peer_segment)?;
                                }
                                if actor.body.photo(segment)? == -1.0 {
                                    self.touch_effects_one(
                                        actor_ref,
                                        actor,
                                        peer_ref,
                                        peer,
                                        segment,
                                        peer_segment,
                                    )?;
                                } else if actor.body.photo(segment)? == -2.0 {
                                    self.touch_effects_two(
                                        actor_ref,
                                        actor,
                                        peer_ref,
                                        peer,
                                        segment,
                                        peer_segment,
                                    )?;
                                }
                                if peer.effects.can_react {
                                    self.touch_reaction(peer, actor, peer_segment, segment)?;
                                }
                                if peer.body.photo(peer_segment)? == -1.0 {
                                    self.touch_effects_one(
                                        peer_ref,
                                        peer,
                                        actor_ref,
                                        actor,
                                        peer_segment,
                                        segment,
                                    )?;
                                } else if peer.body.photo(peer_segment)? == -2.0 {
                                    self.touch_effects_two(
                                        peer_ref,
                                        peer,
                                        actor_ref,
                                        actor,
                                        peer_segment,
                                        segment,
                                    )?;
                                }
                            }
                            return Ok(true);
                        }
                    }
                }
            } else if eyes && actor.body.color_argb(segment)? == ColorKind::Eye.argb().unwrap() {
                let moving = actor.effects.can_move;
                if moving || actor.state.alive {
                    let line = segment_line(actor, segment)?;
                    if !physics::rectangle_intersects_line(peer.state.bounds.get(), line) { continue; }
                    for peer_segment in peer.body.indices() {
                        if peer.body.physical_length(peer_segment)? >= 1.0 {
                            let peer_line = segment_line(peer, peer_segment)?;
                            if physics::rectangle_intersects_line(actor.state.bounds.get(), peer_line)
                                && physics::lines_intersect(line, peer_line)
                            {
                                if moving {
                                    if friendly(actor, peer)? {
                                        self.touch_friend_reaction(
                                            actor,
                                            peer,
                                            segment,
                                            peer_segment,
                                        )?;
                                    } else {
                                        self.touch_reaction(actor, peer, segment, peer_segment)?;
                                    }
                                } else if friendly(actor, peer)? {
                                    self.touch_friend_brake(actor, peer, segment, peer_segment)?;
                                } else {
                                    self.touch_brake(actor, peer, segment, peer_segment)?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(false)
    }
}
