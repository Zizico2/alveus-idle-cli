use crate::AppSystems;
use crate::content::item_display_name;
use crate::interaction::{
    ActiveInteractionTarget, FeedAnimal, GiveItem, LastPickupMessage, PlayerSatchel,
};
use crate::screens::{InRoom, Screen};
use crate::stats::{AnimalId, AnimalStat, AnimalStats, SanctuaryUpkeep};
use bevy::prelude::*;

// ---------------------------------------------------------
// Component Markers
// ---------------------------------------------------------

#[derive(Component)]
pub struct StatsHudUi;

#[derive(Component)]
struct UpkeepText;

#[derive(Component)]
struct UpkeepBarFill;

#[derive(Component)]
struct AnimalStatBarFill {
    animal_id: AnimalId,
    stat: AnimalStat,
}

#[derive(Component)]
struct AnimalStatText {
    animal_id: AnimalId,
    stat: AnimalStat,
}

#[derive(Component)]
struct NeglectBanner;

#[derive(Component)]
struct InteractionPromptRoot;

#[derive(Component)]
struct InteractionPromptText;

#[derive(Component)]
struct SatchelHudRoot;

#[derive(Component)]
struct SatchelBodyText;

// ---------------------------------------------------------
// Plugin
// ---------------------------------------------------------

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        // Spawn HUD when gameplay screen is entered
        app.add_systems(OnEnter(Screen::Gameplay), spawn_hud_system);

        // Despawn HUD when returning to non-gameplay screens
        app.add_systems(OnEnter(Screen::Title), despawn_hud_system);
        app.add_systems(OnEnter(Screen::Splash), despawn_hud_system);
        app.add_systems(OnEnter(Screen::Loading), despawn_hud_system);

        // Update systems run when player is actively playing
        app.add_systems(
            Update,
            (
                update_hud_system,
                update_room_feedback_hud_system,
                animate_neglect_banner_system,
            )
                .in_set(AppSystems::UiUpdate)
                .run_if(in_gameplay_or_room),
        );
    }
}

fn in_gameplay_or_room(screen_state: Res<State<Screen>>) -> bool {
    matches!(screen_state.get(), Screen::Gameplay | Screen::InRoom(_))
}

// ---------------------------------------------------------
// Spawning HUD UI
// ---------------------------------------------------------

