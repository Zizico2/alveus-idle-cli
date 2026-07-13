//! Full-screen care item picker overlay.

use alveus_app::Menu;
use alveus_components::{CareMenuState, PlayerSatchel};
use alveus_configs::{SATCHEL_MAX_SLOTS, item_display_name};
use alveus_theme::widget;
use alveus_types::CareMenuId;
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::CareItemPicker), spawn_care_item_picker)
        .add_systems(
            Update,
            update_selected_row.run_if(in_state(Menu::CareItemPicker)),
        );
}

/// Stable marker for lifecycle tests and UI inspection.
#[derive(Component)]
pub struct CareItemPickerOverlay;

#[derive(Component)]
struct CareItemPickerRow {
    index: usize,
}

#[derive(Component)]
struct CareItemPickerRowLabel;

fn selected_row_color() -> Color {
    Color::srgba(0.18, 0.55, 0.43, 0.95)
}

fn unselected_row_color() -> Color {
    Color::srgba(0.12, 0.15, 0.18, 0.92)
}

fn selected_row_border() -> Color {
    Color::srgb(0.55, 1.0, 0.78)
}

fn unselected_row_border() -> Color {
    Color::srgba(1.0, 1.0, 1.0, 0.08)
}

fn menu_title(menu_id: Option<CareMenuId>) -> &'static str {
    match menu_id {
        Some(CareMenuId::Fridge) => "Fridge supplies",
        None => "Choose an item",
    }
}

fn spawn_care_item_picker(
    mut commands: Commands,
    care_menu: Res<CareMenuState>,
    satchel: Res<PlayerSatchel>,
) {
    commands
        .spawn((
            Name::new("Care Item Picker Overlay"),
            CareItemPickerOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.02, 0.03, 0.72)),
            GlobalZIndex(10),
            DespawnOnExit(Menu::CareItemPicker),
            Pickable::default(),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Name::new("Care Item Picker Card"),
                    Node {
                        width: percent(52),
                        min_width: px(420),
                        max_width: px(620),
                        padding: UiRect::all(px(28)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(16),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(16)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.045, 0.065, 0.08, 0.98)),
                    BorderColor::all(Color::srgba(0.35, 0.9, 0.68, 0.75)),
                ))
                .with_children(|card| {
                    card.spawn(widget::header(menu_title(care_menu.menu_id)));
                    card.spawn(widget::label(format!(
                        "Satchel: {}/{} slots occupied",
                        satchel.occupied_count(),
                        SATCHEL_MAX_SLOTS
                    )));

                    if care_menu.menu_id.is_none() {
                        spawn_empty_copy(
                            card,
                            "This item menu is unavailable. Press Esc to close.",
                        );
                    } else if care_menu.options.is_empty() {
                        spawn_empty_copy(card, "No items are available. Press Esc to close.");
                    } else {
                        card.spawn((
                            Name::new("Care Item Picker Rows"),
                            Node {
                                width: percent(100),
                                flex_direction: FlexDirection::Column,
                                row_gap: px(8),
                                ..default()
                            },
                        ))
                        .with_children(|rows| {
                            for (index, item) in care_menu.options.iter().copied().enumerate() {
                                spawn_option_row(
                                    rows,
                                    index,
                                    item_display_name(item),
                                    index == care_menu.cursor,
                                );
                            }
                        });
                    }

                    card.spawn((
                        Name::new("Care Item Picker Controls"),
                        Text::new("↑/↓ Select   Space/Enter Take   Esc Back"),
                        TextFont::from_font_size(16.0),
                        TextColor(Color::srgb(0.72, 0.74, 0.76)),
                    ));
                });
        });
}

fn spawn_empty_copy(parent: &mut ChildSpawnerCommands, copy: &'static str) {
    parent.spawn((
        Name::new("Care Item Picker Empty State"),
        Text::new(copy),
        TextFont::from_font_size(21.0),
        TextColor(Color::srgb(0.92, 0.76, 0.48)),
    ));
}

