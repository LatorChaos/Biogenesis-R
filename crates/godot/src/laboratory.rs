// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{
    Gene, GenomeDraft, GenomeHandle, GenomeProperties, REACTION_NAMES,
    load_genome_xml, save_genome_xml,
};
use godot::builtin::{PackedStringArray, VarArray, VariantType};
use std::path::Path;

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub(super) struct BgrGenomeDraft {
    draft: Option<GenomeDraft>,
}

#[godot_api]
impl BgrGenomeDraft {
    #[func]
    fn step_gene_value(&mut self, index: i64, field: GString, direction: i64) -> VarDictionary {
        if !matches!(direction, -1 | 1) {
            return outcome(Some("Gene step direction must be -1 or 1"));
        }
        let Some(draft) = self.draft.as_ref() else {
            return outcome(Some("This laboratory draft has closed"));
        };
        let Ok(position) = usize::try_from(index) else {
            return outcome(Some("Gene index must be nonnegative"));
        };
        let Some(gene) = draft.genes().get(position).cloned() else {
            return outcome(Some("The selected gene no longer exists"));
        };
        let name = field.to_string();
        let value = match name.as_str() {
            "length" => {
                let next = gene.length() + direction as f64 * 0.1;
                if !(2.0..=18.0).contains(&next) {
                    return outcome(None);
                }
                next.to_variant()
            }
            "angle_degrees" => {
                let degrees = draft
                    .gene_angle_degrees(position)
                    .expect("Existing draft gene");
                i64::from((degrees + direction as i32).rem_euclid(360)).to_variant()
            }
            "branch" => {
                let next = i64::from(gene.branch()) + direction;
                if !(-1..=999).contains(&next) {
                    return outcome(None);
                }
                next.to_variant()
            }
            _ => {
                let Some(channel) = name
                    .strip_prefix("reaction_")
                    .and_then(|name| REACTION_NAMES.iter().position(|item| *item == name))
                else {
                    return outcome(Some("Unknown gene step field"));
                };
                let next = i64::from(gene.reaction(channel)) + direction;
                if !(0..=5).contains(&next) {
                    return outcome(None);
                }
                next.to_variant()
            }
        };
        self.set_gene_value(index, field, value)
    }

    pub(super) fn set_property_value(
        &mut self,
        field: GString,
        value: Variant,
        mutation_bounds: [i32; 2],
    ) -> VarDictionary {
        let Some(draft) = self.draft.as_mut() else {
            return outcome(Some("This laboratory draft has closed"));
        };
        let mut properties = draft.properties();
        match patch_property(&mut properties, &field.to_string(), &value, mutation_bounds) {
            Ok(()) => {
                draft.set_properties(properties);
                outcome(None)
            }
            Err(message) => outcome(Some(&message)),
        }
    }

    #[func]
    fn set_gene_value(&mut self, index: i64, field: GString, value: Variant) -> VarDictionary {
        let Some(draft) = self.draft.as_mut() else {
            return outcome(Some("This laboratory draft has closed"));
        };
        let Ok(index) = usize::try_from(index) else {
            return outcome(Some("Gene index must be nonnegative"));
        };
        let Some(mut gene) = draft.genes().get(index).cloned() else {
            return outcome(Some("The selected gene no longer exists"));
        };
        let field = field.to_string();
        if field == "angle_degrees" {
            return match integer(&value, 0, 359).and_then(|degrees| {
                draft
                    .set_gene_angle_degrees(index, degrees)
                    .map_err(|e| e.to_string())
            }) {
                Ok(()) => outcome(None),
                Err(message) => outcome(Some(&message)),
            };
        }
        match patch_gene(&mut gene, &field, &value) {
            Ok(()) => match draft.replace_gene(index, gene) {
                Ok(()) => outcome(None),
                Err(error) => outcome(Some(&error.to_string())),
            },
            Err(message) => outcome(Some(&message)),
        }
    }

