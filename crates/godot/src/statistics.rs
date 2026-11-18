// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{
    GenomeHandle, OrganismImage, PreviewImageError, RecordKind, StatisticsHandle, StatisticsView,
};
use godot::builtin::VarArray;

const RECORDS: [RecordKind; 6] = [
    RecordKind::AliveChildren,
    RecordKind::HistoricalChildren,
    RecordKind::AliveKills,
    RecordKind::HistoricalKills,
    RecordKind::AliveInfections,
    RecordKind::HistoricalInfections,
];

const PREVIEW_WIDTH: i32 = 100;
const PREVIEW_HEIGHT: i32 = 100;
const PREVIEW_PIXEL_BYTES: usize = 100 * 100 * 3;
const PREVIEW_BATCH_BUDGET: usize = 64 * 1024 * 1024;

// Reserve all six RGB buffers while only one temporary geometry exists at a time
fn record_preview_images(
    genomes: &[Option<GenomeHandle>; 6],
    budget: usize,
) -> Result<[Option<Result<OrganismImage, PreviewImageError>>; 6], &'static str> {
    let image_budget = budget
        .checked_sub((RECORDS.len() - 1) * PREVIEW_PIXEL_BYTES)
        .filter(|bytes| *bytes >= PREVIEW_PIXEL_BYTES)
        .ok_or("Statistics preview budget cannot hold six RGB8 images")?;
    Ok(std::array::from_fn(|index| {
        genomes[index]
            .as_ref()
            .map(|genome| genome.preview_image(PREVIEW_WIDTH, PREVIEW_HEIGHT, image_budget))
    }))
}

pub(super) fn image_fields(image: OrganismImage) -> VarDictionary {
    let mut fields = VarDictionary::new();
    fields.set("width", i64::from(image.width));
    fields.set("height", i64::from(image.height));
    fields.set("rgb", &PackedByteArray::from(image.rgb.as_slice()));
    fields
}

pub(super) struct StatisticsSession {
    pub(super) handle: StatisticsHandle,
    view: StatisticsView,
}

fn outcome(error: Option<&str>, open: bool) -> VarDictionary {
    let mut result = VarDictionary::new();
    result.set("ok", error.is_none());
    result.set("error", error.unwrap_or(""));
    result.set("open", open);
    result
}

#[godot_api(secondary)]
impl BgrSimulation {
    #[func]
    fn statistics_open(&mut self) -> VarDictionary {
        if let Err(error) = self.open_statistics_state() {
            return outcome(Some(&error), self.statistics_session.is_some());
        }
        self.statistics_packet()
    }

    #[func]
    fn statistics_update(&mut self) -> VarDictionary {
        if let Err(error) = self.update_statistics_state() {
            return outcome(Some(&error), self.statistics_session.is_some());
        }
        self.statistics_packet()
    }

    #[func]
    fn statistics_close(&mut self) -> VarDictionary {
        self.statistics_session = None;
        outcome(None, false)
    }

    #[func]
    fn statistics_copy_record(&mut self, index: i64) -> VarDictionary {
        let error = self.copy_statistics_record(index).err();
        let mut result = outcome(error.as_deref(), self.statistics_session.is_some());
        result.set("clipboard_ready", self.clipboard.is_some());
        result
    }

