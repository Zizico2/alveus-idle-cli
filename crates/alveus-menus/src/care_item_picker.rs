//! Care-specific adapter for the reusable overlay menu presentation.

use alveus_app::Menu;
use alveus_components::{CareMenuState, PlayerSatchel};
use alveus_configs::{SATCHEL_MAX_SLOTS, item_display_name};
use alveus_types::CareMenuId;
use bevy::prelude::*;

use crate::overlay_menu::{
    OverlayMenuSelection, OverlayMenuSpec, OverlayMenuSystems, spawn_overlay_menu,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::CareItemPicker), spawn_care_item_picker)
        .add_systems(
            Update,
            sync_care_selection
                .run_if(in_state(Menu::CareItemPicker))
                .in_set(OverlayMenuSystems::SyncSelection),
        );
}

/// Stable marker for lifecycle tests and UI inspection.
#[derive(Component)]
pub struct CareItemPickerOverlay;

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
        .then_some(care_menu.cursor)
        .filter(|index| *index < care_menu.options.len())
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
        .then(|| care_menu.options.iter().copied().map(item_display_name))
        .into_iter()
        .flatten();
    let spec = OverlayMenuSpec::new(menu_title(care_menu.menu_id), entries)
        .with_summary(format!(
            "Satchel: {}/{} slots occupied",
            satchel.occupied_count(),
            SATCHEL_MAX_SLOTS
        ))
        .with_selected(selected_index(&care_menu))
        .with_empty_copy(empty_copy(&care_menu))
        .with_controls("↑/↓ Select   Space/Enter Take   Esc Back");

    let root = spawn_overlay_menu(&mut commands, "Care Item Picker Overlay", spec);
    commands
        .entity(root)
        .insert((CareItemPickerOverlay, DespawnOnExit(Menu::CareItemPicker)));
}

fn sync_care_selection(
    care_menu: Res<CareMenuState>,
    mut overlay: Single<&mut OverlayMenuSelection, With<CareItemPickerOverlay>>,
) {
    if !care_menu.is_changed() {
        return;
    }
    let selected = selected_index(&care_menu);
    if overlay.selected() != selected {
        overlay.set(selected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay_menu::{
        OverlayMenuEmptyState, OverlayMenuEntry, OverlayMenuEntryState, OverlayMenuRoot,
    };
    use alveus_types::ItemId;
    use bevy::state::app::StatesPlugin;

    fn picker_app(care_menu: CareMenuState) -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.add_plugins(MinimalPlugins);
        app.add_plugins(alveus_app::plugin);
        app.insert_resource(care_menu);
        app.init_resource::<PlayerSatchel>();
        app.add_plugins((crate::overlay_menu::plugin, super::plugin));
        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::CareItemPicker);
        app.update();
        app
    }

    fn care_root(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, (With<CareItemPickerOverlay>, With<OverlayMenuRoot>)>()
            .single(app.world())
            .expect("one care overlay root")
    }

    fn care_entries(app: &mut App, root: Entity) -> Vec<(usize, String, bool)> {
        let mut entry_query = app
            .world_mut()
            .query::<(&OverlayMenuEntry, &OverlayMenuEntryState, &Children)>();
        let mut text_query = app.world_mut().query::<&Text>();
        let mut entries = entry_query
            .iter(app.world())
            .filter(|(entry, _, _)| entry.owner == root)
            .map(|(entry, state, children)| {
                let label = children
                    .iter()
                    .filter_map(|child| text_query.get(app.world(), child).ok())
                    .find(|text| text.as_str() != "▶")
                    .map(|text| text.as_str().to_string())
                    .expect("entry label");
                (entry.index, label, state.selected)
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.0);
        entries
    }

    #[test]
    fn entering_and_leaving_picker_owns_exactly_one_reusable_overlay() {
        let mut app = picker_app(CareMenuState {
            menu_id: Some(CareMenuId::Fridge),
            options: vec![ItemId::RawVeggieTub],
            cursor: 0,
        });

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
        let mut app = picker_app(CareMenuState {
            menu_id: Some(CareMenuId::Fridge),
            options: vec![ItemId::RawVeggieTub, ItemId::TortoiseLeafyGreens],
            cursor: 0,
        });
        let root = care_root(&mut app);

        let entries = care_entries(&mut app, root);
        assert_eq!(
            entries,
            [
                (0, "Lettuce & Veggie Tub".to_string(), true),
                (1, "Tortoise Leafy Greens".to_string(), false),
            ]
        );

        app.world_mut().resource_mut::<CareMenuState>().cursor = 1;
        app.update();
        assert_eq!(
            care_entries(&mut app, root)
                .into_iter()
                .map(|entry| entry.2)
                .collect::<Vec<_>>(),
            [false, true]
        );
    }

    #[test]
    fn invalid_care_states_supply_domain_specific_empty_copy() {
        for (care_menu, expected) in [
            (
                CareMenuState {
                    menu_id: Some(CareMenuId::Fridge),
                    options: Vec::new(),
                    cursor: 0,
                },
                "No items are available. Press Esc to close.",
            ),
            (
                CareMenuState {
                    menu_id: None,
                    options: vec![ItemId::RawVeggieTub],
                    cursor: 0,
                },
                "This item menu is unavailable. Press Esc to close.",
            ),
        ] {
            let mut app = picker_app(care_menu);
            care_root(&mut app);
            let copy = app
                .world_mut()
                .query_filtered::<&Text, With<OverlayMenuEmptyState>>()
                .iter(app.world())
                .next()
                .map(|text| text.as_str().to_string())
                .expect("care empty-state copy");
            assert_eq!(copy, expected);
        }
    }
}