    #[func]
    fn edit_rows(&mut self, operation: GString, index: i64) -> VarDictionary {
        let Some(draft) = self.draft.as_mut() else {
            return outcome(Some("This laboratory draft has closed"));
        };
        if operation == "clear" {
            draft.clear();
            return outcome(None);
        }
        let Ok(index) = usize::try_from(index) else {
            return outcome(Some("Gene index must be nonnegative"));
        };
        let result = match operation.to_string().as_str() {
            "insert" => draft.insert_gene(index),
            "remove" => draft.remove_gene(index),
            _ => return outcome(Some("Unknown laboratory row operation")),
        };
        match result {
            Ok(()) => outcome(None),
            Err(error) => outcome(Some(&error.to_string())),
        }
    }

    #[func]
    fn import_file(&mut self, path: GString) -> VarDictionary {
        if self.draft.is_none() {
            return outcome(Some("This laboratory draft has closed"));
        }
        match load_genome_xml(Path::new(&path.to_string())) {
            Ok(genome) => match GenomeHandle::from_owned(genome).draft() {
                Ok(draft) => {
                    self.draft = Some(draft);
                    outcome(None)
                }
                Err(error) => outcome(Some(&error.to_string())),
            },
            Err(error) => outcome(Some(&error.to_string())),
        }
    }

    #[func]
    fn cancel(&mut self) {
        self.draft = None;
    }
}

impl BgrSimulation {
    pub(super) fn open_draft(&self, source: &str) -> VarDictionary {
        let Some(world) = self.world.as_ref() else {
            return outcome(Some("Initialize a world first"));
        };
        let selected;
        let genome = match source {
            "empty" => None,
            "clipboard" => self.clipboard.as_ref(),
            "selection" => {
                selected = self.selected.as_ref()
                    .and_then(|actor| actor.owner_world().inspect_genome(actor));
                let Some(selected) = selected.as_ref() else {
                    return outcome(Some("Select an organism to examine its genes"));
                };
                selected.as_ref()
            }
            _ => return outcome(Some("Unknown laboratory source")),
        };
        let draft = match world.new_genome_draft(genome) {
            Ok(draft) => BgrGenomeDraft { draft: Some(draft) },
            Err(error) => return outcome(Some(&error.to_string())),
        };
        let mut result = outcome(None);
        result.set("draft", &Gd::from_object(draft));
        result
    }

    pub(super) fn read_draft(
        &self,
        object: Gd<BgrGenomeDraft>,
        width: i64,
        height: i64,
    ) -> VarDictionary {
        let object = object.bind();
        let Some(draft) = object.draft.as_ref() else {
            return outcome(Some("This laboratory draft has closed"));
        };
        let Some(world) = self.world.as_ref() else {
            return outcome(Some("Initialize a world first"));
        };
        let mut result = outcome(None);
        result.set("properties", &property_fields(draft.properties()));
        let parameters = world.genetics_parameters();
        result.set(
            "mutation_bounds",
            &PackedInt32Array::from(
                [parameters.min_mutation_rate, parameters.max_mutation_rate].as_slice(),
            ),
        );
        let mut genes = VarArray::new();
        let mut editor_warnings = PackedStringArray::new();
        let properties = draft.properties();
        for (key, value) in property_fields(properties).iter_shared() {
            let name = key.to::<GString>().to_string();
            let mut unchanged = properties;
            if let Err(message) = patch_property(
                &mut unchanged,
                &name,
                &value,
                [parameters.min_mutation_rate, parameters.max_mutation_rate],
            ) {
                editor_warnings.push(format!("{name}: {message}; raw value preserved").as_str());
            }
        }
        for (index, gene) in draft.genes().iter().enumerate() {
            let mut fields = gene_fields(gene);
            fields.set(
                "angle_degrees",
                draft
                    .gene_angle_degrees(index)
                    .expect("Existing draft gene"),
            );
            genes.push(&fields.to_variant());
            for (name, value) in std::iter::once(("length".to_owned(), gene.length().to_variant()))
                .chain(std::iter::once((
                    "branch".to_owned(),
                    gene.branch().to_variant(),
                )))
                .chain(REACTION_NAMES.iter().enumerate().map(|(channel, name)| {
                    (
                        format!("reaction_{name}"),
                        gene.reaction(channel).to_variant(),
                    )
                }))
            {
                if let Err(message) = patch_gene(&mut gene.clone(), &name, &value) {
                    editor_warnings.push(
                        format!("Gene {} {name}: {message}; raw value preserved", index + 1)
                            .as_str(),
                    );
                }
            }
        }
        result.set("genes", &genes);
        result.set("editor_warnings", &editor_warnings);
        match world.genome_draft_estimates(draft) {
            Ok(estimates) => {
                result.set("estimated_segments", estimates.segments);
                result.set("estimated_energy", estimates.energy);
                result.set("estimated_life", estimates.life);
                result.set("estimate_error", "");
            }
            Err(error) => result.set("estimate_error", error.to_string()),
        };
        let viewport = i32::try_from(width).ok().zip(i32::try_from(height).ok());
        let preview = match viewport {
            Some((width, height)) if width > 0 && height > 0 => {
                draft.preview_image(width, height, 64 * 1024 * 1024).map_err(|e| e.to_string())
            }
            _ => Err("Preview dimensions must be positive signed 32-bit integers".into()),
        };
        match preview {
            Ok(preview) => {
                result.set("preview", &statistics::image_fields(preview));
                result.set("preview_error", "");
            }
            Err(message) => {
                result.set("preview", &VarDictionary::new());
                result.set("preview_error", message);
            }
        }
        result
    }

