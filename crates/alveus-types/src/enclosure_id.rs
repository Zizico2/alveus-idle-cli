use std::fmt;
use std::hash::{Hash, Hasher};

use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use moonshine_save::prelude::*;
use phf_shared::{FmtConst, PhfBorrow, PhfHash};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Component)]
#[require(Save, Unload)]
pub enum EnclosureId {
    #[default]
    NutritionHousePlaypen,
    PushPopEnclosure,
    Pasture,
    ReptileEnclosure,
}

impl EnclosureId {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnclosureId::NutritionHousePlaypen => "nutrition_house_playpen",
            EnclosureId::PushPopEnclosure => "push_pop_enclosure",
            EnclosureId::Pasture => "pasture",
            EnclosureId::ReptileEnclosure => "reptile_enclosure",
        }
    }
}

impl PhfHash for EnclosureId {
    fn phf_hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

impl PhfBorrow<EnclosureId> for EnclosureId {
    fn borrow(&self) -> &EnclosureId {
        self
    }
}

impl FmtConst for EnclosureId {
    fn fmt_const(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "alveus_types::EnclosureId::{:?}", self)
    }
}
