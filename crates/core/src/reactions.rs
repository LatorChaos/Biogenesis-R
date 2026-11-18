// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Segment reactions and eye brakes from Organism.java:6927-8682

use super::World;
use crate::{ColorKind, movement, organism::Organism, physics::between};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
enum Channel {
    Red,
    Green,
    Blue,
    Plague,
    Scourge,
    White,
    Gray,
    Silver,
    Default,
    Consumer,
    Plant,
    Magenta,
    Pink,
    Coral,
    Orange,
    Bark,
    Violet,
    Virus,
    Maroon,
    Olive,
    Mint,
    Cream,
    Spike,
    Fallow,
    LightBlue,
    Ochre,
    Sky,
    Lilac,
    Fire,
    LightBrown,
    GreenBrown,
    Brown,
    Ice,
    Broken,
    Sick,
    Friend,
}

fn color(actor: &Organism, segment: usize) -> Result<ColorKind, crate::WorldError> {
    Ok(actor.body.color_kind(segment)?)
}

fn sick(color: ColorKind) -> bool {
    use ColorKind::*;
    matches!(
        color,
        Spike
            | DarkOlive
            | Broken
            | DarkFire
            | LightBrown
            | GreenBrown
            | LightBlue
            | DeadBark
            | Ice
    )
}

/// Preserve the source's flag precedence, including different SILVER and default priorities
fn peer_channel(actor_color: ColorKind, peer: &Organism, peer_color: ColorKind) -> Option<Channel> {
    use ColorKind::*;
    Some(match peer_color {
        Eye => return None,
        White | Plague => {
            if peer.effects.is_plant {
                if peer.effects.plague_version > 0 {
                    Channel::Plague
                } else {
                    Channel::White
                }
            } else if peer.effects.plague_version > 0
                || peer.effects.is_consumer
                || peer.effects.is_fungus
                || peer.effects.is_auburn
            {
                Channel::Scourge
            } else {
                Channel::Virus
            }
        }
        Orange => Channel::Orange,
        Fire => Channel::Fire,
        Red => Channel::Red,
        Pink => Channel::Pink,
        Maroon => Channel::Maroon,
        Cream => Channel::Cream,
        Bark | OldBark => Channel::Bark,
        Green | Forest | Spring | Summer | Winter | Lime | Leaf | C4 | Jade | DarkJade
        | DarkGreen | Grass | Purple => Channel::Green,
        Blue => Channel::Blue,
        Ochre => Channel::Ochre,
        Olive => Channel::Olive,
        Sky | DeepSky => Channel::Sky,
        Fruit => {
            if peer.effects.spore_version == 2 {
                Channel::White
            } else {
                Channel::Virus
            }
        }
        Silver => {
            if (peer.state.total_kills > 0 && (peer.effects.is_consumer || peer.effects.is_fungus))
                || peer.effects.is_enhanced
            {
                Channel::Silver
            } else if peer.effects.is_plant {
                Channel::Plant
            } else if peer.effects.is_consumer || peer.effects.is_fungus {
                Channel::Consumer
            } else {
                Channel::Default
            }
        }
        Coral => Channel::Coral,
        Fallow => Channel::Fallow,
        Violet => Channel::Violet,
        SpikePoint => Channel::Spike,
        Spike => {
            if actor_color == Eye {
                Channel::Spike
            } else if peer.effects.is_plant {
                Channel::Green
            } else if peer.effects.is_enhanced {
                Channel::Consumer
            } else {
                Channel::Blue
            }
        }
        Lilac | DarkLilac => Channel::Lilac,
        Gray => Channel::Gray,
        Mint | Lavender => Channel::Mint,
        Magenta | Rose => Channel::Magenta,
        LightBrown | DarkFire => Channel::LightBrown,
        GreenBrown => Channel::GreenBrown,
        DarkOlive | LightBlue => Channel::LightBlue,
        Ice | DeadBark => Channel::Ice,
        Broken => Channel::Broken,
        Brown => Channel::Brown,
        NoColor | Spore | Cyan | Teal | Yellow | Auburn | Indigo | Blond | Flower | DarkGray
        | Gold | Dark | Vision => {
            if peer.effects.is_consumer || peer.effects.is_fungus {
                Channel::Consumer
            } else if peer.effects.is_plant {
                Channel::Plant
            } else {
                Channel::Default
            }
        }
    })
}

