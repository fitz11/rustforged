//! Editor command enum for undo/redo operations.

use super::data_types::MapObject;

/// A reversible change to the map. Each user action is one command, so one Ctrl+Z
/// undoes all of it.
#[derive(Clone, Debug, PartialEq)]
pub enum EditorCommand {
    /// Things were added to the map: placed, painted, pasted or drawn
    Add(Vec<MapObject>),
    /// Things were removed from the map: deleted or cut
    Remove(Vec<MapObject>),
    /// Things changed in place, as (before, after): moved, resized, rotated, or edited
    /// in the Properties panel
    Change(Vec<(MapObject, MapObject)>),
}

impl EditorCommand {
    /// A `Change` of the pairs that actually differ, or `None` if nothing changed.
    pub fn change(pairs: impl IntoIterator<Item = (MapObject, MapObject)>) -> Option<Self> {
        let pairs: Vec<_> = pairs
            .into_iter()
            .filter(|(before, after)| before != after)
            .collect();
        (!pairs.is_empty()).then_some(Self::Change(pairs))
    }

    /// Every object the command remembers, mutably.
    pub fn objects_mut(&mut self) -> Box<dyn Iterator<Item = &mut MapObject> + '_> {
        match self {
            Self::Add(objects) | Self::Remove(objects) => Box::new(objects.iter_mut()),
            Self::Change(pairs) => {
                Box::new(pairs.iter_mut().flat_map(|(before, after)| [before, after]))
            }
        }
    }
}
