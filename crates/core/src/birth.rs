// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::organism::primary_genome;

use crate::java_math::min;
use crate::organism::{GenomeRef, Organism, initialize_life};
use crate::{
    ColorKind, EffectsParams, GROWTH_SCALES, GeneticsParams, Genome, JavaRandom, OrganismState,
    WorldError,
};
use std::{cell::RefCell, rc::Rc};
use crate::organism::same_genome;

pub(crate) struct Birth {
    pub child: Organism,
    pub inherited: Option<GenomeRef>,
    threshold: i32,
    memory: i32,
}

fn tint(parent: &mut Organism, color: ColorKind) {
    parent.set_display_color(Some(color.color_ref().expect("Display tint requires a named source literal")));
}

fn reject_virus(parent: &mut Organism) {
    parent.state.children = 1;
    parent.state.virus_children = 0;
    tint(parent, ColorKind::Spore);
}

/// Prepare an actual inherited body before placement; failed attempts retain source side effects
pub(crate) fn prepare(
    parent: &mut Organism,
    mut initial: OrganismState,
    first: bool,
    random: &mut JavaRandom,
    parameters: &GeneticsParams,
) -> Result<Option<Birth>, WorldError> {
    let original = parent.genome.clone();
    let infected = parent.infected_genome.clone();
    let mut memory = 0;
    let inherited;
    let mut threshold;
    if parent.effects.spore_version > 0 {
        if let Some(infection) = infected.as_ref() {
            let mut delay: i32 = 1;
            if parent.effects.is_reproductive {
                if parent.state.virus_children == 0 {
                    delay = 14_i32.wrapping_add(primary_genome(&original)?.borrow().symmetry() / 2);
                } else if parent.state.virus_children == 1 {
                    if (parent.effects.is_consumer || parent.effects.is_fungus)
                        && infection.borrow().modifiers().plague
                    {
                        delay = 4_i32.wrapping_add(primary_genome(&original)?.borrow().symmetry() / 2);
                    } else {
                        let infected_symmetry = infection.borrow().symmetry();
                        delay = (if infected_symmetry != 2 {
                            120 / infected_symmetry
                        } else {
                            40
                        })
                        .wrapping_sub(11)
                        .wrapping_add(primary_genome(&original)?.borrow().symmetry() / 2);
                    }
                }
            }
            if (first && random.bounded(delay) < 3)
                || (parent.state.virus_children > 0 && !first)
                || parent.effects.spore_time < 0
            {
                threshold = parent.state.reproduction.infected_energy;
                if threshold <= 0 {
                    delay = 2_i32.wrapping_add(primary_genome(&original)?.borrow().symmetry() / 4);
                    if (first && random.bounded(delay) < 1)
                        || (parent.state.virus_children > 0 && !first)
                        || parent.effects.spore_time < 0
                    {
                        threshold = threshold.wrapping_neg();
                    } else {
                        reject_virus(parent);
                        return Ok(None);
                    }
                }
                inherited = Some(Rc::clone(infection));
                memory = if parent.state.virus_children == 0 {
                    1
                } else {
                    0
                };
            } else if parent.state.virus_children != 0 {
                reject_virus(parent);
                return Ok(None);
            } else {
                inherited = original.clone();
                threshold = own_threshold(parent);
                if parent.effects.spore_version <= 2 {
                    parent.infected_genome = None;
                    parent.saved_genome = None;
                    memory = -2;
                } else {
                    memory = -1;
                }
                if parent.effects.is_auburn && first {
                    parent.state.children = parent.state.children.wrapping_sub(1);
                }
            }
        } else if parent.effects.spore_time > 0 {
            inherited = original.clone();
            threshold = own_threshold(parent);
            if parent.effects.spore_version <= 2 {
                memory = -2;
            }
        } else {
            if first {
                parent.effects.spore_time = parent.effects.spore_time.wrapping_neg();
            }
            parent.state.children = 1;
            tint(parent, ColorKind::Flower);
            return Ok(None);
        }
    } else if let Some(infection) = infected.as_ref() {
        inherited = Some(Rc::clone(infection));
        threshold = parent.state.reproduction.infected_energy;
    } else {
        inherited = original.clone();
        threshold = own_threshold(parent);
    }

    let own = same_genome(inherited.as_ref(), original.as_ref());
    let infectious = same_genome(infected.as_ref(), inherited.as_ref());
    initial.energy = if parent.effects.spore_version > 0 && own {
        if parent.effects.spore_version <= 3 || parent.effects.is_blond {
            min(f64::from(parent.effects.spore_time), parent.state.energy)
        } else {
            min(
                f64::from(parent.state.reproduction.energy)
                    * 0.025
                    * f64::from(parent.effects.spore_time),
                parent.state.energy,
            )
        }
    } else if primary_genome(&original)?.borrow().modifiers().selfish && own {
        min(
            f64::from(threshold / 2) / f64::from(parent.state.children),
            parent.state.energy,
        )
    } else if parent.effects.indigo > 0 && infectious {
        let threshold = effective_threshold(parent, threshold);
        let scale = GROWTH_SCALES[crate::geometry::growth_index(parent.body.growth_ratio)?];
        let energy = min(
            (f64::from(threshold) / ((f64::from(parent.effects.indigo) * scale) / 10.0 + 1.0))
                / f64::from(parent.state.children.wrapping_add(1)),
            parent.state.energy,
        );
        let code = primary_genome(&inherited)?.borrow();
        let minimum = if code.symmetry() != 3 || code.mirror() == 0 {
            3.0
        } else {
            4.0
        };
        if energy < minimum {
            parent.infected_genome = None;
            parent.saved_genome = None;
            parent.state.children = 1;
            parent.state.virus_children = 0;
            if parent.state.reproduction.countdown > 20 {
                parent.state.reproduction.countdown = 20;
            }
            return Ok(None);
        }
        energy
    } else {
        min(
            f64::from(effective_threshold(parent, threshold))
                / f64::from(parent.state.children.wrapping_add(1)),
            parent.state.energy,
        )
    };
    let genome = if memory == -2 {
        original.clone()
    } else if memory >= 0 && parent.saved_genome.is_some() {
        parent.saved_genome.clone()
    } else {
        Some(Rc::new(RefCell::new(
            Genome::inherit_with(&primary_genome(&inherited)?.borrow(), random, parameters)
                .map_err(WorldError::from)?,
        )))
    };
    initial.parent_id = parent.id as i32;
    let child = Organism::unplaced(Rc::clone(primary_genome(&genome)?), initial).map_err(WorldError::Geometry)?;
    Ok(Some(Birth {
        child,
        inherited,
        threshold,
        memory,
    }))
}