impl World {
    /// Source reactions select only the last eligible actuator, even when its reaction does nothing
    fn react(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        segment: usize,
        peer_segment: usize,
        friendly: bool,
    ) -> Result<(), crate::WorldError> {
        let mut actuator = None;
        for index in actor.body.reverse_indices() {
            if color(actor, index)? == ColorKind::Teal
                || (color(actor, segment)? == ColorKind::Eye
                    && color(actor, index)? == ColorKind::Cyan) {
                actuator = Some(index);
                break;
            }
        }
        let Some(actuator) = actuator else { return Ok(()); };
        let own_color = color(actor, segment)?;
        let other_color = color(peer, peer_segment)?;
        if other_color == ColorKind::Eye {
            return Ok(());
        }
        let channel = if friendly {
            Channel::Friend
        } else if sick(own_color) {
            Channel::Sick
        } else {
            peer_channel(own_color, peer, other_color).unwrap()
        };
        let reaction = actor.body.reaction(segment, channel as usize)?;
        if reaction == 1 {
            if self.use_actor_energy(actor, self.session.0.parameters.borrow().lifecycle.teal_energy_consumption)? {
                movement::rubbing(actor, self.session.0.parameters.borrow().lifecycle.rubbing / 5.0);
            }
            return Ok(());
        }
        if !(2..=5).contains(&reaction) {
            return Ok(());
        }
        let responds = if actor.effects.can_dodge {
            self.session.0.random.borrow_mut().bounded(10) > 2
        } else {
            self.session.0.random.borrow_mut().next_bool()
        };
        if !responds
            || !self.use_actor_energy(actor, self.session.0.parameters.borrow().lifecycle.teal_energy_consumption)?
        {
            return Ok(());
        }
        let denominator = if friendly { 3.0 } else { 1.5 } * actor.body.mass;
        for axis in 0..2 {
            let direction = match reaction {
                2 => actor.body.rotated_start(segment, axis)?.wrapping_sub(actor.body.rotated_end(segment, axis)?),
                3 => actor.body.rotated_start(0, axis)?.wrapping_sub(actor.body.rotated_end(segment, axis)?),
                4 => actor.body.rotated_end(segment, axis)?.wrapping_sub(actor.body.rotated_start(segment, axis)?),
                5 => peer.state.integer_center[axis].wrapping_sub(actor.state.integer_center[axis]),
                _ => unreachable!(),
            };
            let squared_length = actor.body.physical_length(actuator)? * actor.body.physical_length(actuator)?;
            let propulsion = actor.body.rotated_end(actuator, axis)?.wrapping_sub(actor.body.rotated_start(actuator, axis)?);
            actor.state.velocity[axis] = between(
                crate::java_math::div(f64::from(direction) * squared_length + 12.0 * f64::from(propulsion), denominator),
                -self.session.0.parameters.borrow().physics.max_velocity, self.session.0.parameters.borrow().physics.max_velocity,
            );
        }
        let sign = (self.session.0.random.borrow_mut().bounded(2) << 1) - 1;
        actor.state.angular_velocity = between(
            actor.state.angular_velocity + crate::java_math::div(f64::from(sign) * actor.body.physical_length(actuator)? * PI, actor.body.inertia),
            -self.session.0.parameters.borrow().physics.max_rotation, self.session.0.parameters.borrow().physics.max_rotation,
        );
        Ok(())
    }

    pub(super) fn touch_friend_reaction(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        self.react(actor, peer, segment, peer_segment, true)?;
        Ok(())
    }

    pub(super) fn touch_reaction(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        self.react(actor, peer, segment, peer_segment, false)?;
        Ok(())
    }

    pub(super) fn touch_friend_brake(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        // Source classifies the actor before its otherwise unconditional peer switch
        let _actor_color = color(actor, segment)?;
        if color(peer, peer_segment)? != ColorKind::Eye {
            self.brake(actor, segment, Channel::Friend)?;
        }
        Ok(())
    }

