//! A dimmed card shell for menus presented over a running scene.

use alveus_menus_models::ListMenuState;
use alveus_theme::widget;
use bevy::prelude::*;

use crate::list_menu::{ListMenuSpec, SpawnedListMenu, spawn_list_menu};

pub(crate) struct OverlayMenuSpec {
    title: String,
    summary: Option<String>,
    controls: Option<String>,
}

impl OverlayMenuSpec {
    pub(crate) fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            summary: None,
            controls: None,
        }
    }

    pub(crate) fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub(crate) fn with_controls(mut self, controls: impl Into<String>) -> Self {
        self.controls = Some(controls.into());
        self
    }
}

#[derive(Component)]
pub(crate) struct OverlayMenuRoot;

pub(crate) struct SpawnedOverlayMenu {
    pub(crate) root: Entity,
    pub(crate) list: SpawnedListMenu,
}

pub(crate) fn spawn_overlay_menu<T>(
    commands: &mut Commands,
    name: impl Into<String>,
    state: &ListMenuState<T>,
    overlay_spec: OverlayMenuSpec,
    list_spec: ListMenuSpec,
    label: impl Fn(&T) -> String,
) -> SpawnedOverlayMenu {
    let OverlayMenuSpec {
        title,
        summary,
        controls,
    } = overlay_spec;
    let root = commands
        .spawn((
            Name::new(name.into()),
            OverlayMenuRoot,
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

    let mut spawned_list = None;
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
                spawned_list = Some(spawn_list_menu(card, state, list_spec, label));
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
        list: spawned_list.expect("overlay card always spawns its list menu"),
    }
}
