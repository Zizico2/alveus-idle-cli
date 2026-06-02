#![allow(dead_code)]

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use crate::components::{CurrentTilePosition, TileGroup, RectangleTileGroup, TilePosition, BuildingEntrance};
use crate::demo::player::Player;
use bevy_tweening::{lens::UiPositionLens, *};

// Layout coordinates
pub const TOAST_ONSCREEN_BOTTOM: f32 = 24.0;
pub const TOAST_OFFSCREEN_BOTTOM: f32 = -100.0;
pub const TOAST_LEFT_OFFSET: f32 = 24.0;

// Animation & Timing
pub const TOAST_SLIDE_DURATION_MS: u64 = 300;
pub const TOAST_DEFAULT_DELAY_MS: u64 = 2400;

// Visual Specs
pub const TOAST_WIDTH: f32 = 240.0;
pub const TOAST_HEIGHT: f32 = 60.0;
pub const TOAST_PADDING: f32 = 12.0;
pub const TOAST_BORDER_LEFT_WIDTH: f32 = 4.0;
pub const TOAST_FONT_SIZE: f32 = 16.0;
pub const TOAST_BORDER_COLOR: Color = Color::srgb(0.2, 0.6, 1.0);
pub const TOAST_BACKGROUND_COLOR: Color = Color::srgba(0.08, 0.08, 0.08, 0.9);

// Grid Snapping bounds
pub const TILE_SIZE: f32 = 32.0;
pub const GRID_SNAP_EPSILON: f32 = 0.05;

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TileGroup>()
            .add_observer(toast_trigger_observer)
            .add_observer(toast_dismiss_observer)
            .add_systems(
                Update,
                (
                    validate_and_snap_entrances,
                    check_player_tile_triggers,
                ),
            )
            .add_systems(PostUpdate, despawn_completed_toasts);
    }
}

#[derive(Event, Debug, Clone)]
pub struct TriggerToastEvent {
    pub message: String,
    pub duration: Option<std::time::Duration>,
}

impl TriggerToastEvent {
    /// Create a standard temporary toast event with the default delay.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            duration: Some(std::time::Duration::from_millis(TOAST_DEFAULT_DELAY_MS)),
        }
    }

    /// Create a presence-based toast event that remains visible until dismissed.
    pub fn presence(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            duration: None,
        }
    }
}

#[derive(Event, Debug, Clone)]
pub struct DismissToastEvent;

#[derive(Component)]
pub struct ActiveToast {
    pub text_entity: Entity,
}

#[derive(Component)]
pub struct ToastDismissalMarker;

