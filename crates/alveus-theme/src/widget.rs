//! Composable BSN scenes for common widgets.

use std::borrow::Cow;

use bevy::{
    ecs::system::IntoObserverSystem,
    input_focus::{AutoFocus, tab_navigation::TabIndex},
    prelude::*,
    ui::auto_directional_navigation::AutoDirectionalNavigation,
    ui_widgets::Button as UiButton,
};

use crate::{interaction::InteractionPalette, palette::*};

/// A root UI scene that fills the window and centers its content.
pub fn ui_root(name: impl Into<Cow<'static, str>>) -> impl Scene {
    let name = Name::new(name);
    bsn! {
        name
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            row_gap: rem(1),
        }
        Pickable::IGNORE
    }
}

/// A simple header label. Bigger than [`label`].
pub fn header(text: impl Into<String>) -> impl Scene {
    let text = text.into();
    bsn! {
        Name::new("Header")
        Text(text)
        TextFont { font_size: rem(2) }
        TextColor(HEADER_TEXT)
    }
}

/// A simple text label.
pub fn label(text: impl Into<String>) -> impl Scene {
    let text = text.into();
    bsn! {
        Name::new("Label")
        Text(text)
        TextFont { font_size: rem(1.2) }
        TextColor(LABEL_TEXT)
    }
}

/// A large rounded button with an entity observer attached by the scene.
pub fn button<E, M, I>(text: impl Into<String>, action: I) -> impl Scene
where
    E: EntityEvent,
    M: 'static,
    I: IntoObserverSystem<E, M> + Clone + Send + Sync + 'static,
{
    bsn! {
        @button_scene(text.into(), false)
        on(action)
    }
}

/// A large button that receives focus when its menu is spawned.
pub fn button_autofocus<E, M, I>(text: impl Into<String>, action: I) -> impl Scene
where
    E: EntityEvent,
    M: 'static,
    I: IntoObserverSystem<E, M> + Clone + Send + Sync + 'static,
{
    bsn! {
        @button_scene(text.into(), true)
        on(action)
    }
}

/// A small square button, composed by patching only its layout fields.
pub fn button_small<E, M, I>(text: impl Into<String>, action: I) -> impl Scene
where
    E: EntityEvent,
    M: 'static,
    I: IntoObserverSystem<E, M> + Clone + Send + Sync + 'static,
{
    bsn! {
        @button_scene(text.into(), false)
        Node {
            width: rem(1.5),
            height: rem(1.5),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::ZERO,
        }
        on(action)
    }
}

/// Spawn a standard large button whose meaning is supplied by an ECS bundle.
/// App-level `Activate` observers inspect that bundle to route the action.
pub fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    text: impl Into<String>,
    auto_focus: bool,
    extra: impl Bundle,
) -> Entity {
    parent
        .spawn_empty()
        .apply_scene(button_scene(text.into(), auto_focus))
        .insert(extra)
        .id()
}

/// Shared scene for both command-backed list rows and observer-backed buttons.
fn button_scene(text: String, auto_focus: bool) -> impl Scene {
    let focus = auto_focus.then_some(bsn! { AutoFocus });
    bsn! {
        Name::new("Button Inner")
        UiButton
        ThemedButton
        TabIndex(0)
        AutoDirectionalNavigation::default()
        BackgroundColor(BUTTON_BACKGROUND)
        BorderColor::all(Color::NONE)
        InteractionPalette {
            none: BUTTON_BACKGROUND,
            hovered: BUTTON_HOVERED_BACKGROUND,
            pressed: BUTTON_PRESSED_BACKGROUND,
        }
        Node {
            width: rem(19),
            height: rem(4),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(3)),
            border_radius: BorderRadius::MAX,
        }
        @focus
        Children [
            Name::new("Button Text")
            Text(text)
            TextFont { font_size: rem(2) }
            TextColor(BUTTON_TEXT)
            Pickable::IGNORE
        ]
    }
}

/// Marker for the standard Bevy button entities styled by this crate.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct ThemedButton;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{scene::ScenePlugin, ui_widgets::Activate};

    #[derive(Resource, Default)]
    struct Activations(usize);

    fn record_activation(_: On<Activate>, mut count: ResMut<Activations>) {
        count.0 += 1;
    }

    fn scene_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
        app.init_resource::<Activations>();
        app
    }

    #[test]
    fn themed_scene_preserves_focus_children_and_activation() {
        let mut app = scene_app();
        let world = app.world_mut();
        let button = world
            .spawn_scene(button_autofocus("Continue", record_activation))
            .expect("button scene")
            .id();
        assert!(world.get::<UiButton>(button).is_some());
        assert!(world.get::<ThemedButton>(button).is_some());
        assert!(world.get::<AutoFocus>(button).is_some());
        assert!(world.get::<AutoDirectionalNavigation>(button).is_some());
        assert_eq!(world.get::<TabIndex>(button).unwrap().0, 0);
        let child = world.get::<Children>(button).unwrap()[0];
        assert_eq!(world.get::<Text>(child).unwrap().as_str(), "Continue");
        world.trigger(Activate { entity: button });
        assert_eq!(world.resource::<Activations>().0, 1);
    }

    #[test]
    fn small_scene_patches_layout_without_losing_button_style() {
        let mut app = scene_app();
        let world = app.world_mut();
        let button = world
            .spawn_scene(button_small("+", record_activation))
            .expect("small button scene")
            .id();
        let node = world.get::<Node>(button).unwrap();
        assert_eq!(node.width, rem(1.5));
        assert_eq!(node.height, rem(1.5));
        assert_eq!(node.align_items, AlignItems::Center);
        assert!(world.get::<InteractionPalette>(button).is_some());
        assert!(world.get::<AutoFocus>(button).is_none());
        assert_eq!(world.get::<Children>(button).unwrap().len(), 1);
    }
}
