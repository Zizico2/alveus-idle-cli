use bevy::prelude::*;
use crate::stats::{AnimalId, AnimalStats, SanctuaryUpkeep, StatType};
use crate::screens::Screen;

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
    animal_id: String,
    stat_type: StatType,
}

#[derive(Component)]
struct AnimalStatText {
    animal_id: String,
    stat_type: StatType,
}

#[derive(Component)]
struct NeglectBanner;

// ---------------------------------------------------------
// Plugin
// ---------------------------------------------------------

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        // Spawn HUD when gameplay screen is entered
        app.add_systems(OnEnter(Screen::Gameplay), spawn_hud_system);

        // Despawn HUD when returning to non-gameplay screens
        app.add_systems(
            OnEnter(Screen::Title),
            despawn_hud_system,
        );
        app.add_systems(
            OnEnter(Screen::Splash),
            despawn_hud_system,
        );
        app.add_systems(
            OnEnter(Screen::Loading),
            despawn_hud_system,
        );

        // Update systems run when player is actively playing
        app.add_systems(
            PostUpdate,
            (
                update_hud_system,
                animate_neglect_banner_system,
            )
                .run_if(in_gameplay_or_room),
        );
    }
}

fn in_gameplay_or_room(screen_state: Res<State<Screen>>) -> bool {
    matches!(
        screen_state.get(),
        Screen::Gameplay | Screen::InRoom(_)
    )
}

// ---------------------------------------------------------
// Spawning HUD UI
// ---------------------------------------------------------

fn spawn_hud_system(
    mut commands: Commands,
    query: Query<Entity, With<StatsHudUi>>,
) {
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
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
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
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 0.8, 0.7)),
                    ));

                    upkeep_card.spawn((
                        Text::new("100%"),
                        TextFont {
                            font_size: 36.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.2, 0.9, 0.6)), // Radiant teal/green
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

                // B. Animal Stats Cards (vertical container)
                panel.spawn((
                    Name::new("Animal Cards Stack"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                )).with_children(|stack| {
                    // Spawn animal cards for the 4 ambassadors
                    spawn_animal_card(stack, "polly", "Polly", "Silkie Chicken", "Playpen");
                    spawn_animal_card(stack, "stompy", "Stompy", "Emu", "Pasture Grassland");
                    spawn_animal_card(stack, "georgie", "Georgie", "African Bullfrog", "Reptile Enclosure");
                    spawn_animal_card(stack, "siren", "Siren", "Ball Python", "Reptile Enclosure");
                });

                // C. Debug Controls Help Card
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
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.4)),
                    ));
                    debug_card.spawn((
                        Text::new("1-3: Polly (Feed/Clean/Enrich)\n4-6: Stompy (Feed/Clean/Enrich)\n7-9: Georgie (Feed/Clean/Enrich)\n0/I/O: Siren (Feed/Clean/Enrich)\nNote: Georgie & Siren share Reptile Enclosure!\n- (or M): Worsen stats  = (or L): Fast-forward"),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ));
                });
            });
        });
}

fn spawn_animal_card(
    parent: &mut ChildSpawnerCommands,
    animal_id: &str,
    name: &str,
    species: &str,
    enclosure_name: &str,
) {
    parent.spawn((
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
    )).with_children(|card| {
        // Name and Species/Enclosure info
        card.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
        )).with_children(|header| {
            header.spawn(
                Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                }
            ).with_children(|left| {
                left.spawn((
                    Text::new(name),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                left.spawn((
                    Text::new(format!("[{}]", enclosure_name)),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.5, 0.8, 0.9)), // Sleek cyan-blue
                ));
            });
            header.spawn((
                Text::new(species),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
        });

        // Stats Progress Bars
        spawn_stat_row(card, animal_id, StatType::Hunger, "Hunger", Color::srgb(0.2, 0.8, 0.3));
        spawn_stat_row(card, animal_id, StatType::Cleanliness, "Cleanliness", Color::srgb(0.2, 0.6, 0.9));
        spawn_stat_row(card, animal_id, StatType::Happiness, "Happiness", Color::srgb(0.8, 0.4, 0.9));
    });
}

fn spawn_stat_row(
    parent: &mut ChildSpawnerCommands,
    animal_id: &str,
    stat_type: StatType,
    label_text: &str,
    bar_color: Color,
) {
    parent.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },
    )).with_children(|row| {
        // Label & Percentage Text
        row.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
        )).with_children(|labels| {
            labels.spawn((
                Text::new(label_text),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            labels.spawn((
                Text::new("100%"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                AnimalStatText {
                    animal_id: animal_id.to_string(),
                    stat_type,
                },
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
        )).with_children(|track| {
            track.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(bar_color),
                AnimalStatBarFill {
                    animal_id: animal_id.to_string(),
                    stat_type,
                },
            ));
        });
    });
}

// ---------------------------------------------------------
// Despawning HUD UI
// ---------------------------------------------------------

fn despawn_hud_system(
    mut commands: Commands,
    query: Query<Entity, With<StatsHudUi>>,
) {
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
    mut upkeep_text_query: Query<&mut Text, (With<UpkeepText>, Without<AnimalStatText>)>,
    mut upkeep_bar_query: Query<&mut Node, (With<UpkeepBarFill>, Without<AnimalStatBarFill>)>,
    mut stat_text_query: Query<(&mut Text, &AnimalStatText), Without<UpkeepText>>,
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
        animal_stats_map.insert(id.0.clone(), stats.clone());
        animal_enclosure_map.insert(id.0.clone(), enclosure.0.clone());
    }

    let mut enclosure_cleanliness_map = std::collections::HashMap::new();
    for (id, stats) in &enclosures_query {
        enclosure_cleanliness_map.insert(id.0.clone(), stats.cleanliness);
    }

    // Helper to resolve the value of a stat for a given animal_id
    let resolve_stat = |animal_id: &str, stat_type: StatType| -> Option<u32> {
        match stat_type {
            StatType::Hunger => animal_stats_map.get(animal_id).map(|s| s.hunger),
            StatType::Happiness => animal_stats_map.get(animal_id).map(|s| s.happiness),
            StatType::Cleanliness => {
                let enc_id = animal_enclosure_map.get(animal_id)?;
                enclosure_cleanliness_map.get(enc_id).copied()
            }
        }
    };

    // 3. Update Individual Stat Texts
    for (mut txt, marker) in &mut stat_text_query {
        if let Some(val) = resolve_stat(&marker.animal_id, marker.stat_type) {
            txt.0 = format!("{}%", (val as f32 / 10.0).round() as i32);
        }
    }

    // 4. Update Individual Stat Progress Bars
    for (mut node, marker) in &mut stat_bar_query {
        if let Some(val) = resolve_stat(&marker.animal_id, marker.stat_type) {
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

/// Animate the Neglect Banner with a pulsating glow/alpha transition.
fn animate_neglect_banner_system(
    time: Res<Time>,
    mut banner_query: Query<&mut BackgroundColor, With<NeglectBanner>>,
) {
    for mut bg in &mut banner_query {
        // Compute pulsating alpha value
        let pulse = (time.elapsed_secs() * 4.0).sin().abs() * 0.35 + 0.65;
        bg.0.set_alpha(pulse);
    }
}