fn toast_trigger_observer(
    trigger: On<TriggerToastEvent>,
    mut commands: Commands,
    active_toast_query: Query<(Entity, &ActiveToast, &Node)>,
    mut text_query: Query<&mut Text>,
) {
    let event = trigger.event();
    
    if let Some((toast_entity, active_toast, node)) = active_toast_query.iter().next() {
        // Update message text
        if let Ok(mut text) = text_query.get_mut(active_toast.text_entity) {
            text.0 = event.message.clone();
        }
        
        // Resolve current bottom position directly from the Val enum to avoid snapping
        let current_bottom = match node.bottom {
            Val::Px(val) => val,
            _ => TOAST_OFFSCREEN_BOTTOM,
        };
        
        let slide_in = Tween::new(
            EaseFunction::CubicOut,
            std::time::Duration::from_millis(TOAST_SLIDE_DURATION_MS),
            UiPositionLens {
                start: UiRect {
                    bottom: Val::Px(current_bottom),
                    left: Val::Px(TOAST_LEFT_OFFSET),
                    top: Val::Auto,
                    right: Val::Auto,
                },
                end: UiRect {
                    bottom: Val::Px(TOAST_ONSCREEN_BOTTOM),
                    left: Val::Px(TOAST_LEFT_OFFSET),
                    top: Val::Auto,
                    right: Val::Auto,
                },
            },
        );

        let tweenable = if let Some(dur) = event.duration {
            let delay = Delay::new(dur);
            let slide_out = Tween::new(
                EaseFunction::CubicIn,
                std::time::Duration::from_millis(TOAST_SLIDE_DURATION_MS),
                UiPositionLens {
                    start: UiRect {
                        bottom: Val::Px(TOAST_ONSCREEN_BOTTOM),
                        left: Val::Px(TOAST_LEFT_OFFSET),
                        top: Val::Auto,
                        right: Val::Auto,
                    },
                    end: UiRect {
                        bottom: Val::Px(TOAST_OFFSCREEN_BOTTOM),
                        left: Val::Px(TOAST_LEFT_OFFSET),
                        top: Val::Auto,
                        right: Val::Auto,
                    },
                },
            );
            slide_in.then(delay).then(slide_out.with_cycle_completed_event(true)).into_boxed()
        } else {
            slide_in.into_boxed()
        };
        
        let mut entity_cmds = commands.entity(toast_entity);
        entity_cmds.remove::<TweenAnim>();
        
        if event.duration.is_some() {
            entity_cmds.insert((TweenAnim::new(tweenable), ToastDismissalMarker));
        } else {
            entity_cmds.remove::<ToastDismissalMarker>().insert(TweenAnim::new(tweenable));
        }
    } else {
        // Spawn new toast UI
        let text_entity = commands.spawn((
            Text::new(event.message.clone()),
            TextFont {
                font_size: TOAST_FONT_SIZE,
                ..default()
            },
            TextColor(Color::WHITE),
        )).id();
        
        let slide_in = Tween::new(
            EaseFunction::CubicOut,
            std::time::Duration::from_millis(TOAST_SLIDE_DURATION_MS),
            UiPositionLens {
                start: UiRect {
                    bottom: Val::Px(TOAST_OFFSCREEN_BOTTOM),
                    left: Val::Px(TOAST_LEFT_OFFSET),
                    top: Val::Auto,
                    right: Val::Auto,
                },
                end: UiRect {
                    bottom: Val::Px(TOAST_ONSCREEN_BOTTOM),
                    left: Val::Px(TOAST_LEFT_OFFSET),
                    top: Val::Auto,
                    right: Val::Auto,
                },
            },
        );

        let tweenable = if let Some(dur) = event.duration {
            let delay = Delay::new(dur);
            let slide_out = Tween::new(
                EaseFunction::CubicIn,
                std::time::Duration::from_millis(TOAST_SLIDE_DURATION_MS),
                UiPositionLens {
                    start: UiRect {
                        bottom: Val::Px(TOAST_ONSCREEN_BOTTOM),
                        left: Val::Px(TOAST_LEFT_OFFSET),
                        top: Val::Auto,
                        right: Val::Auto,
                    },
                    end: UiRect {
                        bottom: Val::Px(TOAST_OFFSCREEN_BOTTOM),
                        left: Val::Px(TOAST_LEFT_OFFSET),
                        top: Val::Auto,
                        right: Val::Auto,
                    },
                },
            );
            slide_in.then(delay).then(slide_out.with_cycle_completed_event(true)).into_boxed()
        } else {
            slide_in.into_boxed()
        };
        
        let mut entity_cmds = commands.spawn((
            Name::new("Toast Notification"),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(TOAST_OFFSCREEN_BOTTOM), // Start offscreen
                left: Val::Px(TOAST_LEFT_OFFSET),
                width: Val::Px(TOAST_WIDTH),
                height: Val::Px(TOAST_HEIGHT),
                padding: UiRect::all(Val::Px(TOAST_PADDING)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                border: UiRect::left(Val::Px(TOAST_BORDER_LEFT_WIDTH)),
                top: Val::Auto,
                right: Val::Auto,
                ..default()
            },
            BorderColor::all(TOAST_BORDER_COLOR), // Accenting left border
            BackgroundColor(TOAST_BACKGROUND_COLOR), // Premium dark glass
            ActiveToast {
                text_entity,
            },
            TweenAnim::new(tweenable),
        ));
        
        if event.duration.is_some() {
            entity_cmds.insert(ToastDismissalMarker);
        }
        
        entity_cmds.add_child(text_entity);
    }
}

fn toast_dismiss_observer(
    _trigger: On<DismissToastEvent>,
    mut commands: Commands,
    active_toast_query: Query<(Entity, &Node), With<ActiveToast>>,
) {
    if let Some((toast_entity, node)) = active_toast_query.iter().next() {
        // Resolve current bottom position directly from the Val enum to avoid snapping
        let current_bottom = match node.bottom {
            Val::Px(val) => val,
            _ => TOAST_ONSCREEN_BOTTOM,
        };
        
        let slide_out = Tween::new(
            EaseFunction::CubicIn,
            std::time::Duration::from_millis(TOAST_SLIDE_DURATION_MS),
            UiPositionLens {
                start: UiRect {
                    bottom: Val::Px(current_bottom),
                    left: Val::Px(TOAST_LEFT_OFFSET),
                    top: Val::Auto,
                    right: Val::Auto,
                },
                end: UiRect {
                    bottom: Val::Px(TOAST_OFFSCREEN_BOTTOM),
                    left: Val::Px(TOAST_LEFT_OFFSET),
                    top: Val::Auto,
                    right: Val::Auto,
                },
            },
        ).with_cycle_completed_event(true);
        
        commands.entity(toast_entity)
            .remove::<TweenAnim>()
            .insert((TweenAnim::new(slide_out), ToastDismissalMarker));
    }
}

