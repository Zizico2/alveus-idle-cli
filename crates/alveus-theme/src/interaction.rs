use bevy::{
    input_focus::{FocusCause, InputFocus, InputFocusVisible},
    picking::events::Press,
    prelude::*,
    ui_widgets::Activate,
};

use alveus_asset_tracking::LoadResource;
use alveus_audio::sound_effect;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(apply_interaction_palette_on_click);
    app.add_observer(apply_interaction_palette_on_over);
    app.add_observer(apply_interaction_palette_on_out);

    app.load_resource::<InteractionAssets>();
    app.add_observer(play_sound_effect_on_click);
    app.add_observer(play_sound_effect_on_over);
    app.add_observer(focus_button_on_press);
    app.add_systems(Update, apply_button_focus_indicator);
}

fn focus_button_on_press(
    press: On<Pointer<Press>>,
    buttons: Query<(), With<crate::widget::ThemedButton>>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
) {
    if buttons.contains(press.entity) {
        focus.set(press.entity, FocusCause::Pressed);
        focus_visible.0 = false;
    }
}

fn apply_button_focus_indicator(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    mut buttons: Query<(Entity, &mut BorderColor), With<crate::widget::ThemedButton>>,
) {
    if !focus.is_changed() && !focus_visible.is_changed() {
        return;
    }
    for (entity, mut border) in &mut buttons {
        let color = if focus_visible.0 && focus.get() == Some(entity) {
            Color::srgb(0.58, 0.98, 0.79)
        } else {
            Color::NONE
        };
        *border = BorderColor::all(color);
    }
}

/// Palette for widget interactions. Add this to an entity that supports
/// [`Interaction`]s, such as a button, to change its [`BackgroundColor`] based
/// on the current interaction state.
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct InteractionPalette {
    pub none: Color,
    pub hovered: Color,
    pub pressed: Color,
}

fn apply_interaction_palette_on_click(
    click: On<Pointer<Click>>,
    mut palette_query: Query<(&InteractionPalette, &mut BackgroundColor)>,
) {
    let Ok((palette, mut bg)) = palette_query.get_mut(click.event_target()) else {
        return;
    };

    *bg = palette.pressed.into();
}

fn apply_interaction_palette_on_over(
    over: On<Pointer<Over>>,
    mut palette_query: Query<(&InteractionPalette, &mut BackgroundColor)>,
) {
    let Ok((palette, mut bg)) = palette_query.get_mut(over.event_target()) else {
        return;
    };

    *bg = palette.hovered.into();
}

fn apply_interaction_palette_on_out(
    out: On<Pointer<Out>>,
    mut palette_query: Query<(&InteractionPalette, &mut BackgroundColor)>,
) {
    let Ok((palette, mut bg)) = palette_query.get_mut(out.event_target()) else {
        return;
    };

    *bg = palette.none.into();
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct InteractionAssets {
    #[dependency]
    hover: Handle<AudioSource>,
    #[dependency]
    click: Handle<AudioSource>,
}

impl FromWorld for InteractionAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            hover: assets.load("audio/sound_effects/button_hover.ogg"),
            click: assets.load("audio/sound_effects/button_click.ogg"),
        }
    }
}

fn play_sound_effect_on_click(
    _: On<Activate>,
    interaction_assets: If<Res<InteractionAssets>>,
    mut commands: Commands,
) {
    commands.spawn(sound_effect(interaction_assets.click.clone()));
}

fn play_sound_effect_on_over(
    _: On<Pointer<Over>>,
    interaction_assets: If<Res<InteractionAssets>>,
    mut commands: Commands,
) {
    commands.spawn(sound_effect(interaction_assets.hover.clone()));
}
