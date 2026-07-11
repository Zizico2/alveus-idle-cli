//! The main menu (seen on the title screen).

use bevy::prelude::*;

use alveus_app::{Menu, Screen};
use alveus_asset_tracking::ResourceHandles;
use alveus_collision::{CollisionMasks, collision_ready};
use alveus_theme::widget;

#[derive(Event, Debug, Clone, Copy, Reflect)]
pub struct PlayClickEvent;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Main), spawn_main_menu);
    app.add_observer(handle_play_click);
}

fn spawn_main_menu(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Main Menu"),
        GlobalZIndex(2),
        DespawnOnExit(Menu::Main),
        #[cfg(not(target_family = "wasm"))]
        children![
            widget::button("Play", enter_loading_or_gameplay_screen),
            widget::button("Settings", open_settings_menu),
            widget::button("Credits", open_credits_menu),
            widget::button("Exit", exit_app),
        ],
        #[cfg(target_family = "wasm")]
        children![
            widget::button("Play", enter_loading_or_gameplay_screen),
            widget::button("Settings", open_settings_menu),
            widget::button("Credits", open_credits_menu),
        ],
    ));
}

fn enter_loading_or_gameplay_screen(_: On<Pointer<Click>>, mut commands: Commands) {
    commands.trigger(PlayClickEvent);
}

pub fn handle_play_click(
    _: On<PlayClickEvent>,
    resource_handles: Res<ResourceHandles>,
    masks: Option<Res<CollisionMasks>>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    // LevelAssets/InteriorAssets may insert before Tiled maps finish (they are not
    // `#[dependency]`-gated). When CollisionMasks is present, require masks before
    // skipping Loading; when absent (minimal tests without CollisionPlugin), do not gate.
    let collision_ok = match masks.as_deref() {
        None => true,
        Some(m) => collision_ready(m),
    };
    if resource_handles.is_all_done() && collision_ok {
        next_screen.set(Screen::Gameplay);
    } else {
        next_screen.set(Screen::Loading);
    }
}

fn open_settings_menu(_: On<Pointer<Click>>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Settings);
}

fn open_credits_menu(_: On<Pointer<Click>>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Credits);
}

#[cfg(not(target_family = "wasm"))]
fn exit_app(_: On<Pointer<Click>>, mut app_exit: MessageWriter<AppExit>) {
    app_exit.write(AppExit::Success);
}
