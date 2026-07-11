use alveus_app::{Menu, Pause, Screen};
use alveus_headless::{GameCommand, register_headless_types};
use alveus_interaction::{CleanAnimal, PlayerSatchel};
use alveus_stats::SanctuaryUpkeep;
use alveus_types::{CleanStat, EnrichStat, FeedStat, Stat};
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
        std::any::TypeId::of::<Stat>(),
        std::any::TypeId::of::<FeedStat>(),
        std::any::TypeId::of::<EnrichStat>(),
        std::any::TypeId::of::<CleanStat>(),
        std::any::TypeId::of::<CleanAnimal>(),
        std::any::TypeId::of::<State<Screen>>(),
        std::any::TypeId::of::<State<Menu>>(),
        std::any::TypeId::of::<State<Pause>>(),
    ] {
        assert!(
            registry.get(type_id).is_some(),
            "missing type registration for {type_id:?}"
        );
    }
}
