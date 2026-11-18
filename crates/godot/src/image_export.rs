// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::OrganismImage;
use godot::builtin::VariantType;
use godot::classes::{ClassDb, RefCounted};
use godot::obj::Singleton;

const IMAGE_BUDGET: usize = 64 * 1024 * 1024;
const PNG_BUDGET: usize = 96 * 1024 * 1024;

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub(super) struct BgrImageExport {
    image: Option<OrganismImage>,
}

#[godot_api]
impl BgrImageExport {
    #[func]
    fn snapshot(&self) -> VarDictionary {
        let Some(image) = self.image.as_ref() else {
            return laboratory::outcome(Some("This image export has no prepared organism image"));
        };
        let mut result = laboratory::outcome(None);
        result.set("width", image.width);
        result.set("height", image.height);
        result.set("rgb", &PackedByteArray::from(image.rgb.as_slice()));
        result
    }

    #[func]
    fn save_to_file(&self, path: GString) -> VarDictionary {
        let Some(image) = self.image.as_ref() else {
            return laboratory::outcome(Some("This image export has no prepared organism image"));
        };
        let bytes = match encode_png(image) {
            Ok(bytes) => bytes,
            Err(error) => return laboratory::outcome(Some(&error)),
        };
        match biogenesis_core::replace_file_bytes(
            std::path::Path::new(&path.to_string()),
            bytes.as_slice(),
        ) {
            Ok(()) => laboratory::outcome(None),
            Err(error) => laboratory::outcome(Some(&error.to_string())),
        }
    }
}

fn encode_png(image: &OrganismImage) -> Result<PackedByteArray, String> {
    let mut class_db = ClassDb::singleton();
    let maximum_width = class_db.class_get_integer_constant("Image", "MAX_WIDTH");
    let maximum_height = class_db.class_get_integer_constant("Image", "MAX_HEIGHT");
    let rgb8 = class_db.class_get_integer_constant("Image", "FORMAT_RGB8");
    if maximum_width <= 0 || maximum_height <= 0 || rgb8 <= 0 {
        return Err("The native Image limits or RGB8 format are unavailable".into());
    }
    if image.width == 0
        || image.height == 0
        || i64::from(image.width) > maximum_width
        || i64::from(image.height) > maximum_height
    {
        return Err("Image dimensions exceed the PNG encoder limits".into());
    }
    let expected_bytes = usize::try_from(image.width)
        .ok()
        .and_then(|width| {
            usize::try_from(image.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(3));
    if expected_bytes != Some(image.rgb.len()) || image.rgb.len() > IMAGE_BUDGET {
        return Err("Image pixel storage is inconsistent or exceeds 64 MiB".into());
    }
    let arguments = [
        i64::from(image.width).to_variant(),
        i64::from(image.height).to_variant(),
        false.to_variant(),
        rgb8.to_variant(),
        PackedByteArray::from(image.rgb.as_slice()).to_variant(),
    ];
    let created = class_db
        .try_class_call_static("Image", "create_from_data", &arguments)
        .map_err(|error| format!("Cannot create the native organism image: {error}"))?;
    if created.get_type() != VariantType::OBJECT {
        return Err("The native Image constructor returned an invalid value".into());
    }
    let mut native = created
        .try_to::<Gd<RefCounted>>()
        .map_err(|error| format!("Cannot retain the native organism image: {error}"))?;
    if !native.is_class("Image") {
        return Err("The native Image constructor returned a different object class".into());
    }
    let empty = native
        .try_call("is_empty", &[])
        .map_err(|error| format!("Cannot inspect the native organism image: {error}"))?;
    if empty.get_type() != VariantType::BOOL {
        return Err("The native Image status returned an invalid value".into());
    }
    if empty
        .try_to::<bool>()
        .map_err(|error| format!("Cannot read the native Image status: {error}"))?
    {
        return Err("Cannot create the organism image".into());
    }
    let encoded = native
        .try_call("save_png_to_buffer", &[])
        .map_err(|error| format!("Cannot encode the organism image as PNG: {error}"))?;
    if encoded.get_type() != VariantType::PACKED_BYTE_ARRAY {
        return Err("The native PNG encoder returned an invalid value".into());
    }
    let bytes = encoded
        .try_to::<PackedByteArray>()
        .map_err(|error| format!("Cannot retain the native PNG data: {error}"))?;
    if bytes.is_empty()
        || bytes.len() > PNG_BUDGET
        || !bytes.as_slice().starts_with(b"\x89PNG\r\n\x1a\n")
        || !bytes
            .as_slice()
            .ends_with(b"\x00\x00\x00\x00IEND\xae\x42\x60\x82")
    {
        return Err("The native PNG encoder failed or exceeded 96 MiB".into());
    }
    Ok(bytes)
}

impl BgrSimulation {
    pub(super) fn selected_image(&self) -> VarDictionary {
        let Some((_, actor)) = self.world.as_ref().zip(self.selected.as_ref()) else {
            return laboratory::outcome(Some("Select a living organism to save its image"));
        };
        match actor.owner_world().organism_image(actor, IMAGE_BUDGET) {
            Ok(image) => {
                let mut result = laboratory::outcome(None);
                result.set(
                    "export",
                    &Gd::from_object(BgrImageExport { image: Some(image) }),
                );
                result
            }
            Err(error) => laboratory::outcome(Some(&error.to_string())),
        }
    }
}
