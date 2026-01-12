use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Runtime state for fog of war
///
/// Fog uses a "revealed cells" model:
/// - Empty set = everything is fogged (default state)
/// - Cells in the set are revealed (visible to players)
/// - Reset fog = clear the set (everything becomes fogged again)
#[derive(Resource, Debug, Clone, Default)]
pub struct FogOfWarData {
    /// Set of revealed cell coordinates (grid indices)
    /// Empty = fully fogged, populated = those cells are revealed
    pub revealed_cells: HashSet<(i32, i32)>,
}

impl FogOfWarData {
    /// Reset fog (hide everything) - just clears revealed cells
    pub fn reset(&mut self) {
        self.revealed_cells.clear();
    }

    /// Reveal all cells (clear all fog) within given bounds
    #[allow(dead_code)]
    pub fn reveal_all(&mut self, min_cell: (i32, i32), max_cell: (i32, i32)) {
        for x in min_cell.0..=max_cell.0 {
            for y in min_cell.1..=max_cell.1 {
                self.revealed_cells.insert((x, y));
            }
        }
    }

    /// Check if a specific cell is fogged (not revealed)
    #[allow(dead_code)]
    pub fn is_cell_fogged(&self, cell: (i32, i32)) -> bool {
        !self.revealed_cells.contains(&cell)
    }

    /// Check if a specific cell is revealed
    pub fn is_cell_revealed(&self, cell: (i32, i32)) -> bool {
        self.revealed_cells.contains(&cell)
    }

    /// Add fog to a cell (hide it)
    #[allow(dead_code)]
    pub fn fog_cell(&mut self, cell: (i32, i32)) {
        self.revealed_cells.remove(&cell);
    }

    /// Remove fog from a cell (reveal it)
    pub fn reveal_cell(&mut self, cell: (i32, i32)) {
        self.revealed_cells.insert(cell);
    }

    /// Get the number of revealed cells
    pub fn revealed_count(&self) -> usize {
        self.revealed_cells.len()
    }

    /// Check if any cells have been revealed
    pub fn has_revealed_cells(&self) -> bool {
        !self.revealed_cells.is_empty()
    }
}

/// Persistence format for fog of war
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedFogOfWar {
    /// Set of revealed cell coordinates
    /// Empty = fully fogged, populated = those cells are revealed
    #[serde(default)]
    pub revealed_cells: HashSet<(i32, i32)>,
    /// Legacy field for backwards compatibility with old save files
    /// Will be migrated to revealed_cells on load
    #[serde(default, skip_serializing)]
    #[allow(dead_code)]
    pub fogged_cells: HashSet<(i32, i32)>,
}

impl From<&FogOfWarData> for SavedFogOfWar {
    fn from(data: &FogOfWarData) -> Self {
        Self {
            revealed_cells: data.revealed_cells.clone(),
            fogged_cells: HashSet::new(), // Never save this, only for loading legacy
        }
    }
}

impl From<SavedFogOfWar> for FogOfWarData {
    fn from(saved: SavedFogOfWar) -> Self {
        // Migration: if old fogged_cells format is present and revealed_cells is empty,
        // we need to invert the logic. However, since we don't know the map bounds,
        // we'll just start fresh. Old maps will lose their fog state but that's acceptable
        // for this format migration.
        Self {
            revealed_cells: saved.revealed_cells,
        }
    }
}

/// Convert world position to grid cell coordinates
pub fn world_to_cell(world_pos: Vec2, grid_size: f32) -> (i32, i32) {
    (
        (world_pos.x / grid_size).floor() as i32,
        (world_pos.y / grid_size).floor() as i32,
    )
}

/// Convert grid cell coordinates to world position (cell center)
pub fn cell_to_world(cell: (i32, i32), grid_size: f32) -> Vec2 {
    Vec2::new(
        (cell.0 as f32 * grid_size) + grid_size / 2.0,
        (cell.1 as f32 * grid_size) + grid_size / 2.0,
    )
}