fn own_threshold(parent: &Organism) -> i32 {
    if parent.state.reproduction.early_energy > 0 {
        parent.state.reproduction.early_energy
    } else {
        parent.state.reproduction.energy
    }
}

fn effective_threshold(parent: &Organism, threshold: i32) -> i32 {
    if parent.effects.jade_factor != 0
        && !parent.effects.is_consumer
        && !parent.effects.is_fungus
        && threshold > parent.state.reproduction.energy
    {
        parent.state.reproduction.energy
    } else {
        threshold
    }
}

impl Birth {
    pub fn failed(&self, parent: &mut Organism) {
        if parent.saved_genome.is_none() && self.memory >= 0 {
            parent.saved_genome = self.child.genome.clone();
        }
    }

    /// Apply successful-placement bookkeeping before life initialization and spore overrides
    pub fn initialize(&mut self, parent: &mut Organism, parameters: &EffectsParams) -> Result<bool, crate::WorldError> {
        let own = same_genome(self.inherited.as_ref(), parent.genome.as_ref());
        let late = parent.effects.reproduce_late;
        if late > 0 {
            if !parent.effects.is_infectious {
                parent.state.reproduction.countdown = 0;
            }
        } else if late < 0 && own {
            if parent.state.reproduction.countdown == parent.state.reproduction.maximum_countdown {
                parent.state.reproduction.countdown =
                    parent.state.reproduction.countdown.wrapping_sub(1);
                if !parent.effects.is_consumer && !parent.effects.is_fungus && late < -1 {
                    parent.effects.reproduce_late = -1;
                } else {
                    parent.effects.reproduce_late = parent.effects.reproduce_late.wrapping_add(1);
                }
            }
            if parent.effects.reproduce_late < 0 {
                self.threshold = 0;
            }
        }
        if parent.effects.spore_version < 0
            && parent.state.virus_children == 0
            && parent
                .infected_genome
                .as_ref()
                .is_some_and(|code| same_genome(Some(code), self.inherited.as_ref()))
            && parent.state.children <= 1
        {
            parent.state.reproduction.countdown = 0;
        }
        if self.memory == 1 {
            parent.infected_genome = None;
            parent.saved_genome = None;
        }
        let child = &mut self.child;
        initialize_life(
            &primary_genome(&child.genome)?.borrow(),
            child.body.segment_count,
            &mut child.state.reproduction,
            parameters,
        )?;
        Ok(if self.threshold <= 4 {
            child.state.age = child
                .state
                .reproduction
                .maximum_age
                .wrapping_add(1)
                .wrapping_mul(512);
            for index in child.body.indices() {
                let kind = if child.body.color_argb(index)? == ColorKind::Eye.argb().unwrap() {
                    ColorKind::Eye
                } else {
                    ColorKind::Brown
                };
                child.body.set_color(index, kind.color_ref())?;
            }
            if self.threshold > 0 {
                parent.state.reproduction.countdown = 0;
            }
            false
        } else {
            child.effects.apply(
                Some(&primary_genome(&child.genome)?.borrow()),
                &mut child.body,
                child.state.age,
                &mut child.state.reproduction,
                parameters,
            )?;
            true
        })
    }

    pub fn update_lineage(&mut self, complexity: i32) -> Result<(), WorldError> {
        // Shared spore genotypes can be identical to their inherited source
        if same_genome(self.child.genome.as_ref(), self.inherited.as_ref()) {
            let mut code = primary_genome(&self.child.genome)?.borrow_mut();
            if code.update_clade() {
                code.set_update_clade(false);
                if code.clade_id().is_none() {
                    return Err(WorldError::InvalidParameters(
                        crate::GeneticsError::MissingClade.to_string(),
                    ));
                }
            }
            return Ok(());
        }
        primary_genome(&self.child.genome)?.borrow_mut()
            .update_lineage(&primary_genome(&self.inherited)?.borrow(), self.child.id as i32, complexity)
            .map_err(WorldError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gene, GenomeProperties};

    #[test]
    fn shared_birth_lineage_rejects_missing_pending_clade() {
        let gene = Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
        let mut code = Genome::from_parts(
            &[gene],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        code.set_update_clade(true);
        let inherited = Rc::new(RefCell::new(code));
        let child = Organism::unplaced(Rc::clone(&inherited), OrganismState::default()).unwrap();
        let mut birth = Birth {
            child,
            inherited: Some(inherited),
            threshold: 10,
            memory: -2,
        };
        assert!(birth.update_lineage(25).is_err());
        assert!(!birth.child.genome.as_ref().unwrap().borrow().update_clade());
        assert!(birth.update_lineage(25).is_ok());
    }
}