fn spawn_hud_system(mut commands: Commands, query: Query<Entity, With<StatsHudUi>>) {
    // Avoid double-spawning if HUD already exists
    if !query.is_empty() {
        return;
    }

    info!("Spawning permanent HUD UI Overlay...");

    // Main HUD Root node (spans the entire screen, transparent, ignores clicks)
    commands
        .spawn((
            Name::new("Stats HUD Overlay Root"),
            StatsHudUi,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            // 1. Neglect Freeze Banner at the very top (hidden by default)
            parent.spawn((
                Name::new("Neglect Banner"),
                NeglectBanner,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(40.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.8, 0.1, 0.1, 0.9)),
                BorderColor::all(Color::srgb(1.0, 0.2, 0.2)),
                Visibility::Hidden,
                children![(
                    Text::new("⚠️ SANCTUARY NEGLECTED – PROGRESS HALTED!"),
                    TextFont::from_font_size(20.0),
                    TextColor(Color::WHITE),
                )],
            ));

            // 2. HUD Right Panel (Upkeep & Animal stats cards)
            parent.spawn((
                Name::new("Stats Panel"),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    top: Val::Px(24.0),
                    width: Val::Px(300.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
            )).with_children(|panel| {
                // A. Sanctuary Upkeep Card
                panel.spawn((
                    Name::new("Upkeep Card"),
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        border_radius: BorderRadius::all(Val::Px(12.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.85)), // Glassmorphism dark cyan
                    BorderColor::all(Color::srgba(0.2, 0.8, 0.6, 0.3)),
                )).with_children(|upkeep_card| {
                    upkeep_card.spawn((
                        Text::new("SANCTUARY UPKEEP"),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::srgb(0.5, 0.8, 0.7)),
                    ));

                    upkeep_card.spawn((
                        Text::new("100%"),
                        TextFont::from_font_size(36.0),
                        TextColor(Color::srgb(0.2, 0.9, 0.6)), // Radiant teal/green
                        TextLayout::no_wrap(),
                        UpkeepText,
                    ));

                    // Small upkeep progress bar track
                    upkeep_card.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(4.0),
                            border_radius: BorderRadius::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.1)),
                    )).with_children(|track| {
                        track.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                border_radius: BorderRadius::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.9, 0.6)),
                            UpkeepBarFill,
                        ));
                    });
                });

                // B. Satchel Card (in the stats column — avoids overlapping animal cards)
                panel.spawn((
                    Name::new("Satchel Card"),
                    SatchelHudRoot,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.75)),
                    BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.05)),
                )).with_children(|card| {
                    card.spawn((
                        Text::new("SATCHEL"),
                        TextFont::from_font_size(11.0),
                        TextColor(Color::srgb(0.5, 0.8, 0.7)),
                    ));
                    card.spawn((
                        Text::new("Empty"),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::WHITE),
                        TextLayout::no_wrap(),
                        SatchelBodyText,
                    ));
                });

                // C. Animal Stats Cards (vertical container)
                panel.spawn((
                    Name::new("Animal Cards Stack"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                )).with_children(|stack| {
                    // Spawn animal cards for the 4 ambassadors
                    spawn_animal_card(stack, AnimalId::Polly, "Polly", "Silkie Chicken", "Playpen");
                    spawn_animal_card(stack, AnimalId::PushPop, "Push Pop", "Sulcata Tortoise", "Push Pop Enclosure");
                    spawn_animal_card(stack, AnimalId::Stompy, "Stompy", "Emu", "Pasture Grassland");
                    spawn_animal_card(stack, AnimalId::Georgie, "Georgie", "African Bullfrog", "Reptile Enclosure");
                    spawn_animal_card(stack, AnimalId::Siren, "Siren", "Ball Python", "Reptile Enclosure");
                });

                // D. Debug Controls Help Card
                panel.spawn((
                    Name::new("Debug Card"),
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.8)),
                    BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.15)),
                )).with_children(|debug_card| {
                    debug_card.spawn((
                        Text::new("DEBUG CARE SHORTCUTS"),
                        TextFont::from_font_size(11.0),
                        TextColor(Color::srgb(0.8, 0.8, 0.4)),
                    ));
                    debug_card.spawn((
                        Text::new("1-3: Polly (Feed/Clean/Enrich)\n4-6: Stompy (Feed/Clean/Enrich)\n7-9: Georgie (Feed/Clean/Enrich)\n0/I/O: Siren (Feed/Clean/Enrich)\nU/J/Y: Push Pop (Feed/Clean/Enrich)\nNote: Georgie & Siren share Reptile Enclosure!\n- (or M): Worsen stats  = (or L): Fast-forward"),
                        TextFont::from_font_size(10.0),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ));
                });
            });

            // 3. Interaction prompt (bottom-left, same card style as upkeep)
            parent.spawn((
                Name::new("Interaction Prompt Card"),
                InteractionPromptRoot,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    left: Val::Px(24.0),
                    width: Val::Auto,
                    padding: UiRect::all(Val::Px(16.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    border_radius: BorderRadius::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.85)),
                BorderColor::all(Color::srgba(0.2, 0.8, 0.6, 0.3)),
            )).with_children(|card| {
                card.spawn((
                    Text::new(" "),
                    TextFont::from_font_size(16.0),
                    TextColor(Color::WHITE),
                    TextLayout::no_wrap(),
                    InteractionPromptText,
                ));
            });
        });
}

fn spawn_animal_card(
    parent: &mut ChildSpawnerCommands,
    animal_id: AnimalId,
    name: &str,
    species: &str,
    enclosure_name: &str,
) {
    parent
        .spawn((
            Name::new(format!("Animal Card - {}", name)),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.75)), // Glassmorphic charcoal
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.05)),
        ))
        .with_children(|card| {
            // Name and Species/Enclosure info
            card.spawn((Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },))
                .with_children(|header| {
                    header
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            ..default()
                        })
                        .with_children(|left| {
                            left.spawn((
                                Text::new(name),
                                TextFont::from_font_size(16.0),
                                TextColor(Color::WHITE),
                            ));
                            left.spawn((
                                Text::new(format!("[{}]", enclosure_name)),
                                TextFont::from_font_size(10.0),
                                TextColor(Color::srgb(0.5, 0.8, 0.9)), // Sleek cyan-blue
                            ));
                        });
                    header.spawn((
                        Text::new(species),
                        TextFont::from_font_size(11.0),
                        TextColor(Color::srgb(0.6, 0.6, 0.6)),
                    ));
                });

            // Stats Progress Bars
            spawn_stat_row(
                card,
                animal_id,
                AnimalStat::Hunger,
                "Hunger",
                Color::srgb(0.2, 0.8, 0.3),
            );
            spawn_stat_row(
                card,
                animal_id,
                AnimalStat::Cleanliness,
                "Cleanliness",
                Color::srgb(0.2, 0.6, 0.9),
            );
            spawn_stat_row(
                card,
                animal_id,
                AnimalStat::Happiness,
                "Happiness",
                Color::srgb(0.8, 0.4, 0.9),
            );
        });
}

