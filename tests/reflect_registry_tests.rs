use alveus_app::{Menu, Pause, Screen};
use alveus_command::{GameCommand, HeadlessRenderTarget};
use alveus_interaction::{CleanAnimal, PlayerSatchel};
use alveus_reflect::register_types;
use alveus_stats::SanctuaryUpkeep;
use alveus_types::{CleanStat, EnrichStat, FeedStat, Stat};
use bevy::prelude::*;
use bevy::reflect::TypePath;

#[test]
fn reflect_registry_exposes_headless_control_types() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    register_types(&mut app);

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

#[test]
fn game_command_preserves_the_legacy_brp_event_path() {
    assert_eq!(
        GameCommand::type_path(),
        "alveus_headless::command::GameCommand"
    );
}

#[test]
fn headless_render_target_preserves_its_legacy_reflect_path() {
    assert_eq!(
        HeadlessRenderTarget::type_path(),
        "alveus_headless::camera::HeadlessRenderTarget"
    );
}
