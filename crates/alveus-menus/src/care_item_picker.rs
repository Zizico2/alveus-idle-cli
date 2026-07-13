//! Care-specific adapter for the reusable overlay menu presentation.

use alveus_app::Menu;
use alveus_command::GameCommand;
use alveus_components::PlayerSatchel;
use alveus_configs::{SATCHEL_MAX_SLOTS, item_display_name};
use alveus_interaction::{CareMenuState, care_menu_set_cursor};
use alveus_types::CareMenuId;
use bevy::prelude::*;

use crate::{
    list_menu::{ListMenuEntry, ListMenuPlugin, ListMenuSpec, project_selection},
    overlay_menu::{OverlayMenuSpec, spawn_overlay_menu},
};

pub(super) fn plugin(app: &mut App) {
    alveus_app::ensure_plugin(app, ListMenuPlugin);
    app.add_systems(OnEnter(Menu::CareItemPicker), spawn_care_item_picker)
        .add_systems(
            Update,
            sync_care_selection.run_if(in_state(Menu::CareItemPicker)),
        )
        .add_observer(hover_care_row)
        .add_observer(click_care_row);
}

/// Stable marker for lifecycle tests and UI inspection.
#[derive(Component)]
pub struct CareItemPickerOverlay;

#[derive(Component)]
struct CareItemPickerList;

fn menu_title(menu_id: Option<CareMenuId>) -> &'static str {
    match menu_id {
        Some(CareMenuId::Fridge) => "Fridge supplies",
        None => "Choose an item",
    }
}

fn selected_index(care_menu: &CareMenuState) -> Option<usize> {
    care_menu
        .menu_id
        .is_some()
        .then(|| care_menu.list.selected_index())
        .flatten()
}

fn empty_copy(care_menu: &CareMenuState) -> &'static str {
    if care_menu.menu_id.is_none() {
        "This item menu is unavailable. Press Esc to close."
    } else {
        "No items are available. Press Esc to close."
    }
}

fn spawn_care_item_picker(
    mut commands: Commands,
    care_menu: Res<CareMenuState>,
    satchel: Res<PlayerSatchel>,
) {
    let entries = care_menu
        .menu_id
        .is_some()
        .then(|| {
            care_menu
                .list
                .options
                .iter()
                .copied()
                .map(item_display_name)
        })
        .into_iter()
        .flatten();
    let list_state = alveus_menus_models::ListMenuState {
        options: entries.collect(),
        cursor: care_menu.list.cursor,
    };
    let overlay_spec = OverlayMenuSpec::new(menu_title(care_menu.menu_id))
        .with_summary(format!(
            "Satchel: {}/{} slots occupied",
            satchel.occupied_count(),
            SATCHEL_MAX_SLOTS
        ))
        .with_controls("Up/Down Select   Space/Enter/A Take   Esc/P/B Back");
    let list_spec = ListMenuSpec::selection().with_empty_copy(empty_copy(&care_menu));

    let spawned = spawn_overlay_menu(
        &mut commands,
        "Care Item Picker Overlay",
        &list_state,
        overlay_spec,
        list_spec,
        |label| (*label).to_string(),
    );
    commands
        .entity(spawned.root)
        .insert((CareItemPickerOverlay, DespawnOnExit(Menu::CareItemPicker)));
    if let Some(list) = spawned.list.list {
        commands.entity(list).insert(CareItemPickerList);
    }
}

fn sync_care_selection(
    mut commands: Commands,
    care_menu: Res<CareMenuState>,
    list: Single<Entity, With<CareItemPickerList>>,
    entries: Query<
        (Entity, &ListMenuEntry, Has<bevy::ui::Selected>),
        With<bevy::ui_widgets::ListItem>,
    >,
) {
    project_selection(&mut commands, *list, selected_index(&care_menu), &entries);
}

fn hover_care_row(
    over: On<Pointer<Over>>,
    entries: Query<&ListMenuEntry>,
    mut care_menu: ResMut<CareMenuState>,
) {
    if let Ok(entry) = entries.get(over.entity) {
        care_menu_set_cursor(&mut care_menu, entry.index);
    }
}

