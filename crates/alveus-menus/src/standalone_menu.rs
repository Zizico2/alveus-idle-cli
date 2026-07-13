//! A screen-level shell for menus that replace the running scene UI.

use alveus_menus_models::ListMenuState;
use alveus_theme::widget;
use bevy::prelude::*;

use crate::list_menu::{ListMenuSpec, SpawnedListMenu, spawn_list_menu};

pub(crate) struct StandaloneMenuSpec {
    title: Option<String>,
}

impl StandaloneMenuSpec {
    pub(crate) fn new() -> Self {
        Self { title: None }
    }

    #[allow(dead_code)]
    pub(crate) fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

#[derive(Component)]
pub(crate) struct StandaloneMenuRoot;

pub(crate) struct SpawnedStandaloneMenu {
    pub(crate) root: Entity,
    pub(crate) list: SpawnedListMenu,
}

pub(crate) fn spawn_standalone_menu<T>(
    commands: &mut Commands,
    name: impl Into<String>,
    state: &ListMenuState<T>,
    standalone_spec: StandaloneMenuSpec,
    list_spec: ListMenuSpec,
    label: impl Fn(&T) -> String,
) -> SpawnedStandaloneMenu {
    let name = name.into();
    let root = commands
        .spawn((widget::ui_root(name), StandaloneMenuRoot, GlobalZIndex(2)))
        .id();
    let mut spawned_list = None;
    commands.entity(root).with_children(|menu| {
        if let Some(title) = standalone_spec.title {
            menu.spawn(widget::header(title));
        }
        spawned_list = Some(spawn_list_menu(menu, state, list_spec, label));
    });
    SpawnedStandaloneMenu {
        root,
        list: spawned_list.expect("standalone menu always spawns its list menu"),
    }
}