    #[func]
    fn statistics_previews(&self) -> VarDictionary {
        let Some(session) = self.statistics_session.as_ref() else {
            return outcome(Some("Open statistics first"), false);
        };
        let genomes = std::array::from_fn(|index| session.view.record(RECORDS[index]).genome);
        let images = match record_preview_images(&genomes, PREVIEW_BATCH_BUDGET) {
            Ok(images) => images,
            Err(error) => return outcome(Some(error), true),
        };
        let mut previews = VarArray::new();
        for (genome, image) in genomes.into_iter().zip(images) {
            let mut item = VarDictionary::new();
            item.set("present", genome.is_some());
            item.set("error", "");
            item.set("image", &VarDictionary::new());
            if let Some(genome) = genome {
                let (clade, generation) = genome.lineage();
                let display_clade = clade.as_ref().map(biogenesis_core::JavaText::display_utf8_lossy);
                item.set("clade", &display_clade.as_deref().map(|value| GString::from(value).to_variant()).unwrap_or_else(Variant::nil));
                item.set("generation", generation);
                match image.expect("Present record has a prepared image result") {
                    Ok(image) => item.set("image", &image_fields(image)),
                    Err(error) => item.set("error", error.to_string()),
                };
            }
            previews.push(&item.to_variant());
        }
        let mut result = outcome(None, true);
        result.set("previews", &previews);
        result
    }
}

impl BgrSimulation {
    pub(super) fn open_statistics_state(&mut self) -> Result<(), String> {
        if self.settings_applying {
            return Err("Wait for settings application to finish".to_owned());
        }
        self.statistics_session = None;
        let world = self.world.as_ref().ok_or("Initialize a world first")?;
        let handle = world.open_statistics();
        let view = world.refresh_statistics(&handle).map_err(|error| error.to_string())?;
        self.statistics_session = Some(StatisticsSession { handle, view });
        Ok(())
    }