    pub(super) fn commit_draft(&mut self, mut object: Gd<BgrGenomeDraft>) -> VarDictionary {
        let Some(draft) = object.bind_mut().draft.take() else {
            return self.response(Some("This laboratory draft has closed"));
        };
        if let Some(genome) = draft.commit() {
            self.clipboard = Some(genome);
        }
        self.response(None)
    }

    pub(super) fn copy_genome(&mut self) -> VarDictionary {
        let selected = self.selected.as_ref()
            .and_then(|actor| actor.owner_world().copy_genome(actor));
        let Some(genome) = selected else {
            return self.response(Some("Select a living organism to copy"));
        };
        self.clipboard = genome;
        self.response(None)
    }

    pub(super) fn place_genome(
        &mut self,
        x: f64,
        y: f64,
        random: bool,
        import: Option<&Path>,
    ) -> VarDictionary {
        let center = match selection_position(x, y) {
            Ok(center) => center,
            Err(message) => return self.response(Some(message)),
        };
        let genome = if let Some(path) = import {
            match load_genome_xml(path) {
                Ok(genome) => Some(GenomeHandle::from_owned(genome)),
                Err(error) => return self.response(Some(&error.to_string())),
            }
        } else {
            self.clipboard.clone()
        };
        let Some(world) = self.world.as_mut() else {
            return self.response(Some("Initialize a world first"));
        };
        let placed = if random {
            world.random_genome_at(center)
        } else if let Some(genome) = genome {
            world.paste_genome_handle(&genome, center)
        } else {
            return self.response(Some("Copy or create a genotype before pasting"));
        };
        match placed {
            Ok(actor) => {
                let mut result = self.response(None);
                result.set("placement_succeeded", actor.is_some());
                result
            }
            Err(error) => self.response(Some(&error.to_string())),
        }
    }

    pub(super) fn export_genome(&self, path: &Path) -> VarDictionary {
        let genome = self.selected.as_ref()
            .and_then(|actor| actor.owner_world().copy_genome(actor))
            .flatten();
        let Some(genome) = genome else {
            return outcome(Some("Select a living organism to export"));
        };
        match save_genome_xml(path, &genome.detached_clone()) {
            Ok(()) => outcome(None),
            Err(error) => outcome(Some(&error.to_string())),
        }
    }
}

pub(super) fn outcome(error: Option<&str>) -> VarDictionary {
    let mut result = VarDictionary::new();
    result.set("ok", error.is_none());
    result.set("error", error.unwrap_or(""));
    result
}

