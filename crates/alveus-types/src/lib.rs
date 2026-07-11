//! Shared identifiers and spatial value types used by the game crate,
//! the `alveus-configs` build script, and tests.

mod animal_id;
mod care_menu_id;
mod chore_id;
mod enclosure_id;
mod item_id;
mod stat;
mod tile;

pub use animal_id::AnimalId;
pub use care_menu_id::CareMenuId;
pub use chore_id::ChoreId;
pub use enclosure_id::EnclosureId;
pub use item_id::ItemId;
pub use stat::{CleanStat, EnrichStat, FeedStat, Stat};
pub use tile::{TileBounds, TilePosition};
