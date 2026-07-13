//! Reusable full-screen overlay menu presentation.
//!
//! Overlay content is intentionally static for one menu lifecycle: callers
//! provide any number of labelled entries when spawning, then update only the
//! selected index. Menus whose structure changes can respawn on their next
//! state transition, which keeps domain state out of this presentation layer.

use std::collections::HashMap;

use alveus_theme::widget;
use bevy::prelude::*;

pub(crate) struct OverlayMenuPlugin;

impl Plugin for OverlayMenuPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                OverlayMenuSystems::SyncSelection,
                OverlayMenuSystems::ApplySelection,
            )
                .chain(),
        )
        .add_systems(
            Update,
            apply_changed_selection.in_set(OverlayMenuSystems::ApplySelection),
        );
    }
}

/// Ordering contract for feature adapters and the shared renderer.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OverlayMenuSystems {
    /// Feature adapters copy their domain cursor into [`OverlayMenuSelection`].
    SyncSelection,
    /// The shared renderer applies the resulting row styling.
    ApplySelection,
}

/// Static content used to construct one overlay menu.
pub(crate) struct OverlayMenuSpec {
    title: String,
    summary: Option<String>,
    entries: Vec<String>,
    selected: Option<usize>,
    empty_copy: String,
    controls: Option<String>,
}

impl OverlayMenuSpec {
    pub(crate) fn new<I, S>(title: impl Into<String>, entries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            title: title.into(),
            summary: None,
            entries: entries.into_iter().map(Into::into).collect(),
            selected: None,
            empty_copy: "No options are available.".to_string(),
            controls: None,
        }
    }

    pub(crate) fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub(crate) fn with_selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn with_empty_copy(mut self, empty_copy: impl Into<String>) -> Self {
        self.empty_copy = empty_copy.into();
        self
    }

    pub(crate) fn with_controls(mut self, controls: impl Into<String>) -> Self {
        self.controls = Some(controls.into());
        self
    }
}

/// Marker for a complete styled overlay root.
#[derive(Component)]
pub(crate) struct OverlayMenuRoot;

/// Dynamic state supported without rebuilding the static menu content.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverlayMenuSelection(Option<usize>);

impl OverlayMenuSelection {
    pub(crate) fn selected(&self) -> Option<usize> {
        self.0
    }

    pub(crate) fn set(&mut self, selected: Option<usize>) {
        self.0 = selected;
    }
}

#[derive(Component)]
pub(crate) struct OverlayMenuEntry {
    pub(crate) owner: Entity,
    pub(crate) index: usize,
}

#[derive(Component)]
pub(crate) struct OverlayMenuEntryState {
    pub(crate) selected: bool,
}

#[derive(Component)]
struct OverlayMenuCursor {
    owner: Entity,
    index: usize,
}

#[derive(Component)]
pub(crate) struct OverlayMenuEmptyState;

/// Spawn one overlay with the shared styling and return its root entity.
///
/// Callers attach lifecycle and feature marker components to the returned root.
pub(crate) fn spawn_overlay_menu(
    commands: &mut Commands,
    name: impl Into<String>,
    spec: OverlayMenuSpec,
) -> Entity {
    let OverlayMenuSpec {
        title,
        summary,
        entries,
        selected,
        empty_copy,
        controls,
    } = spec;
    let selected = selected.filter(|index| *index < entries.len());

    let root = commands
        .spawn((
            Name::new(name.into()),
            OverlayMenuRoot,
            OverlayMenuSelection(selected),
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
            Pickable::default(),
        ))
        .id();

    commands.entity(root).with_children(|overlay| {
        overlay
            .spawn((
                Name::new("Overlay Menu Card"),
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
                card.spawn(widget::header(title));
                if let Some(summary) = summary {
                    card.spawn(widget::label(summary));
                }

                if entries.is_empty() {
                    card.spawn((
                        Name::new("Overlay Menu Empty State"),
                        OverlayMenuEmptyState,
                        Text::new(empty_copy),
                        TextFont::from_font_size(21.0),
                        TextColor(Color::srgb(0.92, 0.76, 0.48)),
                    ));
                } else {
                    card.spawn((
                        Name::new("Overlay Menu Rows"),
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(8),
                            ..default()
                        },
                    ))
                    .with_children(|rows| {
                        for (index, label) in entries.into_iter().enumerate() {
                            spawn_entry(rows, root, index, label, selected == Some(index));
                        }
                    });
                }

                if let Some(controls) = controls {
                    card.spawn((
                        Name::new("Overlay Menu Controls"),
                        Text::new(controls),
                        TextFont::from_font_size(16.0),
                        TextColor(Color::srgb(0.72, 0.74, 0.76)),
                    ));
                }
            });
    });

    root
}

