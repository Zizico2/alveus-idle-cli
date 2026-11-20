//! The settings menu.
//!
//! Additional settings and accessibility options should go here.

use bevy::prelude::*;
use bevy::scene::Ready;
use bevy::ui_widgets::Activate;

use alveus_app::Menu;
use alveus_command::GameCommand;
use alveus_theme::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Settings), spawn_settings_menu);
    app.add_systems(
        Update,
        update_global_volume_label
            .run_if(in_state(Menu::Settings))
            .run_if(resource_changed::<GlobalVolume>),
    );
}

fn spawn_settings_menu(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        @widget::ui_root("Settings Menu")
        GlobalZIndex(2)
        DespawnOnExit::<Menu>(Menu::Settings)
        on(initialize_global_volume_label)
        Children [
            @widget::header("Settings")
            --
            @settings_grid()
            --
            @widget::button_autofocus("Back", go_back_on_click)
        ]
    });
}

fn settings_grid() -> impl Scene {
    bsn! {
        Name::new("Settings Grid")
        Node {
            display: Display::Grid,
            row_gap: px(10),
            column_gap: px(30),
            grid_template_columns: RepeatedGridTrack::px::<Vec<RepeatedGridTrack>> (2, 400.0),
        }
        Children [
            @widget::label("Master Volume")
            Node { justify_self: JustifySelf::End }
            --
            @global_volume_widget()
        ]
    }
}

fn global_volume_widget() -> impl Scene {
    bsn! {
        Name::new("Global Volume Widget")
        Node {
            justify_self: JustifySelf::Start,
        }
        Children [
            @widget::button_small("-", lower_global_volume)
            --
            Name::new("Current Volume")
            Node {
                padding: UiRect::horizontal(px(10)),
                justify_content: JustifyContent::Center,
            }
            Children [ @widget::label("") GlobalVolumeLabel ]
            --
            @widget::button_small("+", raise_global_volume)
        ]
    }
}

fn lower_global_volume(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::AdjustVolume { delta: -0.1 });
}

fn raise_global_volume(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::AdjustVolume { delta: 0.1 });
}

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
struct GlobalVolumeLabel;

// Ready runs after the scene's descendants exist, including on reopening the
// menu while GlobalVolume itself has not changed.
fn initialize_global_volume_label(
    _: On<Ready>,
    global_volume: Res<GlobalVolume>,
    mut labels: Query<&mut Text, With<GlobalVolumeLabel>>,
) {
    for mut label in &mut labels {
        label.0 = volume_label(&global_volume);
    }
}

fn volume_label(global_volume: &GlobalVolume) -> String {
    let percent = 100.0 * global_volume.volume.to_linear();
    format!("{percent:3.0}%")
}

fn update_global_volume_label(
    global_volume: Res<GlobalVolume>,
    mut label: Single<&mut Text, With<GlobalVolumeLabel>>,
) {
    label.0 = volume_label(&global_volume);
}

fn go_back_on_click(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::Back);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{audio::Volume, scene::ScenePlugin, state::app::StatesPlugin};

    fn settings_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            AssetPlugin::default(),
            ScenePlugin,
            alveus_app::plugin,
            super::plugin,
        ));
        app.init_resource::<GlobalVolume>();
        app
    }

    fn label(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&Text, With<GlobalVolumeLabel>>()
            .single(app.world())
            .expect("volume label spawned by scene")
            .0
            .clone()
    }

    #[test]
    fn ready_initializes_volume_label_on_every_menu_open() {
        let mut app = settings_app();
        app.world_mut().resource_mut::<GlobalVolume>().volume = Volume::Linear(0.5);
        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::Settings);
        app.update();
        assert_eq!(label(&mut app), " 50%");

        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::None);
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<GlobalVolumeLabel>>()
                .iter(app.world())
                .next()
                .is_none()
        );

        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::Settings);
        app.update();
        assert_eq!(
            label(&mut app),
            " 50%",
            "Ready initializes unchanged volume after reopening"
        );

        app.world_mut().resource_mut::<GlobalVolume>().volume = Volume::Linear(0.25);
        app.update();
        assert_eq!(label(&mut app), " 25%");
    }
}