    pub(super) fn update_statistics_state(&mut self) -> Result<(), String> {
        if self.settings_applying {
            return Err("Wait for settings application to finish".to_owned());
        }
        let session = self.statistics_session.as_mut().ok_or("Open statistics first")?;
        let world = self.world.as_ref().ok_or("Initialize a world first")?;
        session.view = world.refresh_statistics(&session.handle).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(super) fn copy_statistics_record(&mut self, index: i64) -> Result<(), String> {
        let index = usize::try_from(index)
            .ok()
            .filter(|index| *index < RECORDS.len())
            .ok_or("Record index must be between zero and five")?;
        let session = self.statistics_session.as_ref().ok_or("Open statistics first")?;
        if let Some(genome) = session.view.record(RECORDS[index]).genome {
            self.clipboard = Some(genome);
        }
        Ok(())
    }

    fn statistics_packet(&self) -> VarDictionary {
        let session = self.statistics_session.as_ref().expect("Opened statistics session");
        let view = &session.view;
        let n = &view.numbers;
        let c = &view.current;
        let mut raw = VarDictionary::new();
        let mut display = VarDictionary::new();
        macro_rules! integer {
            ($key:literal, $value:expr) => {{
                let value = i64::from($value);
                raw.set($key, value);
                display.set($key, number_format::integer(value));
            }};
        }
        macro_rules! real {
            ($key:literal, $value:expr) => {{
                raw.set($key, $value);
                display.set($key, number_format::real($value));
            }};
        }
        integer!("current_time", c.time);
        integer!("retained_time", n.time);
        integer!("current_population", c.population);
        integer!("current_corpses", c.corpses);
        real!("current_oxygen", c.oxygen);
        real!("current_carbon_dioxide", c.carbon_dioxide);
        real!("current_methane", c.methane);
        real!("average_population", n.average_population);
        real!("average_births", n.average_births);
        real!("average_deaths", n.average_deaths);
        real!("average_infections", n.average_infections);
        integer!("created_organisms", n.created_organisms);
        integer!("max_population", n.max_population);
        integer!("max_birth", n.max_birth);
        integer!("max_deaths", n.max_deaths);
        integer!("max_population_time", n.max_population_time);
        integer!("min_population", n.min_population);
        integer!("min_population_time", n.min_population_time);
        integer!("mass_extinctions", n.mass_extinctions);
        real!("max_carbon_dioxide", n.max_carbon_dioxide);
        integer!("max_carbon_dioxide_time", n.max_carbon_dioxide_time);
        real!("min_carbon_dioxide", n.min_carbon_dioxide);
        integer!("min_carbon_dioxide_time", n.min_carbon_dioxide_time);
        real!("max_methane", n.max_methane);
        integer!("max_methane_time", n.max_methane_time);
        real!("min_methane", n.min_methane);
        integer!("min_methane_time", n.min_methane_time);
        real!("max_oxygen", n.max_oxygen);
        integer!("max_oxygen_time", n.max_oxygen_time);
        real!("min_oxygen", n.min_oxygen);
        integer!("min_oxygen_time", n.min_oxygen_time);
        let mut records = VarArray::new();
        for (index, kind) in RECORDS.into_iter().enumerate() {
            let record = view.record(kind);
            let mut item = VarDictionary::new();
            item.set("kind", index as i64);
            item.set("count", record.count);
            item.set("time", &record.time.map(|time| time.to_variant()).unwrap_or_else(Variant::nil));
            item.set("present", record.genome.is_some());
            records.push(&item.to_variant());
        }
        let mut graphs = VarArray::new();
        for (index, series) in view.plot_series.iter().enumerate() {
            let points: Vec<i32> = series.points.iter().flatten().copied().collect();
            let mut item = VarDictionary::new();
            item.set("kind", index as i64);
            item.set("argb", i64::from(series.argb));
            item.set("minimum", series.minimum);
            item.set("maximum", series.maximum);
            item.set("points", &PackedInt32Array::from(points.as_slice()));
            graphs.push(&item.to_variant());
        }
        let counts: Vec<i32> = view.color_counts.iter().map(|entry| entry.count).collect();
        let colors: Vec<i64> = view.color_counts.iter().map(|entry| i64::from(entry.argb)).collect();
        let rectangles: Vec<i32> = view.color_rectangles(640, 25).iter()
            .flat_map(|entry| [entry.bounds.x, entry.bounds.y, entry.bounds.width, entry.bounds.height]).collect();
        let mut result = outcome(None, true);
        result.set("language", self.settings.as_ref().map(|settings| settings.language()).unwrap_or("en"));
        result.set("raw", &raw);
        result.set("display", &display);
        result.set("records", &records);
        result.set("graphs", &graphs);
        result.set("palette_counts", &PackedInt32Array::from(counts.as_slice()));
        result.set("palette_colors", &PackedInt64Array::from(colors.as_slice()));
        result.set("palette_rectangles", &PackedInt32Array::from(rectangles.as_slice()));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biogenesis_core::{Gene, Genome, GenomeProperties, OrganismImageError, RotatedSegment};

    #[test]
    fn record_preview_budget_reserves_six_pixels_and_only_one_temporary_geometry() {
        let genome = GenomeHandle::from_owned(Genome::from_parts(
            &[Gene::new(12.0, 0.0, Some(0xff00ff00), 0, [0; 36]).shared()],
            GenomeProperties { symmetry: 1, ..GenomeProperties::default() },
        ));
        let records = std::array::from_fn(|_| Some(genome.clone()));
        let geometry = std::mem::size_of::<RotatedSegment>() + std::mem::size_of::<u32>();
        let required = 6 * PREVIEW_PIXEL_BYTES + geometry;
        let images = record_preview_images(&records, required).unwrap();
        for image in images {
            let image = image.unwrap().unwrap();
            assert_eq!((image.width, image.height, image.rgb.len()), (100, 100, 30000));
            assert!(image.rgb.chunks_exact(3).any(|pixel| pixel == [0, 255, 0]));
        }
        let images = record_preview_images(&records, required - 1).unwrap();
        for image in images {
            assert_eq!(image, Some(Err(PreviewImageError::Image(
                OrganismImageError::BudgetExceeded {
                    required: PREVIEW_PIXEL_BYTES + geometry,
                    budget: PREVIEW_PIXEL_BYTES + geometry - 1,
                },
            ))));
        }
        assert!(record_preview_images(&records, 6 * PREVIEW_PIXEL_BYTES - 1).is_err());
    }
}