fn spawn_entry(
    parent: &mut ChildSpawnerCommands,
    owner: Entity,
    index: usize,
    label: String,
    selected: bool,
) {
    parent
        .spawn((
            Name::new(format!("Overlay Menu Entry {index}")),
            OverlayMenuEntry { owner, index },
            OverlayMenuEntryState { selected },
            Node {
                width: percent(100),
                min_height: px(54),
                align_items: AlignItems::Center,
                column_gap: px(10),
                padding: UiRect::horizontal(px(18)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(entry_background(selected)),
            BorderColor::all(entry_border(selected)),
        ))
        .with_children(|entry| {
            entry.spawn((
                Name::new("Overlay Menu Cursor"),
                OverlayMenuCursor { owner, index },
                Node {
                    width: px(22),
                    ..default()
                },
                Text::new("▶"),
                TextFont::from_font_size(22.0),
                TextColor(Color::WHITE),
                if selected {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
            ));
            entry.spawn((
                Name::new("Overlay Menu Entry Label"),
                Text::new(label),
                TextFont::from_font_size(22.0),
                TextColor(entry_text(selected)),
            ));
        });
}

fn apply_changed_selection(
    selections: Query<(Entity, &OverlayMenuSelection), Changed<OverlayMenuSelection>>,
    mut entries: Query<(
        &OverlayMenuEntry,
        &mut OverlayMenuEntryState,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    mut cursors: Query<(&OverlayMenuCursor, &mut Visibility)>,
    mut labels: Query<&mut TextColor>,
) {
    let changed = selections
        .iter()
        .map(|(entity, selection)| (entity, selection.selected()))
        .collect::<HashMap<_, _>>();
    if changed.is_empty() {
        return;
    }

    for (entry, mut state, mut background, mut border, children) in &mut entries {
        let Some(selected_index) = changed.get(&entry.owner) else {
            continue;
        };
        let selected = *selected_index == Some(entry.index);
        state.selected = selected;
        background.0 = entry_background(selected);
        *border = BorderColor::all(entry_border(selected));

        for child in children.iter() {
            if let Ok((cursor, mut visibility)) = cursors.get_mut(child)
                && cursor.owner == entry.owner
                && cursor.index == entry.index
            {
                *visibility = if selected {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            } else if let Ok(mut color) = labels.get_mut(child) {
                color.0 = entry_text(selected);
            }
        }
    }
}

fn entry_background(selected: bool) -> Color {
    if selected {
        Color::srgba(0.18, 0.55, 0.43, 0.95)
    } else {
        Color::srgba(0.12, 0.15, 0.18, 0.92)
    }
}

fn entry_border(selected: bool) -> Color {
    if selected {
        Color::srgb(0.55, 1.0, 0.78)
    } else {
        Color::srgba(1.0, 1.0, 1.0, 0.08)
    }
}

fn entry_text(selected: bool) -> Color {
    if selected {
        Color::WHITE
    } else {
        Color::srgb(0.8, 0.82, 0.84)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlay_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(OverlayMenuPlugin);
        app
    }

    fn spawn(app: &mut App, spec: OverlayMenuSpec) -> Entity {
        let root = spawn_overlay_menu(&mut app.world_mut().commands(), "Test Overlay", spec);
        app.world_mut().flush();
        root
    }

    fn entry_snapshot(app: &mut App, owner: Entity) -> Vec<(usize, String, bool)> {
        let mut entry_query = app
            .world_mut()
            .query::<(&OverlayMenuEntry, &OverlayMenuEntryState, &Children)>();
        let mut text_query = app
            .world_mut()
            .query::<(&Text, Option<&OverlayMenuCursor>)>();
        let mut entries = entry_query
            .iter(app.world())
            .filter(|(entry, _, _)| entry.owner == owner)
            .map(|(entry, state, children)| {
                let label = children
                    .iter()
                    .find_map(|child| {
                        text_query
                            .get(app.world(), child)
                            .ok()
                            .filter(|(_, cursor)| cursor.is_none())
                            .map(|(text, _)| text.as_str().to_string())
                    })
                    .expect("entry label");
                (entry.index, label, state.selected)
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.0);
        entries
    }

    #[test]
    fn accepts_variable_entry_counts_and_preserves_order() {
        for count in [1, 2, 4] {
            let mut app = overlay_app();
            let labels = (0..count)
                .map(|index| format!("Entry {index}"))
                .collect::<Vec<_>>();
            let root = spawn(
                &mut app,
                OverlayMenuSpec::new("Variable", labels.clone()).with_selected(Some(0)),
            );

            let entries = entry_snapshot(&mut app, root);
            assert_eq!(entries.len(), count);
            assert_eq!(
                entries.iter().map(|entry| &entry.1).collect::<Vec<_>>(),
                labels.iter().collect::<Vec<_>>()
            );
            assert!(entries[0].2);
        }
    }

    #[test]
    fn empty_entries_render_caller_copy_without_rows() {
        let mut app = overlay_app();
        let root = spawn(
            &mut app,
            OverlayMenuSpec::new("Empty", Vec::<String>::new())
                .with_empty_copy("Nothing here yet."),
        );

        assert!(entry_snapshot(&mut app, root).is_empty());
        let copies = app
            .world_mut()
            .query_filtered::<&Text, With<OverlayMenuEmptyState>>()
            .iter(app.world())
            .map(|text| text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(copies, ["Nothing here yet."]);
    }

    #[test]
    fn selection_updates_are_scoped_and_out_of_range_selects_nothing() {
        let mut app = overlay_app();
        let first = spawn(
            &mut app,
            OverlayMenuSpec::new("First", ["A", "B"]).with_selected(Some(0)),
        );
        let second = spawn(
            &mut app,
            OverlayMenuSpec::new("Second", ["C", "D", "E"]).with_selected(Some(2)),
        );
        app.update();

        app.world_mut()
            .get_mut::<OverlayMenuSelection>(first)
            .unwrap()
            .set(Some(1));
        app.update();

        assert_eq!(
            entry_snapshot(&mut app, first)
                .into_iter()
                .map(|entry| entry.2)
                .collect::<Vec<_>>(),
            [false, true]
        );
        assert_eq!(
            entry_snapshot(&mut app, second)
                .into_iter()
                .map(|entry| entry.2)
                .collect::<Vec<_>>(),
            [false, false, true]
        );

        app.world_mut()
            .get_mut::<OverlayMenuSelection>(first)
            .unwrap()
            .set(Some(99));
        app.update();
        assert!(entry_snapshot(&mut app, first).iter().all(|entry| !entry.2));
    }
}
