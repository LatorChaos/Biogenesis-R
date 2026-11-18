// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-only

//! Owned RGB8 genotype previews using GeneticCode.draw geometry and the shared Java2D raster

use super::*;
use crate::{GenomePreview, RotatedSegment};

/// Geometry or framebuffer preparation fails before returning partial pixels
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewImageError {
    Geometry(GeometryError),
    Image(OrganismImageError),
}

impl fmt::Display for PreviewImageError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry(error) => fmt::Display::fmt(error, output),
            Self::Image(error) => fmt::Display::fmt(error, output),
        }
    }
}

impl std::error::Error for PreviewImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Geometry(error) => Some(error),
            Self::Image(error) => Some(error),
        }
    }
}

impl From<GeometryError> for PreviewImageError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

impl From<OrganismImageError> for PreviewImageError {
    fn from(error: OrganismImageError) -> Self {
        Self::Image(error)
    }
}

fn storage(
    width: i32,
    height: i32,
    segments: usize,
    budget: usize,
) -> Result<usize, PreviewImageError> {
    if width <= 0 || height <= 0 {
        return Err(GeometryError::InvalidViewport.into());
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(3))
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or(OrganismImageError::SizeOverflow)?;
    let geometry = segments
        .checked_mul(std::mem::size_of::<RotatedSegment>() + std::mem::size_of::<u32>())
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or(OrganismImageError::SizeOverflow)?;
    let required = pixels
        .checked_add(geometry)
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or(OrganismImageError::SizeOverflow)?;
    if required > budget {
        return Err(OrganismImageError::BudgetExceeded { required, budget }.into());
    }
    Ok(pixels)
}

pub(super) fn from_genome(
    genome: &Genome,
    width: i32,
    height: i32,
    budget: usize,
) -> Result<OrganismImage, PreviewImageError> {
    if width <= 0 || height <= 0 {
        return Err(GeometryError::InvalidViewport.into());
    }
    let (_, segments) = crate::geometry::preview_dimensions(genome)?;
    storage(width, height, segments, budget)?;
    let preview = GenomePreview::new(genome, width, height)?;
    render(&preview, width, height, budget)
}

fn render(
    preview: &GenomePreview,
    width: i32,
    height: i32,
    budget: usize,
) -> Result<OrganismImage, PreviewImageError> {
    assert_eq!(preview.segments.len(), preview.colors.len());
    assert!(preview.scale.is_finite() && preview.scale > 0.0 && preview.scale <= 1.0);
    let bytes = storage(width, height, preview.segments.len(), budget)?;
    let mut rgb = Vec::new();
    rgb.try_reserve_exact(bytes)
        .map_err(|_| OrganismImageError::Allocation)?;
    rgb.resize(bytes, 0);
    for (segment, argb) in preview.segments.iter().zip(&preview.colors) {
        organism_image::raster_preview_segment(
            &mut rgb,
            width as u32,
            height as u32,
            *segment,
            *argb,
            preview.translation,
            preview.scale,
        );
    }
    Ok(OrganismImage {
        width: width as u32,
        height: height as u32,
        rgb,
    })
}

#[cfg(test)]
#[path = "preview_image_tests.rs"]
mod tests;
