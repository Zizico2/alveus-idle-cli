//! Reusable list-menu presentation backed by [`ListMenuState`].

use alveus_menus_models::ListMenuState;
use alveus_theme::widget;
use bevy::{
    input_focus::InputFocus,
    prelude::*,
    ui::Selected,
    ui_widgets::{ActiveDescendant, ListBox, ListItem},
};

pub(crate) struct ListMenuPlugin;

impl Plugin for ListMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, style_selection_entries);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListMenuKind {
    /// Each row is a focusable Bevy button and activates immediately.
    Actions,
    /// The list owns one active descendant; individual rows are not focusable.
    Selection,
}

pub(crate) struct ListMenuSpec {
    kind: ListMenuKind,
    empty_copy: String,
}

impl ListMenuSpec {
    pub(crate) fn actions() -> Self {
        Self {
            kind: ListMenuKind::Actions,
            empty_copy: "No actions are available.".to_string(),
        }
    }

    pub(crate) fn selection() -> Self {
        Self {
            kind: ListMenuKind::Selection,
            empty_copy: "No options are available.".to_string(),
        }
    }

    pub(crate) fn with_empty_copy(mut self, empty_copy: impl Into<String>) -> Self {
        self.empty_copy = empty_copy.into();
        self
    }
}

#[derive(Component)]
pub(crate) struct ListMenu;

#[derive(Component)]
pub(crate) struct ListMenuEntry {
    pub(crate) list: Entity,
    pub(crate) index: usize,
}

#[derive(Component)]
struct SelectionListMenuEntry;

#[derive(Component)]
struct ListMenuEntryLabel;

#[derive(Component)]
pub(crate) struct ListMenuEmptyState;

pub(crate) struct SpawnedListMenu {
    pub(crate) list: Option<Entity>,
}

pub(crate) fn spawn_list_menu<T>(
    parent: &mut ChildSpawnerCommands,
    state: &ListMenuState<T>,
    spec: ListMenuSpec,
    label: impl Fn(&T) -> String,
) -> SpawnedListMenu {
    if state.options.is_empty() {
        parent.spawn((
            Name::new("List Menu Empty State"),
            ListMenuEmptyState,
            Text::new(spec.empty_copy),
            TextFont::from_font_size(21.0),
            TextColor(Color::srgb(0.92, 0.76, 0.48)),
        ));
        return SpawnedListMenu { list: None };
    }

    let mut list_commands = parent.spawn((
        Name::new("List Menu"),
        ListMenu,
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: match spec.kind {
                ListMenuKind::Actions => px(20),
                ListMenuKind::Selection => px(8),
            },
            align_items: AlignItems::Center,
            ..default()
        },
    ));
    if spec.kind == ListMenuKind::Selection {
        list_commands.insert(ListBox);
    }
    let list = list_commands.id();
    list_commands.with_children(|rows| {
        for (index, option) in state.options.iter().enumerate() {
            let text = label(option);
            match spec.kind {
                ListMenuKind::Actions => widget::spawn_button(
                    rows,
                    text,
                    index == state.cursor,
                    ListMenuEntry { list, index },
                ),
                ListMenuKind::Selection => {
                    let selected = state.selected_index() == Some(index);
                    let mut row = rows.spawn((
                        Name::new(format!("List Menu Entry {index}")),
                        ListItem,
                        ListMenuEntry { list, index },
                        SelectionListMenuEntry,
                        Node {
                            width: percent(100),
                            min_height: px(54),
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(px(18)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(10)),
                            ..default()
                        },
                        BackgroundColor(entry_background(selected)),
                        BorderColor::all(entry_border(selected)),
                        children![(
                            Name::new("List Menu Entry Label"),
                            ListMenuEntryLabel,
                            Text::new(text),
                            TextFont::from_font_size(22.0),
                            TextColor(entry_text(selected)),
                            Pickable::IGNORE,
                        )],
                    ));
                    if selected {
                        row.insert(Selected);
                    }
                    row.id()
                }
            };
        }
    });

    SpawnedListMenu { list: Some(list) }
}

pub(crate) fn project_selection(
    commands: &mut Commands,
    list: Entity,
    selected: Option<usize>,
    entries: &Query<(Entity, &ListMenuEntry, Has<Selected>), With<ListItem>>,
) {
    let mut active = None;
    for (entity, entry, is_selected) in entries.iter() {
        if entry.list != list {
            continue;
        }
        let should_select = selected == Some(entry.index);
        if should_select {
            active = Some(entity);
        }
        if should_select && !is_selected {
            commands.entity(entity).insert(Selected);
        } else if !should_select && is_selected {
            commands.entity(entity).remove::<Selected>();
        }
    }
    commands.entity(list).insert(ActiveDescendant(active));
}

/// Keep an action menu's generic model cursor aligned with Bevy's focused row.
pub(crate) fn sync_action_cursor<T: Send + Sync + 'static>(
    focus: Option<Res<InputFocus>>,
    entries: Query<&ListMenuEntry>,
    mut lists: Query<&mut ListMenuState<T>>,
) {
    let Some(focus) = focus else {
        return;
    };
    if !focus.is_changed() {
        return;
    }
    let Some(focused) = focus.get() else {
        return;
    };
    let Ok(entry) = entries.get(focused) else {
        return;
    };
    if let Ok(mut state) = lists.get_mut(entry.list) {
        state.set_cursor(entry.index);
    }
}

fn style_selection_entries(
    mut entries: Query<
        (
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<SelectionListMenuEntry>,
    >,
    mut labels: Query<&mut TextColor, With<ListMenuEntryLabel>>,
) {
    for (selected, mut background, mut border, children) in &mut entries {
        *background = entry_background(selected).into();
        *border = BorderColor::all(entry_border(selected));
        for child in children.iter() {
            if let Ok(mut color) = labels.get_mut(child) {
                *color = TextColor(entry_text(selected));
            }
        }
    }
}

fn entry_background(selected: bool) -> Color {
    if selected {
        Color::srgba(0.12, 0.33, 0.27, 0.98)
    } else {
        Color::srgba(0.065, 0.09, 0.11, 0.96)
    }
}

fn entry_border(selected: bool) -> Color {
    if selected {
        Color::srgb(0.45, 0.95, 0.72)
    } else {
        Color::srgba(0.34, 0.4, 0.43, 0.8)
    }
}

fn entry_text(selected: bool) -> Color {
    if selected {
        Color::WHITE
    } else {
        Color::srgb(0.82, 0.85, 0.87)
    }
}
