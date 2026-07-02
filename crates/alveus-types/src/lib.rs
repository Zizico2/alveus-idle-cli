//! Shared identifiers and spatial value types used by the game crate,
//! the `alveus-configs` build script, and tests.

mod animal_id;
mod enclosure_id;
mod item_id;
mod tile;

pub use animal_id::AnimalId;
pub use enclosure_id::EnclosureId;
pub use item_id::ItemId;
pub use tile::{TileBounds, TilePosition};
