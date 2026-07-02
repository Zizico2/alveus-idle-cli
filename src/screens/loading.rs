//! A loading screen during which game assets are loaded if necessary.
//! This reduces stuttering, especially for audio on Wasm.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledMapAsset;

use crate::{
    asset_tracking::ResourceHandles,
    collision::{CollisionMasks, build_all_collision_masks, collision_ready},
    demo::level::{InteriorAssets, LevelAssets},
    screens::Screen,
    theme::prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Loading), spawn_loading_screen);

    app.add_systems(
        Update,
        (
            build_collision_masks_during_loading,
            enter_gameplay_screen
                .after(build_collision_masks_during_loading)
                .run_if(in_state(Screen::Loading).and(loading_complete)),
        ),
    );
}

fn spawn_loading_screen(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Loading Screen"),
        DespawnOnExit(Screen::Loading),
        children![widget::label("Loading...")],
    ));
}

fn build_collision_masks_during_loading(
    mut masks: ResMut<CollisionMasks>,
    map_assets: Res<Assets<TiledMapAsset>>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
) {
    let (Some(level_assets), Some(interior_assets)) = (level_assets, interior_assets) else {
        return;
    };

    if collision_ready(&masks) {
        return;
    }

    build_all_collision_masks(&mut masks, &map_assets, &level_assets, &interior_assets);
}

fn loading_complete(resource_handles: Res<ResourceHandles>, masks: Res<CollisionMasks>) -> bool {
    resource_handles.is_all_done() && collision_ready(&masks)
}

fn enter_gameplay_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Gameplay);
}
