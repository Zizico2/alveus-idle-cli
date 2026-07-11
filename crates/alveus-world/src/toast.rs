#![allow(dead_code)]

use alveus_components::{BuildingEntrance, CareFeedbackEvent};
use bevy::prelude::*;
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

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(toast_trigger_observer)
            .add_observer(toast_dismiss_observer)
            .add_observer(player_entered_building_observer)
            .add_observer(player_exited_building_observer)
            .add_observer(on_anim_completed)
            .add_observer(care_feedback_toast_observer);
    }
}

fn care_feedback_toast_observer(trigger: On<CareFeedbackEvent>, mut commands: Commands) {
    let message = trigger.event().message.clone();
    commands.trigger(TriggerToastEvent::new(message));
}

#[derive(Event, Debug, Clone, Reflect)]
#[reflect(Event)]
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

#[derive(Event, Debug, Clone, Copy, Reflect)]
#[reflect(Event)]
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
            slide_in
                .then(delay)
                .then(slide_out.with_cycle_completed_event(true))
                .into_boxed()
        } else {
            slide_in.into_boxed()
        };

        let mut entity_cmds = commands.entity(toast_entity);
        entity_cmds.remove::<TweenAnim>();

        if event.duration.is_some() {
            entity_cmds.insert((TweenAnim::new(tweenable), ToastDismissalMarker));
        } else {
            entity_cmds
                .remove::<ToastDismissalMarker>()
                .insert(TweenAnim::new(tweenable));
        }
    } else {
        // Spawn new toast UI
        let text_entity = commands
            .spawn((
                Text::new(event.message.clone()),
                TextFont::from_font_size(TOAST_FONT_SIZE),
                TextColor(Color::WHITE),
            ))
            .id();

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
            slide_in
                .then(delay)
                .then(slide_out.with_cycle_completed_event(true))
                .into_boxed()
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
            ActiveToast { text_entity },
            TweenAnim::new(tweenable),
        ));

        if event.duration.is_some() {
            entity_cmds.insert(ToastDismissalMarker);
        }

        entity_cmds.add_child(text_entity);
    }
}

pub fn despawn_active_toast(mut commands: Commands, query: Query<Entity, With<ActiveToast>>) {
    for entity in &query {
        commands.entity(entity).despawn();
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
        )
        .with_cycle_completed_event(true);

        commands
            .entity(toast_entity)
            .remove::<TweenAnim>()
            .insert((TweenAnim::new(slide_out), ToastDismissalMarker));
    }
}

fn on_anim_completed(
    trigger: On<AnimCompletedEvent>,
    mut commands: Commands,
    query: Query<(), With<ToastDismissalMarker>>,
) {
    let anim_entity = trigger.anim_entity;
    if query.contains(anim_entity) {
        info!("Despawning toast notification on completion (Observer)");
        commands.entity(anim_entity).despawn();
    }
}

fn player_entered_building_observer(
    trigger: On<crate::entrance::PlayerEnteredBuildingEvent>,
    mut commands: Commands,
) {
    let entrance = trigger.event().entrance;
    let name = match entrance {
        BuildingEntrance::NutritionHouse => "Nutrition House",
        BuildingEntrance::PushPopEnclosure => "Push Pop Enclosure",
        _ => "Unknown Area",
    };
    commands.trigger(TriggerToastEvent::presence(format!(
        "Press [Enter] to enter {}",
        name
    )));
}

fn player_exited_building_observer(
    _trigger: On<crate::entrance::PlayerExitedBuildingEvent>,
    mut commands: Commands,
) {
    commands.trigger(DismissToastEvent);
}
