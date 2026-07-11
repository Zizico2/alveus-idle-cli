use bevy_reflect::Reflect;

/// Identifies a care item-picker menu opened via an `OpenMenu` tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[type_path = "alveus_types"]
pub enum CareMenuId {
    Fridge,
}

impl CareMenuId {
    pub fn as_str(self) -> &'static str {
        match self {
            CareMenuId::Fridge => "fridge",
        }
    }
}