fn spawn_option_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    label: &'static str,
    selected: bool,
) {
    parent
        .spawn((
            Name::new(format!("Care Item Picker Row {index}")),
            CareItemPickerRow { index },
            Node {
                width: percent(100),
                min_height: px(54),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(18)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(if selected {
                selected_row_color()
            } else {
                unselected_row_color()
            }),
            BorderColor::all(if selected {
                selected_row_border()
            } else {
                unselected_row_border()
            }),
        ))
        .with_child((
            CareItemPickerRowLabel,
            Text::new(format!("{}{}", if selected { "▶ " } else { "  " }, label)),
            TextFont::from_font_size(22.0),
            TextColor(if selected {
                Color::WHITE
            } else {
                Color::srgb(0.8, 0.82, 0.84)
            }),
        ));
}

fn update_selected_row(
    care_menu: Res<CareMenuState>,
    mut rows: Query<(
        &CareItemPickerRow,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    mut labels: Query<(&mut Text, &mut TextColor), With<CareItemPickerRowLabel>>,
) {
    if !care_menu.is_changed() {
        return;
    }

    for (row, mut background, mut border, children) in &mut rows {
        let selected = row.index == care_menu.cursor;
        background.0 = if selected {
            selected_row_color()
        } else {
            unselected_row_color()
        };
        *border = BorderColor::all(if selected {
            selected_row_border()
        } else {
            unselected_row_border()
        });

        let Some(item) = care_menu.options.get(row.index).copied() else {
            continue;
        };
        for child in children.iter() {
            if let Ok((mut text, mut color)) = labels.get_mut(child) {
                text.0 = format!(
                    "{}{}",
                    if selected { "▶ " } else { "  " },
                    item_display_name(item)
                );
                color.0 = if selected {
                    Color::WHITE
                } else {
                    Color::srgb(0.8, 0.82, 0.84)
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alveus_types::ItemId;
    use bevy::state::app::StatesPlugin;

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

    #[test]
    fn entering_and_leaving_picker_owns_exactly_one_overlay_root() {
        let mut app = picker_app(CareMenuState {
            menu_id: Some(CareMenuId::Fridge),
            options: vec![ItemId::RawVeggieTub],
            cursor: 0,
        });

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<CareItemPickerOverlay>>()
                .iter(app.world())
                .count(),
            1
        );

        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::None);
        app.update();

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<CareItemPickerOverlay>>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn rows_preserve_option_order_and_follow_cursor_changes() {
        let mut app = picker_app(CareMenuState {
            menu_id: Some(CareMenuId::Fridge),
            options: vec![ItemId::RawVeggieTub, ItemId::TortoiseLeafyGreens],
            cursor: 0,
        });

        let mut rows = app
            .world_mut()
            .query::<(&CareItemPickerRow, &BackgroundColor, &Children)>()
            .iter(app.world())
            .map(|(row, background, children)| {
                let label = children
                    .iter()
                    .find_map(|child| app.world().get::<Text>(child))
                    .expect("row label")
                    .0
                    .clone();
                (row.index, background.0, label)
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.0);

        assert_eq!(rows.len(), 2);
        assert!(rows[0].2.contains("Lettuce & Veggie Tub"));
        assert!(rows[1].2.contains("Tortoise Leafy Greens"));
        assert_eq!(rows[0].1, selected_row_color());
        assert_eq!(rows[1].1, unselected_row_color());

        app.world_mut().resource_mut::<CareMenuState>().cursor = 1;
        app.update();

        let colors = app
            .world_mut()
            .query::<(&CareItemPickerRow, &BackgroundColor)>()
            .iter(app.world())
            .map(|(row, background)| (row.index, background.0))
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(colors[&0], unselected_row_color());
        assert_eq!(colors[&1], selected_row_color());
    }

    #[test]
    fn invalid_picker_states_show_explicit_recovery_copy() {
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
            let copy = app
                .world_mut()
                .query::<(&Name, &Text)>()
                .iter(app.world())
                .find(|(name, _)| name.as_str() == "Care Item Picker Empty State")
                .map(|(_, text)| text.as_str().to_string())
                .expect("explicit picker recovery copy");
            assert_eq!(copy, expected);
        }
    }
}
