//! The main menu (seen on the title screen).

use alveus_app::Menu;
use alveus_command::GameCommand;
use alveus_menus_models::ListMenuState;
use bevy::{prelude::*, ui_widgets::Activate};

use crate::{
    list_menu::{ListMenuEntry, ListMenuPlugin, ListMenuSpec},
    standalone_menu::{StandaloneMenuSpec, spawn_standalone_menu},
};

pub(super) fn plugin(app: &mut App) {
    alveus_app::ensure_plugin(app, ListMenuPlugin);
    app.add_systems(OnEnter(Menu::Main), spawn_main_menu)
        .add_systems(
            Update,
            crate::list_menu::sync_action_cursor::<MainMenuAction>.run_if(in_state(Menu::Main)),
        )
        .add_observer(activate_main_menu_action);
}

#[derive(Debug, Clone, Copy)]
enum MainMenuAction {
    Play,
    Settings,
    Credits,
    Exit,
}

impl MainMenuAction {
    fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Settings => "Settings",
            Self::Credits => "Credits",
            Self::Exit => "Exit",
        }
    }
}

fn spawn_main_menu(mut commands: Commands) {
    let mut actions = vec![
        MainMenuAction::Play,
        MainMenuAction::Settings,
        MainMenuAction::Credits,
    ];
    #[cfg(not(target_family = "wasm"))]
    actions.push(MainMenuAction::Exit);
    let state = ListMenuState::new(actions);
    let spawned = spawn_standalone_menu(
        &mut commands,
        "Main Menu",
        &state,
        StandaloneMenuSpec::new(),
        ListMenuSpec::actions(),
        |action| action.label().to_string(),
    );
    commands
        .entity(spawned.root)
        .insert(DespawnOnExit(Menu::Main));
    if let Some(list) = spawned.list.list {
        commands.entity(list).insert(state);
    }
}

fn activate_main_menu_action(
    activate: On<Activate>,
    entries: Query<&ListMenuEntry>,
    lists: Query<&ListMenuState<MainMenuAction>>,
    mut commands: Commands,
    #[allow(unused_mut)] mut app_exit: MessageWriter<AppExit>,
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
    match action {
        MainMenuAction::Play => commands.trigger(GameCommand::Play),
        MainMenuAction::Settings => commands.trigger(GameCommand::OpenSettings),
        MainMenuAction::Credits => commands.trigger(GameCommand::OpenCredits),
        MainMenuAction::Exit => {
            #[cfg(not(target_family = "wasm"))]
            app_exit.write(AppExit::Success);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{list_menu::ListMenu, standalone_menu::StandaloneMenuRoot};
    use bevy::{
        input_focus::{FocusCause, InputFocus},
        state::app::StatesPlugin,
        ui_widgets::Button,
    };

    #[test]
    fn main_menu_composes_standalone_shell_and_action_list() {
        let mut app = App::new();
        app.add_plugins((StatesPlugin, MinimalPlugins, alveus_app::plugin));
        app.add_plugins(super::plugin);
        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::Main);
        app.update();

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<StandaloneMenuRoot>>()
                .iter(app.world())
                .count(),
            1
        );
        let list = app
            .world_mut()
            .query_filtered::<Entity, (With<ListMenu>, With<ListMenuState<MainMenuAction>>)>()
            .single(app.world())
            .expect("one main action list");
        assert_eq!(
            app.world_mut()
                .query_filtered::<&ListMenuEntry, With<Button>>()
                .iter(app.world())
                .filter(|entry| entry.list == list)
                .count(),
            if cfg!(target_family = "wasm") { 3 } else { 4 }
        );

        let second = app
            .world_mut()
            .query_filtered::<(Entity, &ListMenuEntry), With<Button>>()
            .iter(app.world())
            .find(|(_, entry)| entry.list == list && entry.index == 1)
            .map(|(entity, _)| entity)
            .expect("second action row");
        app.init_resource::<InputFocus>();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(second, FocusCause::Navigated);
        app.update();
        assert_eq!(
            app.world()
                .get::<ListMenuState<MainMenuAction>>(list)
                .expect("main list state")
                .cursor,
            1
        );
    }
}