fn click_care_row(
    mut click: On<Pointer<Click>>,
    entries: Query<&ListMenuEntry>,
    mut care_menu: ResMut<CareMenuState>,
    mut commands: Commands,
) {
    if let Ok(entry) = entries.get(click.entity) {
        click.propagate(false);
        care_menu_set_cursor(&mut care_menu, entry.index);
        commands.trigger(GameCommand::Continue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        list_menu::{ListMenuEmptyState, ListMenuEntry},
        overlay_menu::OverlayMenuRoot,
    };
    use alveus_types::ItemId;
    use bevy::state::app::StatesPlugin;
    use bevy::{input_focus::tab_navigation::TabIndex, ui::Selected, ui_widgets::ListItem};

    fn picker_app(care_menu: CareMenuState) -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.add_plugins(MinimalPlugins);
        app.add_plugins(alveus_app::plugin);
        app.insert_resource(care_menu);
        app.init_resource::<PlayerSatchel>();
        app.add_plugins(super::plugin);
        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::CareItemPicker);
        app.update();
        app
    }

    fn care_state(
        menu_id: Option<CareMenuId>,
        options: impl IntoIterator<Item = ItemId>,
        cursor: usize,
    ) -> CareMenuState {
        let mut state = CareMenuState::new(menu_id, options);
        state.list.cursor = cursor;
        state
    }

    fn care_root(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, (With<CareItemPickerOverlay>, With<OverlayMenuRoot>)>()
            .single(app.world())
            .expect("one care overlay root")
    }

    fn care_entries(app: &mut App) -> Vec<(usize, String, bool)> {
        let mut entry_query = app
            .world_mut()
            .query::<(&ListMenuEntry, Has<Selected>, &Children)>();
        let mut text_query = app.world_mut().query::<&Text>();
        let mut entries = entry_query
            .iter(app.world())
            .map(|(entry, selected, children)| {
                let label = children
                    .iter()
                    .filter_map(|child| text_query.get(app.world(), child).ok())
                    .next()
                    .map(|text| text.as_str().to_string())
                    .expect("entry label");
                (entry.index, label, selected)
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.0);
        entries
    }

    #[test]
    fn entering_and_leaving_picker_owns_exactly_one_reusable_overlay() {
        let mut app = picker_app(care_state(
            Some(CareMenuId::Fridge),
            [ItemId::RawVeggieTub],
            0,
        ));

        care_root(&mut app);
        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::None);
        app.update();

        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<CareItemPickerOverlay>>()
                .iter(app.world())
                .next()
                .is_none()
        );
    }

    #[test]
    fn care_items_map_to_generic_entries_and_cursor_selection() {
        let mut app = picker_app(care_state(
            Some(CareMenuId::Fridge),
            [ItemId::RawVeggieTub, ItemId::TortoiseLeafyGreens],
            0,
        ));
        care_root(&mut app);

        let entries = care_entries(&mut app);
        assert_eq!(
            entries,
            [
                (0, "Lettuce & Veggie Tub".to_string(), true),
                (1, "Tortoise Leafy Greens".to_string(), false),
            ]
        );

        app.world_mut().resource_mut::<CareMenuState>().list.cursor = 1;
        app.update();
        assert_eq!(
            care_entries(&mut app)
                .into_iter()
                .map(|entry| entry.2)
                .collect::<Vec<_>>(),
            [false, true]
        );
    }

    #[test]
    fn care_rows_are_list_items_without_individual_tab_stops() {
        let mut app = picker_app(care_state(
            Some(CareMenuId::Fridge),
            [ItemId::RawVeggieTub, ItemId::TortoiseLeafyGreens],
            0,
        ));
        care_root(&mut app);
        let mut rows = app
            .world_mut()
            .query_filtered::<(Entity, &ListMenuEntry), With<ListItem>>();
        let entries = rows
            .iter(app.world())
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 2);
        assert!(
            entries
                .iter()
                .all(|entity| app.world().get::<TabIndex>(*entity).is_none())
        );
    }

    #[test]
    fn invalid_care_states_supply_domain_specific_empty_copy() {
        for (care_menu, expected) in [
            (
                care_state(Some(CareMenuId::Fridge), [], 0),
                "No items are available. Press Esc to close.",
            ),
            (
                care_state(None, [ItemId::RawVeggieTub], 0),
                "This item menu is unavailable. Press Esc to close.",
            ),
        ] {
            let mut app = picker_app(care_menu);
            care_root(&mut app);
            let copy = app
                .world_mut()
                .query_filtered::<&Text, With<ListMenuEmptyState>>()
                .iter(app.world())
                .next()
                .map(|text| text.as_str().to_string())
                .expect("care empty-state copy");
            assert_eq!(copy, expected);
        }
    }
}
