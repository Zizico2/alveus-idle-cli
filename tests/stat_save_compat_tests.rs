use alveus_app::Screen;
use alveus_stats::{AnimalId, AnimalStats, EnclosureId, EnclosureStats, SavePath, StatsPlugin};
use alveus_types::Stat;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

#[test]
fn old_scalar_stat_save_loads_into_stat_fields() {
    let save_path = "stat_save_compat_scalar.ron";
    let _ = std::fs::remove_file(save_path);
    std::fs::write(
        save_path,
        r#"(
  resources: {},
  entities: {
    4294967295: (
      components: {
        "alveus_types::AnimalId": Polly,
        "alveus_stats::AnimalStats": (
          hunger: 495,
          happiness: 500,
        ),
      },
    ),
    4294967294: (
      components: {
        "alveus_types::EnclosureId": PushPopEnclosure,
        "alveus_stats::EnclosureStats": (
          cleanliness: 495,
        ),
      },
    ),
  },
)"#,
    )
    .expect("write scalar stat fixture");

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(alveus_app::plugin);
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins(StatsPlugin);

    app.insert_resource(NextState::Pending(Screen::Gameplay));
    for _ in 0..5 {
        app.update();
    }

    let polly = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&entity| app.world().get::<AnimalId>(entity) == Some(&AnimalId::Polly))
        .expect("loaded Polly entity");
    let animal_stats = app
        .world()
        .get::<AnimalStats>(polly)
        .expect("loaded animal stats");
    assert_eq!(animal_stats.hunger, Stat(495));
    assert_eq!(animal_stats.happiness, Stat(500));

    let push_pop_enclosure = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&entity| {
            app.world().get::<EnclosureId>(entity) == Some(&EnclosureId::PushPopEnclosure)
        })
        .expect("loaded Push Pop enclosure entity");
    let enclosure_stats = app
        .world()
        .get::<EnclosureStats>(push_pop_enclosure)
        .expect("loaded enclosure stats");
    assert_eq!(enclosure_stats.cleanliness, Stat(495));

    let _ = std::fs::remove_file(save_path);
}