    pub(super) fn touch_brake(
        &self,
        actor: &mut Organism,
        peer: &Organism,
        segment: usize,
        peer_segment: usize,
    ) -> Result<(), crate::WorldError> {
        // Brakes do not apply the moving reaction's injured-self override
        if let Some(channel) = peer_channel(color(actor, segment)?, peer, color(peer, peer_segment)?)
        {
            self.brake(actor, segment, channel)?;
        }
        Ok(())
    }

    fn brake(&self, actor: &mut Organism, segment: usize, channel: Channel) -> Result<(), crate::WorldError> {
        if actor.body.reaction(segment, channel as usize)? == 1
            && self.use_actor_energy(actor, self.session.0.parameters.borrow().lifecycle.teal_energy_consumption)?
        {
            movement::rubbing(actor, self.session.0.parameters.borrow().lifecycle.rubbing / 2.5);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Gene, Genome, GenomeProperties, JavaRandom, OrganismState, RotatedSegment, WorldConfig,
    };
    use std::{cell::RefCell, rc::Rc};

    fn organism(colors: &[ColorKind]) -> Organism {
        let genes: Vec<_> = colors
            .iter()
            .map(|color| Gene::new(12.0, 0.0, color.argb(), 0, [0; 36]).shared())
            .collect();
        let genome = Genome::from_parts(
            &genes,
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        let mut actor =
            Organism::unplaced(Rc::new(RefCell::new(genome)), OrganismState::default()).unwrap();
        actor.state.energy = 10.0;
        actor.body.mass = 8.0;
        actor.body.inertia = 100.0;
        actor
    }

    fn owned_organism(world: &World, colors: &[ColorKind]) -> Organism {
        let mut actor = organism(colors);
        actor.state.world = world.association();
        actor
    }

    fn world() -> World {
        let world = World::new(WorldConfig {
            seed: 0,
            width: 1000,
            height: 1000,
            initial_attempts: 0,
        })
        .unwrap();
        world.session.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 1.0;
        world.oxygen.set(10.0);
        world.carbon_dioxide.set(0.0);
        world
    }

    #[test]
    fn state_sensitive_channels_preserve_source_precedence_and_exact_argb() {
        let mut peer = organism(&[ColorKind::White]);
        let classify = |peer: &Organism, own, other| peer_channel(own, peer, other).unwrap();
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::White),
            Channel::Virus
        );
        peer.effects.is_auburn = true;
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::White),
            Channel::Scourge
        );
        peer.effects.is_plant = true;
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::White),
            Channel::White
        );
        peer.effects.plague_version = 1;
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::White),
            Channel::Plague
        );
        peer.effects.is_consumer = true;
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::Silver),
            Channel::Plant
        );
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::NoColor),
            Channel::Consumer
        );
        peer.state.total_kills = 1;
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::Silver),
            Channel::Silver
        );
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::Spike),
            Channel::Spike
        );
        assert_eq!(
            classify(&peer, ColorKind::Green, ColorKind::Spike),
            Channel::Green
        );
        peer.effects.is_plant = false;
        peer.effects.is_enhanced = true;
        assert_eq!(
            classify(&peer, ColorKind::Green, ColorKind::Spike),
            Channel::Consumer
        );
        peer.effects.is_enhanced = false;
        assert_eq!(
            classify(&peer, ColorKind::Green, ColorKind::Spike),
            Channel::Blue
        );
        peer.effects.spore_version = 2;
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::Fruit),
            Channel::White
        );
        peer.effects.spore_version = 1;
        assert_eq!(
            classify(&peer, ColorKind::Eye, ColorKind::Fruit),
            Channel::Virus
        );
        peer.body.set_color_argb(0, 0x80004040).unwrap();
        assert_eq!(color(&peer, 0).unwrap(), ColorKind::NoColor);
        assert_eq!(peer_channel(ColorKind::Eye, &peer, ColorKind::Eye), None);
    }

    #[test]
    fn reverse_selection_spends_once_and_preserves_random_draw_order() {
        let world = world();
        let mut actor = owned_organism(&world, &[ColorKind::Teal, ColorKind::Teal, ColorKind::Green]);
        let peer = owned_organism(&world, &[ColorKind::Green]);
        actor.body.set_reaction(2, Channel::Green as usize, 2).unwrap();
        actor.body.set_physical_length(0, 100.0).unwrap();
        actor.body.set_physical_length(1, 2.0).unwrap();
        { let value = [-100, -100]; actor.body.set_rotated_end(0, 0, value[0]).unwrap(); actor.body.set_rotated_end(0, 1, value[1]).unwrap(); };
        { let value = [2, 0]; actor.body.set_rotated_end(1, 0, value[0]).unwrap(); actor.body.set_rotated_end(1, 1, value[1]).unwrap(); };
        let mut expected_random = world.random_snapshot();
        assert!(expected_random.next_bool());
        let sign = (expected_random.bounded(2) << 1) - 1;
        world.touch_reaction(&mut actor, &peer, 2, 0).unwrap();
        assert_eq!(actor.state.velocity, [2.0, 0.0]);
        assert_eq!(
            actor.state.angular_velocity.to_bits(),
            (f64::from(sign) * 2.0 * PI / 100.0).to_bits()
        );
        assert_eq!(actor.state.energy, 9.0);
        assert_eq!((world.oxygen.get(), world.carbon_dioxide.get()), (9.0, 1.0));
        assert_eq!(world.random_snapshot(), expected_random);
    }

    #[test]
    fn partial_payment_consumes_gas_after_gate_without_rotation_draw_or_motion() {
        let world = world();
        let mut actor = owned_organism(&world, &[ColorKind::Teal]);
        let peer = owned_organism(&world, &[ColorKind::Green]);
        actor.body.set_reaction(0, Channel::Green as usize, 2).unwrap();
        actor.state.velocity = [2.0, -3.0];
        actor.state.angular_velocity = 0.125;
        world.oxygen.set(0.5);
        let mut expected_random = world.random_snapshot();
        assert!(expected_random.next_bool());
        world.touch_reaction(&mut actor, &peer, 0, 0).unwrap();
        assert_eq!(actor.state.velocity, [2.0, -3.0]);
        assert_eq!(actor.state.angular_velocity, 0.125);
        assert_eq!(actor.state.energy, 9.5);
        assert_eq!((world.oxygen.get(), world.carbon_dioxide.get()), (0.0, 0.5));
        assert_eq!(world.random_snapshot(), expected_random);
        actor.body.set_reaction(0, Channel::Green as usize, 1).unwrap();
        world.touch_brake(&mut actor, &peer, 0, 0).unwrap();
        assert_eq!(actor.state.velocity, [2.0, -3.0]);
        assert_eq!(world.random_snapshot(), expected_random);
    }

    #[test]
    fn mode_three_uses_first_segment_start_and_wrapped_integer_subtraction() {
        let world = world();
        let mut actor = owned_organism(&world, &[ColorKind::Teal, ColorKind::Eye]);
        let peer = owned_organism(&world, &[ColorKind::Green]);
        actor.body.set_reaction(1, Channel::Green as usize, 3).unwrap();
        actor.body.set_physical_length(0, 1.0).unwrap();
        actor.body.mass = 2.0;
        actor.body.set_rotated_segment(0, RotatedSegment {
            start: [i32::MAX, 3],
            end: [i32::MAX, 3],
        }).unwrap();
        actor.body.set_rotated_segment(1, RotatedSegment {
            start: [100, 10],
            end: [i32::MIN, 6],
        }).unwrap();
        world.touch_reaction(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(
            actor.state.velocity[0].to_bits(),
            (-1.0_f64 / 3.0).to_bits()
        );
        assert_eq!(actor.state.velocity[1], -1.0);
    }

    #[test]
    fn friendly_acceleration_and_brakes_keep_distinct_coefficients() {
        let world = world();
        let mut actor = owned_organism(&world, &[ColorKind::Teal, ColorKind::Green]);
        let peer = owned_organism(&world, &[ColorKind::Green]);
        actor.body.set_reaction(1, Channel::Friend as usize, 2).unwrap();
        { let value = [2, 0]; actor.body.set_rotated_end(0, 0, value[0]).unwrap(); actor.body.set_rotated_end(0, 1, value[1]).unwrap(); };
        world.touch_friend_reaction(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(actor.state.velocity, [1.0, 0.0]);
        actor.body.set_reaction(1, Channel::Friend as usize, 1).unwrap();
        actor.state.velocity = [1.0, -1.0];
        actor.state.angular_velocity = 1.0;
        let random = world.random_snapshot();
        world.touch_friend_brake(&mut actor, &peer, 1, 0).unwrap();
        let factor = world.parameters().lifecycle.rubbing / 2.5;
        assert_eq!(actor.state.velocity, [factor, -factor]);
        assert_eq!(actor.state.angular_velocity, factor);
        assert_eq!(world.random_snapshot(), random);
        actor.state.velocity = [1.0, -1.0];
        world.touch_friend_reaction(&mut actor, &peer, 1, 0).unwrap();
        let factor = world.parameters().lifecycle.rubbing / 5.0;
        assert_eq!(actor.state.velocity, [factor, -factor]);
    }

    #[test]
    fn eye_exclusion_and_unknown_actuator_argb_do_not_consume_energy_or_rng() {
        let world = world();
        let mut actor = owned_organism(&world, &[ColorKind::Teal, ColorKind::Green]);
        let peer = owned_organism(&world, &[ColorKind::Eye]);
        actor.body.set_reaction_values(1, [1; 36]).unwrap();
        let random = world.random_snapshot();
        world.touch_reaction(&mut actor, &peer, 1, 0).unwrap();
        world.touch_friend_reaction(&mut actor, &peer, 1, 0).unwrap();
        world.touch_brake(&mut actor, &peer, 1, 0).unwrap();
        world.touch_friend_brake(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        peer.body.set_color_argb(0, ColorKind::Green.argb().unwrap()).unwrap();
        actor.body.set_color_argb(0, 0x80008080).unwrap();
        world.touch_reaction(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn injured_self_override_changes_reactions_but_not_brakes() {
        let world = world();
        let mut actor = owned_organism(&world, &[ColorKind::Teal, ColorKind::Spike]);
        let peer = owned_organism(&world, &[ColorKind::Orange]);
        actor.body.set_reaction(1, Channel::Sick as usize, 1).unwrap();
        actor.state.velocity = [1.0, -1.0];
        world.touch_brake(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        world.touch_reaction(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(actor.state.energy, 9.0);
        let factor = world.parameters().lifecycle.rubbing / 5.0;
        assert_eq!(actor.state.velocity, [factor, -factor]);
    }

    #[test]
    fn dodge_gate_uses_bounded_draws_before_payment_and_mode_five_wraps_integer_centers() {
        let world = world();
        let mut actor = owned_organism(&world, &[ColorKind::Teal]);
        let mut peer = owned_organism(&world, &[ColorKind::Green]);
        actor.effects.can_dodge = true;
        actor.body.set_reaction(0, Channel::Friend as usize, 5).unwrap();
        actor.body.set_physical_length(0, 2.0).unwrap();
        actor.state.center = [2.0, 3.0];
        actor.state.integer_center = [i32::MAX, i32::MIN];
        peer.state.center = [5.0, 9.0];
        peer.state.integer_center = [i32::MIN, i32::MAX];
        let mut expected_random = world.random_snapshot();
        assert_eq!(expected_random.bounded(10), 0);
        world.touch_friend_reaction(&mut actor, &peer, 0, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        assert_eq!(world.random_snapshot(), expected_random);
        world.session.0.random.replace(JavaRandom::new(2));
        let mut expected_random = world.random_snapshot();
        assert!(expected_random.bounded(10) > 2);
        expected_random.bounded(2);
        world.touch_friend_reaction(&mut actor, &peer, 0, 0).unwrap();
        assert_eq!(actor.state.velocity, [1.0 / 6.0, -1.0 / 6.0]);
        assert_eq!(actor.state.energy, 9.0);
        assert_eq!(world.random_snapshot(), expected_random);
    }

    #[test]
    fn cyan_actuators_require_exact_eye_contact_color() {
        let world = world();
        let mut actor = owned_organism(&world, &[ColorKind::Cyan, ColorKind::Green]);
        let peer = owned_organism(&world, &[ColorKind::Green]);
        actor.body.set_reaction(1, Channel::Green as usize, 1).unwrap();
        world.touch_reaction(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        actor.body.set_color_argb(1, 0x80004040).unwrap();
        world.touch_reaction(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(actor.state.energy, 10.0);
        actor.body.set_color_argb(1, ColorKind::Eye.argb().unwrap()).unwrap();
        world.touch_reaction(&mut actor, &peer, 1, 0).unwrap();
        assert_eq!(actor.state.energy, 9.0);
    }

    #[test]
    fn reverse_reaction_scan_reports_bounds_before_null_contact_for_both_callers() {
        for friendly in [false, true] {
            let world = world();
            let mut actor = owned_organism(&world, &[ColorKind::Green, ColorKind::Teal]);
            let mut peer = owned_organism(&world, &[ColorKind::Green]);
            actor.body.phenotype_colors = Some(crate::ColorArrayRef::from_slots(vec![None]));
            peer.body.phenotype_colors = None;
            let before = actor.snapshot();
            let random = world.random_snapshot();
            let atmosphere = [world.oxygen.get(), world.carbon_dioxide.get()];
            let result = if friendly {
                world.touch_friend_reaction(&mut actor, &peer, 0, 0)
            } else {
                world.touch_reaction(&mut actor, &peer, 0, 0)
            };
            assert_eq!(result, Err(crate::WorldError::Geometry(
                crate::GeometryError::ColorIndex { segment: 1, length: 1 })));
            assert_eq!(actor.snapshot(), before);
            assert_eq!(world.random_snapshot(), random);
            assert_eq!([world.oxygen.get(), world.carbon_dioxide.get()], atmosphere);
        }
    }

    #[test]
    fn zero_segment_reaction_scan_reads_no_colors_for_both_callers() {
        for friendly in [false, true] {
            let world = world();
            let mut actor = owned_organism(&world, &[ColorKind::Green]);
            let mut peer = owned_organism(&world, &[ColorKind::Green]);
            actor.body.segment_count = 0;
            actor.body.phenotype_colors = None;
            peer.body.phenotype_colors = None;
            let before = actor.snapshot();
            let random = world.random_snapshot();
            let atmosphere = [world.oxygen.get(), world.carbon_dioxide.get()];
            let result = if friendly {
                world.touch_friend_reaction(&mut actor, &peer, 0, 0)
            } else {
                world.touch_reaction(&mut actor, &peer, 0, 0)
            };
            assert_eq!(result, Ok(()));
            assert_eq!(actor.snapshot(), before);
            assert_eq!(world.random_snapshot(), random);
            assert_eq!([world.oxygen.get(), world.carbon_dioxide.get()], atmosphere);
        }
    }

    #[test]
    fn friendly_brake_classifies_actor_before_peer_or_payment() {
        for null_peer in [false, true] {
            let world = world();
            let mut actor = owned_organism(&world, &[ColorKind::Eye]);
            let mut peer = owned_organism(&world, &[ColorKind::Green]);
            actor.body.set_color(0, None).unwrap();
            actor.body.set_reaction(0, Channel::Friend as usize, 1).unwrap();
            actor.state.velocity = [3.0, -2.0];
            actor.state.angular_velocity = 0.5;
            if null_peer { peer.body.phenotype_colors = None; }
            let before = actor.snapshot();
            let random = world.random_snapshot();
            let atmosphere = [world.oxygen.get(), world.carbon_dioxide.get()];
            assert_eq!(world.touch_friend_brake(&mut actor, &peer, 0, 0),
                Err(crate::WorldError::Geometry(
                    crate::GeometryError::MissingPhenotypeColor { segment: 0 })));
            assert_eq!(actor.snapshot(), before);
            assert_eq!(world.random_snapshot(), random);
            assert_eq!([world.oxygen.get(), world.carbon_dioxide.get()], atmosphere);
        }
    }

}
