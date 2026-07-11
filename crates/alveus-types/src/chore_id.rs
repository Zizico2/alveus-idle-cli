use bevy_reflect::Reflect;

/// Identifies a mini-chore interaction (prep taps, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[type_path = "alveus_types"]
pub enum ChoreId {
    ChopVeggies,
}

impl ChoreId {
    pub fn as_str(self) -> &'static str {
        match self {
            ChoreId::ChopVeggies => "chop_veggies",
        }
    }
}
