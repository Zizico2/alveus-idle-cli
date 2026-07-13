//! The pause menu presented over the running gameplay scene.

use alveus_app::Menu;
use alveus_command::GameCommand;
use alveus_menus_models::ListMenuState;
use bevy::{prelude::*, ui_widgets::Activate};

use crate::{
    list_menu::{ListMenuEntry, ListMenuPlugin, ListMenuSpec},
    overlay_menu::{OverlayMenuSpec, spawn_overlay_menu},
};

pub(super) fn plugin(app: &mut App) {
    alveus_app::ensure_plugin(app, ListMenuPlugin);
    app.add_systems(OnEnter(Menu::Pause), spawn_pause_menu)
        .add_systems(
            Update,
            crate::list_menu::sync_action_cursor::<PauseMenuAction>.run_if(in_state(Menu::Pause)),
        )
        .add_observer(activate_pause_menu_action);
}

#[derive(Debug, Clone, Copy)]
enum PauseMenuAction {
    Continue,
    Settings,
    QuitToTitle,
}

impl PauseMenuAction {
    fn label(self) -> &'static str {
        match self {
            Self::Continue => "Continue",
            Self::Settings => "Settings",
            Self::QuitToTitle => "Quit to title",
        }
    }
}

fn spawn_pause_menu(mut commands: Commands) {
    let state = ListMenuState::new([
        PauseMenuAction::Continue,
        PauseMenuAction::Settings,
        PauseMenuAction::QuitToTitle,
    ]);
    let spawned = spawn_overlay_menu(
        &mut commands,
        "Pause Menu",
        &state,
        OverlayMenuSpec::new("Game paused")
            .with_controls("Up/Down Select   Space/Enter/A Confirm   Esc/P/B Resume"),
        ListMenuSpec::actions(),
        |action| action.label().to_string(),
    );
    commands
        .entity(spawned.root)
        .insert(DespawnOnExit(Menu::Pause));
    if let Some(list) = spawned.list.list {
        commands.entity(list).insert(state);
    }
}

fn activate_pause_menu_action(
    activate: On<Activate>,
    entries: Query<&ListMenuEntry>,
    lists: Query<&ListMenuState<PauseMenuAction>>,
    mut commands: Commands,
) {
    let Ok(entry) = entries.get(activate.entity) else {
        return;
    };
    let Ok(state) = lists.get(entry.list) else {
        return;
    };
    let Some(action) = state.options.get(entry.index).copied() else {
        return;
    };
    commands.trigger(match action {
        PauseMenuAction::Continue => GameCommand::Continue,
        PauseMenuAction::Settings => GameCommand::OpenSettings,
        PauseMenuAction::QuitToTitle => GameCommand::QuitToTitle,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{list_menu::ListMenu, overlay_menu::OverlayMenuRoot};
    use bevy::{state::app::StatesPlugin, ui_widgets::Button};

    #[test]
    fn pause_menu_composes_overlay_shell_and_action_list() {
        let mut app = App::new();
        app.add_plugins((StatesPlugin, MinimalPlugins, alveus_app::plugin));
        app.add_plugins(super::plugin);
        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::Pause);
        app.update();

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<OverlayMenuRoot>>()
                .iter(app.world())
                .count(),
            1
        );
        let list = app
            .world_mut()
            .query_filtered::<Entity, (With<ListMenu>, With<ListMenuState<PauseMenuAction>>)>()
            .single(app.world())
            .expect("one pause action list");
        assert_eq!(
            app.world_mut()
                .query_filtered::<&ListMenuEntry, With<Button>>()
                .iter(app.world())
                .filter(|entry| entry.list == list)
                .count(),
            3
        );
    }
}
