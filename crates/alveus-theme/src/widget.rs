//! Helper functions for creating common widgets.

use std::borrow::Cow;

use bevy::{
    ecs::{spawn::SpawnWith, system::IntoObserverSystem},
    input_focus::{AutoFocus, tab_navigation::TabIndex},
    prelude::*,
    ui::auto_directional_navigation::AutoDirectionalNavigation,
    ui_widgets::Button as UiButton,
};

use crate::{interaction::InteractionPalette, palette::*};

/// A root UI node that fills the window and centers its content.
pub fn ui_root(name: impl Into<Cow<'static, str>>) -> impl Bundle {
    (
        Name::new(name),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            row_gap: px(20),
            ..default()
        },
        // Don't block picking events for other UI roots.
        Pickable::IGNORE,
    )
}

/// A simple header label. Bigger than [`label`].
pub fn header(text: impl Into<String>) -> impl Bundle {
    (
        Name::new("Header"),
        Text(text.into()),
        TextFont::from_font_size(40.0),
        TextColor(HEADER_TEXT),
    )
}

/// A simple text label.
pub fn label(text: impl Into<String>) -> impl Bundle {
    (
        Name::new("Label"),
        Text(text.into()),
        TextFont::from_font_size(24.0),
        TextColor(LABEL_TEXT),
    )
}

/// A large rounded button with text and an action defined as an [`Observer`].
pub fn button<E, B, M, I>(text: impl Into<String>, action: I) -> impl Bundle
where
    E: EntityEvent,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    button_base(
        text,
        action,
        false,
        Node {
            width: px(380),
            height: px(80),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(3)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
    )
}

/// A large button that receives focus when its menu is spawned.
pub fn button_autofocus<E, B, M, I>(text: impl Into<String>, action: I) -> impl Bundle
where
    E: EntityEvent,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    button_base(
        text,
        action,
        true,
        Node {
            width: px(380),
            height: px(80),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(3)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
    )
}

/// A small square button with text and an action defined as an [`Observer`].
pub fn button_small<E, B, M, I>(text: impl Into<String>, action: I) -> impl Bundle
where
    E: EntityEvent,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    button_base(
        text,
        action,
        false,
        Node {
            width: px(30),
            height: px(30),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(2)),
            ..default()
        },
    )
}

/// A simple button with text and an action defined as an [`Observer`]. The button's layout is provided by `button_bundle`.
fn button_base<E, B, M, I>(
    text: impl Into<String>,
    action: I,
    auto_focus: bool,
    button_bundle: impl Bundle,
) -> impl Bundle
where
    E: EntityEvent,
    B: Bundle,
    I: IntoObserverSystem<E, B, M>,
{
    let text = text.into();
    let action = IntoObserverSystem::into_system(action);
    (
        Name::new("Button"),
        Node::default(),
        Children::spawn(SpawnWith(move |parent: &mut ChildSpawner| {
            let mut button = parent.spawn((
                Name::new("Button Inner"),
                UiButton,
                ThemedButton,
                TabIndex(0),
                AutoDirectionalNavigation::default(),
                BackgroundColor(BUTTON_BACKGROUND),
                BorderColor::all(Color::NONE),
                InteractionPalette {
                    none: BUTTON_BACKGROUND,
                    hovered: BUTTON_HOVERED_BACKGROUND,
                    pressed: BUTTON_PRESSED_BACKGROUND,
                },
                children![(
                    Name::new("Button Text"),
                    Text(text),
                    TextFont::from_font_size(40.0),
                    TextColor(BUTTON_TEXT),
                    // Don't bubble picking events from the text up to the button.
                    Pickable::IGNORE,
                )],
            ));
            button.insert(button_bundle).observe(action);
            if auto_focus {
                button.insert(AutoFocus);
            }
        })),
    )
}

/// Marker for the standard Bevy button entities styled by this crate.
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct ThemedButton;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        input_focus::{AutoFocus, tab_navigation::TabIndex},
        ui::auto_directional_navigation::AutoDirectionalNavigation,
        ui_widgets::{Activate, Button},
    };

    fn ignore_activation(_: On<Activate>) {}

    #[test]
    fn themed_buttons_use_standard_focusable_widgets() {
        let mut world = World::new();
        world.spawn(button_autofocus("Continue", ignore_activation));
        world.flush();

        let mut query = world.query_filtered::<(Entity, &TabIndex), (
            With<Button>,
            With<ThemedButton>,
            With<AutoFocus>,
            With<AutoDirectionalNavigation>,
        )>();
        let (_, tab_index) = query.single(&world).expect("one focusable themed button");
        assert_eq!(tab_index.0, 0);
    }
}
