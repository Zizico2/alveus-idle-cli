//! Reusable full-screen overlay presentation built from Bevy's list widgets.

use alveus_theme::widget;
use bevy::{
    input_focus::AutoFocus,
    prelude::*,
    ui::Selected,
    ui_widgets::{ActiveDescendant, ListBox, ListItem},
};

pub(crate) struct OverlayMenuPlugin;

impl Plugin for OverlayMenuPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (OverlayMenuSystems::SyncSelection, OverlayMenuSystems::Style).chain(),
        )
        .add_systems(
            Update,
            style_overlay_entries.in_set(OverlayMenuSystems::Style),
        );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OverlayMenuSystems {
    SyncSelection,
    Style,
}

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

#[derive(Component)]
pub(crate) struct OverlayMenuRoot;

#[derive(Component)]
pub(crate) struct OverlayMenuList {}

#[derive(Component)]
pub(crate) struct OverlayMenuEntry {
    pub(crate) list: Entity,
    pub(crate) index: usize,
}

#[derive(Component)]
struct OverlayMenuEntryLabel;

#[derive(Component)]
pub(crate) struct OverlayMenuEmptyState;

pub(crate) struct SpawnedOverlayMenu {
    pub(crate) root: Entity,
    pub(crate) list: Option<Entity>,
}

pub(crate) fn spawn_overlay_menu(
    commands: &mut Commands,
    name: impl Into<String>,
    spec: OverlayMenuSpec,
) -> SpawnedOverlayMenu {
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
            AutoFocus,
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

    let mut list_entity = None;
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
                    let mut list_commands = card.spawn((
                        Name::new("Overlay Menu ListBox"),
                        ListBox,
                        OverlayMenuList {},
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            row_gap: px(8),
                            ..default()
                        },
                    ));
                    let list = list_commands.id();
                    list_entity = Some(list);
                    list_commands.with_children(|rows| {
                        for (index, label) in entries.into_iter().enumerate() {
                            let mut row = rows.spawn((
                                Name::new(format!("Overlay Menu Entry {index}")),
                                ListItem,
                                OverlayMenuEntry { list, index },
                                Node {
                                    width: percent(100),
                                    min_height: px(54),
                                    align_items: AlignItems::Center,
                                    padding: UiRect::horizontal(px(18)),
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(10)),
                                    ..default()
                                },
                                BackgroundColor(entry_background(selected == Some(index))),
                                BorderColor::all(entry_border(selected == Some(index))),
                                children![(
                                    Name::new("Overlay Menu Entry Label"),
                                    OverlayMenuEntryLabel,
                                    Text::new(label),
                                    TextFont::from_font_size(22.0),
                                    TextColor(entry_text(selected == Some(index))),
                                    Pickable::IGNORE,
                                )],
                            ));
                            if selected == Some(index) {
                                row.insert(Selected);
                            }
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

    SpawnedOverlayMenu {
        root,
        list: list_entity,
    }
}

pub(crate) fn project_selection(
    commands: &mut Commands,
    list: Entity,
    selected: Option<usize>,
    entries: &Query<(Entity, &OverlayMenuEntry, Has<Selected>)>,
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

fn style_overlay_entries(
    mut entries: Query<
        (
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<OverlayMenuEntry>,
    >,
    mut labels: Query<&mut TextColor, With<OverlayMenuEntryLabel>>,
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