pub(super) fn destination(path: &Path) -> VarDictionary {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            let mut result = outcome(None);
            result.set("exists", true);
            result
        }
        Ok(_) => outcome(Some("The export destination must be a regular file")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut result = outcome(None);
            result.set("exists", false);
            result
        }
        Err(error) => outcome(Some(&format!("Cannot inspect export destination: {error}"))),
    }
}

fn integer(value: &Variant, minimum: i32, maximum: i32) -> Result<i32, String> {
    if value.get_type() != VariantType::INT {
        return Err("Enter an integer".into());
    }
    let value = value
        .try_to::<i64>()
        .map_err(|_| "Integer is not representable")?;
    let value = i32::try_from(value).map_err(|_| "Integer exceeds signed 32-bit limits")?;
    if value < minimum || value > maximum {
        return Err(format!("Enter a value from {minimum} to {maximum}"));
    }
    Ok(value)
}

fn patch_property(
    properties: &mut GenomeProperties,
    field: &str,
    value: &Variant,
    mutation: [i32; 2],
) -> Result<(), String> {
    macro_rules! number {
        ($target:expr, $min:expr, $max:expr) => {{
            $target = integer(value, $min, $max)?;
        }};
    }
    macro_rules! boolean {
        ($target:expr) => {{
            if value.get_type() != VariantType::BOOL {
                return Err("Choose a boolean value".into());
            }
            $target = value
                .try_to::<bool>()
                .map_err(|_| "Invalid boolean value")?;
        }};
    }
    match field {
        "symmetry" => number!(properties.symmetry, 1, 8),
        "mirror" => number!(properties.mirror, 0, 1),
        "mutation_rate" => number!(properties.mutation_rate, mutation[0], mutation[1]),
        "activity" => number!(properties.activity, 0, 2),
        "cream" => number!(properties.modifiers.cream, 1, 3),
        "fallow" => number!(properties.modifiers.fallow, 1, 4),
        "spore" => number!(properties.modifiers.spore, 1, 12),
        "adapt_spore" => number!(properties.modifiers.adapt_spore, 1, 70),
        "plague" => boolean!(properties.modifiers.plague),
        "disperse_children" => boolean!(properties.modifiers.disperse_children),
        "generation_battle" => boolean!(properties.modifiers.generation_battle),
        "sibling_battle" => boolean!(properties.modifiers.sibling_battle),
        "altruist" => boolean!(properties.modifiers.altruist),
        "familial" => boolean!(properties.modifiers.familial),
        "social" => boolean!(properties.modifiers.social),
        "peaceful" => boolean!(properties.modifiers.peaceful),
        "passive" => boolean!(properties.modifiers.passive),
        "clockwise" => boolean!(properties.modifiers.clockwise),
        "mimic_all" => boolean!(properties.modifiers.mimic_all),
        "modifies_pink" => boolean!(properties.modifiers.modifies_pink),
        "modifies_lilac" => boolean!(properties.modifiers.modifies_lilac),
        "modifies_sky" => boolean!(properties.modifiers.modifies_sky),
        "selfish" => boolean!(properties.modifiers.selfish),
        _ => return Err("Unknown genome property".into()),
    }
    Ok(())
}

enum ColorEdit {
    Palette(i64),
    Argb(i64),
}

fn patch_color(gene: &mut Gene, edit: ColorEdit) -> Result<(), String> {
    let color = match edit {
        ColorEdit::Palette(index) => usize::try_from(index)
            .ok()
            .and_then(biogenesis_core::source_palette_color)
            .ok_or_else(|| "Palette index must be from zero to 63".to_owned())?,
        ColorEdit::Argb(value) => {
            let argb = u32::try_from(value)
                .map_err(|_| "ARGB must fit an unsigned 32-bit integer".to_owned())?;
            biogenesis_core::JavaColor::from_argb(argb)
        }
    };
    gene.set_color_handle(Some(color));
    Ok(())
}

