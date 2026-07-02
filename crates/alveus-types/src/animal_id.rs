use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use moonshine_save::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Component)]
#[require(Save, Unload)]
pub enum AnimalId {
    #[default]
    Polly,
    PushPop,
    Stompy,
    Georgie,
    Siren,
}

impl AnimalId {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnimalId::Polly => "polly",
            AnimalId::PushPop => "push_pop",
            AnimalId::Stompy => "stompy",
            AnimalId::Georgie => "georgie",
            AnimalId::Siren => "siren",
        }
    }
}