fn despawn_completed_toasts(
    mut events: MessageReader<AnimCompletedEvent>,
    mut commands: Commands,
    query: Query<(), With<ToastDismissalMarker>>,
) {
    for event in events.read() {
        if query.contains(event.anim_entity) {
            info!("Despawning toast notification on completion");
            commands.entity(event.anim_entity).despawn();
        }
    }
}

fn validate_and_snap_entrances(
    mut commands: Commands,
    query: Query<
        (Entity, &Transform, &BuildingEntrance, &TiledObject),
        (Added<BuildingEntrance>, Without<TileGroup>),
    >,
) {
    for (entity, transform, entrance, tiled_object) in query.iter() {
        let x = transform.translation.x;
        let y = transform.translation.y;

        let rem_x = x.rem_euclid(TILE_SIZE);
        let rem_y = y.rem_euclid(TILE_SIZE);

        let dist_x = rem_x.min(TILE_SIZE - rem_x);
        let dist_y = rem_y.min(TILE_SIZE - rem_y);

        if dist_x >= GRID_SNAP_EPSILON || dist_y >= GRID_SNAP_EPSILON {
            panic!(
                "\n❌ MAP INTEGRITY ERROR ❌\nObject: '{:?}'\nPosition: [x:{:.2}, y:{:.2}]\nIssue: Not aligned to {}-pixel grid.\n",
                entrance, x, y, TILE_SIZE
            );
        }

        let TiledObject::Rectangle { width, height } = tiled_object else {
            panic!(
                "\n❌ MAP INTEGRITY ERROR ❌\nObject: '{:?}'\nIssue: Unsupported TiledObject type for size validation.\n",
                entrance
            );
        };

        if width % TILE_SIZE != 0.0 || height % TILE_SIZE != 0.0 {
            panic!(
                "\n❌ MAP INTEGRITY ERROR ❌\nObject: '{:?}'\nSize: [w:{}, h:{}]\nIssue: Dimensions are not multiples of tile size ({}).\n",
                entrance, width, height, TILE_SIZE
            );
        }

        let adjusted_y = y - height;

        let start_grid_x = (x / TILE_SIZE).round() as u32;
        let start_grid_y = (adjusted_y / TILE_SIZE).round() as u32;

        let width_in_tiles = (width / TILE_SIZE).round() as u32;
        let height_in_tiles = (height / TILE_SIZE).round() as u32;

        let tile_group = TileGroup::Rectangle(RectangleTileGroup {
            bottom_left: TilePosition {
                x: start_grid_x,
                y: start_grid_y,
            },
            top_right: TilePosition {
                x: start_grid_x + width_in_tiles - 1,
                y: start_grid_y + height_in_tiles - 1,
            },
        });
        info!("Inserting snapped TileGroup: {:?}", tile_group);

        commands.entity(entity).insert(tile_group);
    }
}

fn check_player_tile_triggers(
    mut commands: Commands,
    player_query: Query<(Entity, &CurrentTilePosition, Option<&BuildingEntrance>), With<Player>>,
    entrance_query: Query<(&TileGroup, &BuildingEntrance)>,
) {
    let Some((player_entity, player_pos, current_entrance)) = player_query.iter().next() else {
        return;
    };

    let player_grid = player_pos.0;
    let mut overlapping_entrance = None;

    for (tile_group, entrance) in &entrance_query {
        match tile_group {
            TileGroup::Rectangle(rect) => {
                if player_grid.x >= rect.bottom_left.x
                    && player_grid.x <= rect.top_right.x
                    && player_grid.y >= rect.bottom_left.y
                    && player_grid.y <= rect.top_right.y
                {
                    overlapping_entrance = Some(*entrance);
                    break;
                }
            }
        }
    }

    if let Some(entrance) = overlapping_entrance {
        if current_entrance != Some(&entrance) {
            let name = match entrance {
                BuildingEntrance::NutritionHouse => "Nutrition House",
                _ => "Unknown Area",
            };
            commands.trigger(TriggerToastEvent::presence(format!("Entered {}", name)));
            commands.entity(player_entity).insert(entrance);
        }
    } else {
        if current_entrance.is_some() {
            commands.trigger(DismissToastEvent);
            commands.entity(player_entity).remove::<BuildingEntrance>();
        }
    }
}