fn spawn_stat_row(
    parent: &mut ChildSpawnerCommands,
    animal_id: AnimalId,
    stat: AnimalStat,
    label_text: &str,
    bar_color: Color,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },))
        .with_children(|row| {
            // Label & Percentage Text
            row.spawn((Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },))
                .with_children(|labels| {
                    labels.spawn((
                        Text::new(label_text),
                        TextFont::from_font_size(11.0),
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    labels.spawn((
                        Text::new("100%"),
                        TextFont::from_font_size(11.0),
                        TextColor(Color::WHITE),
                        TextLayout::no_wrap(),
                        AnimalStatText { animal_id, stat },
                    ));
                });

            // Progress Bar Track
            row.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(6.0),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.05)),
            ))
            .with_children(|track| {
                track.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(bar_color),
                    AnimalStatBarFill { animal_id, stat },
                ));
            });
        });
}

// ---------------------------------------------------------
// Despawning HUD UI
// ---------------------------------------------------------

fn despawn_hud_system(mut commands: Commands, query: Query<Entity, With<StatsHudUi>>) {
    for entity in &query {
        info!("Despawning Stats HUD UI Overlay...");
        // In Bevy 0.18, calling despawn() automatically recursively cleans up child hierarchies
        commands.entity(entity).despawn();
    }
}

// ---------------------------------------------------------
// Update & Animation Systems
// ---------------------------------------------------------

fn update_hud_system(
    upkeep: Res<SanctuaryUpkeep>,
    animals_query: Query<(&AnimalId, &AnimalStats, &crate::stats::AnimalEnclosure)>,
    enclosures_query: Query<(&crate::stats::EnclosureId, &crate::stats::EnclosureStats)>,
    mut upkeep_text_query: Query<
        &mut Text,
        (
            With<UpkeepText>,
            Without<AnimalStatText>,
            Without<SatchelBodyText>,
            Without<InteractionPromptText>,
        ),
    >,
    mut upkeep_bar_query: Query<&mut Node, (With<UpkeepBarFill>, Without<AnimalStatBarFill>)>,
    mut stat_text_query: Query<
        (&mut Text, &AnimalStatText),
        (
            Without<UpkeepText>,
            Without<SatchelBodyText>,
            Without<InteractionPromptText>,
        ),
    >,
    mut stat_bar_query: Query<(&mut Node, &AnimalStatBarFill), Without<UpkeepBarFill>>,
    mut neglect_banner_query: Query<&mut Visibility, With<NeglectBanner>>,
) {
    // 1. Update Upkeep Score UI
    let upkeep_percentage = (upkeep.score * 100.0).round() as i32;
    for mut txt in &mut upkeep_text_query {
        txt.0 = format!("{}%", upkeep_percentage);
    }
    for mut node in &mut upkeep_bar_query {
        node.width = Val::Percent(upkeep.score * 100.0);
    }

    // 2. Map animal_ids to their stats, and enclosures to their cleanliness
    let mut animal_stats_map = std::collections::HashMap::new();
    let mut animal_enclosure_map = std::collections::HashMap::new();
    for (id, stats, enclosure) in &animals_query {
        animal_stats_map.insert(*id, stats.clone());
        animal_enclosure_map.insert(*id, enclosure.0);
    }

    let mut enclosure_cleanliness_map = std::collections::HashMap::new();
    for (id, stats) in &enclosures_query {
        enclosure_cleanliness_map.insert(*id, stats.cleanliness);
    }

    // Helper to resolve the value of a stat for a given animal_id
    let resolve_stat = |animal_id: AnimalId, stat: AnimalStat| -> Option<u32> {
        match stat {
            AnimalStat::Hunger => animal_stats_map.get(&animal_id).map(|s| s.hunger),
            AnimalStat::Happiness => animal_stats_map.get(&animal_id).map(|s| s.happiness),
            AnimalStat::Cleanliness => {
                let enc_id = animal_enclosure_map.get(&animal_id)?;
                enclosure_cleanliness_map.get(enc_id).copied()
            }
        }
    };

    // 3. Update Individual Stat Texts
    for (mut txt, marker) in &mut stat_text_query {
        if let Some(val) = resolve_stat(marker.animal_id, marker.stat) {
            txt.0 = format!("{}%", (val as f32 / 10.0).round() as i32);
        }
    }

    // 4. Update Individual Stat Progress Bars
    for (mut node, marker) in &mut stat_bar_query {
        if let Some(val) = resolve_stat(marker.animal_id, marker.stat) {
            node.width = Val::Percent(val as f32 / 10.0);
        }
    }

    // 5. Update Neglect Banner Visibility
    for mut vis in &mut neglect_banner_query {
        let desired_visibility = if upkeep.score < 0.30 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != desired_visibility {
            *vis = desired_visibility;
        }
    }
}