fn patch_gene(gene: &mut Gene, field: &str, value: &Variant) -> Result<(), String> {
    match field {
        "length" => {
            if !matches!(value.get_type(), VariantType::FLOAT | VariantType::INT) {
                return Err("Enter a numeric length".into());
            }
            let length = value.try_to::<f64>().map_err(|_| "Invalid length")?;
            if !length.is_finite() || !(2.0..=18.0).contains(&length) {
                return Err("Gene length must be from 2 to 18".into());
            }
            gene.set_length(length);
        }
        "branch" => gene.set_branch(integer(value, -1, 999)?),
        "palette_color" | "color" => {
            let palette = field == "palette_color";
            if value.get_type() != VariantType::INT {
                return Err(if palette {
                    "Palette requires an integer index"
                } else {
                    "Color requires an ARGB integer"
                }
                .into());
            }
            let number = value.try_to::<i64>().map_err(|_| {
                if palette {
                    "Invalid palette index"
                } else {
                    "Invalid ARGB integer"
                }
            })?;
            patch_color(
                gene,
                if palette {
                    ColorEdit::Palette(number)
                } else {
                    ColorEdit::Argb(number)
                },
            )?;
        }
        name => {
            let Some(index) = name
                .strip_prefix("reaction_")
                .and_then(|name| REACTION_NAMES.iter().position(|item| *item == name))
            else {
                return Err("Unknown gene field".into());
            };
            gene.set_reaction(index, integer(value, 0, 5)?);
        }
    }
    Ok(())
}

fn property_fields(properties: GenomeProperties) -> VarDictionary {
    let mut fields = VarDictionary::new();
    macro_rules! values { ($base:expr, $($name:ident),* $(,)?) => { $(fields.set(stringify!($name), $base.$name);)* }; }
    values!(properties, symmetry, mirror, mutation_rate, activity);
    values!(
        properties.modifiers,
        cream,
        fallow,
        spore,
        adapt_spore,
        plague,
        disperse_children,
        generation_battle,
        sibling_battle,
        altruist,
        familial,
        social,
        peaceful,
        passive,
        clockwise,
        mimic_all,
        modifies_pink,
        modifies_lilac,
        modifies_sky,
        selfish
    );
    fields
}

fn gene_fields(gene: &Gene) -> VarDictionary {
    let mut fields = VarDictionary::new();
    fields.set("length", gene.length());
    fields.set("theta", gene.theta());
    if let Some(color) = gene.color() {
        fields.set("color", i64::from(color));
    } else {
        fields.set("color", &Variant::nil());
    }
    fields.set("branch", gene.branch());
    fields.set(
        "reactions",
        &PackedInt32Array::from(gene.reactions().as_slice()),
    );
    fields
}



#[cfg(test)]
mod color_edit_tests {
    use super::*;

    #[test]
    fn actual_color_edit_path_rejects_invalid_input_before_replacing_the_gene_holder() {
        let initial = biogenesis_core::JavaColor::from_argb(0xff00_ff00);
        let mut gene = Gene::new_with_color(4.0, 0.0, Some(initial.clone()), -1, [0; 36]);
        for index in [-1, 64, i64::MAX] {
            assert!(patch_color(&mut gene, ColorEdit::Palette(index)).is_err());
            assert!(gene.color_handle().unwrap().same_instance(&initial));
        }
        patch_color(&mut gene, ColorEdit::Palette(0)).unwrap();
        let fixed = biogenesis_core::source_palette_color(0).unwrap();
        assert!(gene.color_handle().unwrap().same_instance(&fixed));
        patch_color(&mut gene, ColorEdit::Argb(i64::from(fixed.argb()))).unwrap();
        let fresh = gene.color_handle().unwrap().clone();
        assert_eq!(fresh, fixed);
        assert!(!fresh.same_instance(&fixed));
        for value in [-1, i64::from(u32::MAX) + 1] {
            assert!(patch_color(&mut gene, ColorEdit::Argb(value)).is_err());
            assert!(gene.color_handle().unwrap().same_instance(&fresh));
        }
        patch_color(&mut gene, ColorEdit::Argb(0)).unwrap();
        assert_eq!(gene.color(), Some(0));
        patch_color(&mut gene, ColorEdit::Argb(i64::from(u32::MAX))).unwrap();
        assert_eq!(gene.color(), Some(u32::MAX));
    }
}