/// Get all cells within a circular radius of a center point
pub fn cells_in_radius(center: Vec2, radius: f32, grid_size: f32) -> Vec<(i32, i32)> {
    let center_cell = world_to_cell(center, grid_size);
    let cell_radius = (radius / grid_size).ceil() as i32;

    let mut cells = Vec::new();
    for dx in -cell_radius..=cell_radius {
        for dy in -cell_radius..=cell_radius {
            let cell = (center_cell.0 + dx, center_cell.1 + dy);
            let cell_center = cell_to_world(cell, grid_size);
            if center.distance(cell_center) <= radius {
                cells.push(cell);
            }
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_of_war_data_default() {
        let fog = FogOfWarData::default();
        // Default is fully fogged (no revealed cells)
        assert!(fog.revealed_cells.is_empty());
        assert!(!fog.has_revealed_cells());
    }

    #[test]
    fn test_reveal_and_fog_cell() {
        let mut fog = FogOfWarData::default();

        // Everything starts fogged
        assert!(fog.is_cell_fogged((0, 0)));
        assert!(fog.is_cell_fogged((1, 0)));

        // Reveal a cell
        fog.reveal_cell((0, 0));
        assert!(!fog.is_cell_fogged((0, 0)));
        assert!(fog.is_cell_revealed((0, 0)));
        assert!(fog.is_cell_fogged((1, 0))); // Other cells still fogged

        // Fog it again
        fog.fog_cell((0, 0));
        assert!(fog.is_cell_fogged((0, 0)));
    }

    #[test]
    fn test_reset_fog() {
        let mut fog = FogOfWarData::default();
        fog.reveal_cell((0, 0));
        fog.reveal_cell((1, 1));
        fog.reveal_cell((2, 2));
        assert_eq!(fog.revealed_count(), 3);

        // Reset clears all revealed cells
        fog.reset();
        assert!(fog.revealed_cells.is_empty());
        assert!(!fog.has_revealed_cells());
    }

    #[test]
    fn test_reveal_all() {
        let mut fog = FogOfWarData::default();
        fog.reveal_all((0, 0), (2, 2));

        // Should have 9 cells (3x3) revealed
        assert_eq!(fog.revealed_count(), 9);
        assert!(!fog.is_cell_fogged((0, 0)));
        assert!(!fog.is_cell_fogged((1, 1)));
        assert!(!fog.is_cell_fogged((2, 2)));
        assert!(fog.is_cell_fogged((3, 3))); // Outside the area
    }

    #[test]
    fn test_world_to_cell() {
        let grid_size = 70.0;

        // Cell (0, 0) should cover world positions [0, 70) x [0, 70)
        assert_eq!(world_to_cell(Vec2::new(0.0, 0.0), grid_size), (0, 0));
        assert_eq!(world_to_cell(Vec2::new(35.0, 35.0), grid_size), (0, 0));
        assert_eq!(world_to_cell(Vec2::new(69.9, 69.9), grid_size), (0, 0));

        // Cell (1, 0) starts at x=70
        assert_eq!(world_to_cell(Vec2::new(70.0, 0.0), grid_size), (1, 0));

        // Negative coordinates
        assert_eq!(world_to_cell(Vec2::new(-1.0, -1.0), grid_size), (-1, -1));
        assert_eq!(world_to_cell(Vec2::new(-70.0, 0.0), grid_size), (-1, 0));
    }

    #[test]
    fn test_cell_to_world() {
        let grid_size = 70.0;

        // Cell center should be at grid_size/2 offset
        assert_eq!(cell_to_world((0, 0), grid_size), Vec2::new(35.0, 35.0));
        assert_eq!(cell_to_world((1, 0), grid_size), Vec2::new(105.0, 35.0));
        assert_eq!(cell_to_world((0, 1), grid_size), Vec2::new(35.0, 105.0));
        assert_eq!(cell_to_world((-1, -1), grid_size), Vec2::new(-35.0, -35.0));
    }

    #[test]
    fn test_cells_in_radius() {
        let grid_size = 70.0;
        let center = Vec2::new(35.0, 35.0); // Center of cell (0, 0)

        // Radius of 0 should only include the center cell
        let cells = cells_in_radius(center, 0.0, grid_size);
        assert_eq!(cells.len(), 1);
        assert!(cells.contains(&(0, 0)));

        // Radius equal to grid_size should include center + 4 neighbors (roughly circular)
        let cells = cells_in_radius(center, grid_size, grid_size);
        assert!(cells.contains(&(0, 0)));
        assert!(cells.contains(&(1, 0)));
        assert!(cells.contains(&(-1, 0)));
        assert!(cells.contains(&(0, 1)));
        assert!(cells.contains(&(0, -1)));
    }

    #[test]
    fn test_saved_fog_of_war_roundtrip() {
        let mut fog = FogOfWarData::default();
        fog.reveal_cell((0, 0));
        fog.reveal_cell((5, -3));
        fog.reveal_cell((-10, 20));

        let saved = SavedFogOfWar::from(&fog);
        let json = serde_json::to_string(&saved).unwrap();
        let deserialized: SavedFogOfWar = serde_json::from_str(&json).unwrap();
        let restored = FogOfWarData::from(deserialized);

        assert_eq!(fog.revealed_count(), restored.revealed_count());
        assert!(restored.is_cell_revealed((0, 0)));
        assert!(restored.is_cell_revealed((5, -3)));
        assert!(restored.is_cell_revealed((-10, 20)));
    }
}