fn update_room_feedback_hud_system(
    screen: Res<State<Screen>>,
    satchel: Res<PlayerSatchel>,
    pickup_message: Res<LastPickupMessage>,
    active: Res<ActiveInteractionTarget>,
    give_query: Query<&GiveItem>,
    feed_query: Query<&FeedAnimal>,
    mut interaction_root: Query<
        &mut Node,
        (
            With<InteractionPromptRoot>,
            Without<SatchelHudRoot>,
            Without<UpkeepBarFill>,
            Without<AnimalStatBarFill>,
        ),
    >,
    mut interaction_text: Query<
        &mut Text,
        (
            With<InteractionPromptText>,
            Without<UpkeepText>,
            Without<AnimalStatText>,
            Without<SatchelBodyText>,
        ),
    >,
    mut satchel_root: Query<
        &mut Node,
        (
            With<SatchelHudRoot>,
            Without<InteractionPromptRoot>,
            Without<UpkeepBarFill>,
            Without<AnimalStatBarFill>,
        ),
    >,
    mut satchel_body: Query<
        &mut Text,
        (
            With<SatchelBodyText>,
            Without<UpkeepText>,
            Without<AnimalStatText>,
            Without<InteractionPromptText>,
        ),
    >,
) {
    let in_interactive_room = matches!(
        screen.get(),
        Screen::InRoom(InRoom::NutritionHouse) | Screen::InRoom(InRoom::PushPopEnclosure)
    );

    let prompt_message = if in_interactive_room {
        active.interactable.and_then(|entity| {
            if let Ok(give) = give_query.get(entity) {
                return Some(format!("Press [Space] to {}", give.prompt));
            }
            if let Ok(feed) = feed_query.get(entity) {
                return Some(format!("Press [Space] to {}", feed.prompt));
            }
            None
        })
    } else {
        None
    };

    let prompt_display = if prompt_message.is_some() {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut interaction_root {
        if node.display != prompt_display {
            node.display = prompt_display;
        }
    }

    if let Some(message) = &prompt_message {
        for mut txt in &mut interaction_text {
            if txt.as_str() != message {
                txt.0 = message.clone();
            }
        }
    }

    let on_overview_with_item = matches!(screen.get(), Screen::Gameplay) && satchel.item.is_some();
    let show_satchel = in_interactive_room || on_overview_with_item;
    let satchel_display = if show_satchel {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut satchel_root {
        if node.display != satchel_display {
            node.display = satchel_display;
        }
    }

    let body_label = satchel_body_label(&pickup_message, &satchel);
    for mut txt in &mut satchel_body {
        if txt.as_str() != body_label {
            txt.0 = body_label.clone();
        }
    }
}

fn satchel_body_label(pickup_message: &LastPickupMessage, satchel: &PlayerSatchel) -> String {
    if let Some(message) = &pickup_message.text {
        return message.clone();
    }

    if let Some(item) = satchel.item {
        return format!("Carrying: {}\nPress [K] to drop", item_display_name(item));
    }

    "Empty".to_string()
}

/// Animate the Neglect Banner with a pulsating glow/alpha transition.
fn animate_neglect_banner_system(
    time: Res<Time>,
    mut banner_query: Query<(&mut BackgroundColor, &Visibility), With<NeglectBanner>>,
) {
    for (mut bg, visibility) in &mut banner_query {
        if visibility == Visibility::Hidden {
            continue;
        }
        // Compute pulsating alpha value
        let pulse = (time.elapsed_secs() * 4.0).sin().abs() * 0.35 + 0.65;
        bg.0.set_alpha(pulse);
    }
}
