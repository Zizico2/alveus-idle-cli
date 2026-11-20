//! The credits menu.

use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use alveus_app::Menu;
use alveus_asset_tracking::LoadResource;
use alveus_audio::music;
use alveus_command::GameCommand;
use alveus_theme::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Credits), spawn_credits_menu);

    app.load_resource::<CreditsAssets>();
    app.add_systems(OnEnter(Menu::Credits), start_credits_music);
}

fn spawn_credits_menu(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        @widget::ui_root("Credits Menu")
        GlobalZIndex(2)
        DespawnOnExit::<Menu>(Menu::Credits)
        Children [
            @widget::header("Created by")
            --
            @created_by()
            --
            @widget::header("Assets")
            --
            @assets()
            --
            @widget::button_autofocus("Back", go_back_on_click)
        ]
    });
}

fn created_by() -> impl Scene {
    grid(vec![
        ["Bernardo António Borda d'Água", "Design & programming"],
        [
            "Alveus Sanctuary",
            "Ambassadors & inspiration (fan project)",
        ],
    ])
}

fn assets() -> impl Scene {
    grid(vec![
        ["Button SFX", "CC0 by Jaszunio15"],
        ["Music", "CC BY 3.0 by Kevin MacLeod"],
        [
            "Bevy logo",
            "All rights reserved by the Bevy Foundation, permission granted for splash screen use when unmodified",
        ],
    ])
}

fn grid(content: Vec<[&'static str; 2]>) -> impl Scene {
    let cells = content
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, text)| {
            bsn! {
                @widget::label(text)
                Node {
                    justify_self: {if i.is_multiple_of(2) {
                        JustifySelf::End
                    } else {
                        JustifySelf::Start
                    }},
                }
            }
        })
        .collect::<Vec<_>>();
    bsn! {
        Name::new("Grid")
        Node {
            display: Display::Grid,
            row_gap: px(10),
            column_gap: px(30),
            grid_template_columns: RepeatedGridTrack::px::<Vec<RepeatedGridTrack>> (2, 400.0),
        }
        Children [ {cells} ]
    }
}

fn go_back_on_click(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::Back);
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct CreditsAssets {
    #[dependency]
    music: Handle<AudioSource>,
}

impl FromWorld for CreditsAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/Monkeys Spinning Monkeys.ogg"),
        }
    }
}

fn start_credits_music(mut commands: Commands, credits_music: Res<CreditsAssets>) {
    commands.spawn((
        Name::new("Credits Music"),
        DespawnOnExit::<Menu>(Menu::Credits),
        music(credits_music.music.clone()),
    ));
}
