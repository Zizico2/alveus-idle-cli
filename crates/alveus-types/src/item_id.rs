use bevy_reflect::Reflect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum ItemId {
    TortoiseLeafyGreens,
    ChickenGrains,
}

impl ItemId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemId::TortoiseLeafyGreens => "tortoise_leafy_greens",
            ItemId::ChickenGrains => "chicken_grains",
        }
    }
}
