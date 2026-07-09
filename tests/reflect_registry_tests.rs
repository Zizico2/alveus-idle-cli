use alveus_headless::{GameCommand, register_headless_types};
use alveus_interaction::PlayerSatchel;
use alveus_stats::SanctuaryUpkeep;
use bevy::prelude::*;

#[test]
fn reflect_registry_exposes_headless_control_types() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    register_headless_types(&mut app);

    let registry = app.world().resource::<AppTypeRegistry>();
    let registry = registry.read();

    for type_id in [
        std::any::TypeId::of::<GameCommand>(),
        std::any::TypeId::of::<PlayerSatchel>(),
        std::any::TypeId::of::<SanctuaryUpkeep>(),
    ] {
        assert!(
            registry.get(type_id).is_some(),
            "missing type registration for {type_id:?}"
        );
    }
}
