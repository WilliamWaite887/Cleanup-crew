use bevy::prelude::*;
use crate::{GameState, FONT_PATH, SelectedWeapon, SelectedRun, StationLevel, PlanetCount, SavedPlayerBuffs, BeamRifleUnlocked};
use crate::weapons::WeaponType;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Setup), setup_screen)
            .add_systems(
                Update,
                (handle_weapon_buttons, handle_run_buttons, handle_action_buttons)
                    .run_if(in_state(GameState::Setup)),
            )
            .add_systems(OnExit(GameState::Setup), cleanup_setup);
    }
}

#[derive(Component)]
struct SetupUI;

/// Marks a weapon card button with which weapon it represents.
#[derive(Component)]
struct WeaponCard(WeaponType);

#[derive(Component)]
enum RunCycleButton {
    Prev,
    Next,
}

#[derive(Component)]
struct RunLabel;

#[derive(Component)]
enum SetupActionButton {
    StartRun,
    Back,
}

const CARD_SELECTED_BG: Color = Color::srgba(0.05, 0.15, 0.35, 0.95);
const CARD_SELECTED_BORDER: Color = Color::srgba(0.2, 0.8, 1.0, 1.0);
const CARD_UNSELECTED_BG: Color = Color::srgba(0.07, 0.07, 0.12, 0.9);
const CARD_UNSELECTED_BORDER: Color = Color::srgba(0.3, 0.3, 0.5, 0.5);

fn setup_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut selected_weapon: ResMut<SelectedWeapon>,
    unlocked: Res<BeamRifleUnlocked>,
) {
    // If BeamRifle is somehow selected but not unlocked, reset to Zapper.
    if selected_weapon.0 == WeaponType::BeamRifle && !unlocked.0 {
        selected_weapon.0 = WeaponType::Zapper;
    }

    let font = asset_server.load(FONT_PATH);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            ZIndex(200),
            SetupUI,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(580.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(18.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.14, 0.97)),
                BorderColor(Color::srgba(0.3, 0.3, 0.6, 0.8)),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|panel| {
                // Title
                panel.spawn((
                    Text::new("LOADOUT"),
                    TextFont { font: font.clone(), font_size: 38.0, ..default() },
                    TextColor(Color::WHITE),
                ));

                // Weapon section label
                panel.spawn((
                    Text::new("WEAPON"),
                    TextFont { font: font.clone(), font_size: 18.0, ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.9, 1.0)),
                ));

                // Weapon cards row
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(20.0),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_weapon_card(row, font.clone(), WeaponType::Zapper,    "Zapper",     "High burst, slow rate", selected_weapon.0, true);
                        spawn_weapon_card(row, font.clone(), WeaponType::BeamRifle, "Beam Rifle", "Fast rate, lower burst", selected_weapon.0, unlocked.0);
                    });

                // Run section label
                panel.spawn((
                    Text::new("RUN"),
                    TextFont { font: font.clone(), font_size: 18.0, ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.9, 1.0)),
                ));

                // Run selector row: < label >
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(14.0),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_cycle_button(row, font.clone(), "<", RunCycleButton::Prev);

                        row.spawn(Node {
                            width: Val::Px(120.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|c| {
                            c.spawn((
                                Text::new("Run 1"),
                                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                                TextColor(Color::WHITE),
                                RunLabel,
                            ));
                        });

                        spawn_cycle_button(row, font.clone(), ">", RunCycleButton::Next);
                    });

                // Action buttons
                spawn_action_button(panel, font.clone(), "Start Run", SetupActionButton::StartRun, Color::srgba(0.08, 0.42, 0.08, 0.9));
                spawn_action_button(panel, font.clone(), "Back",      SetupActionButton::Back,     Color::srgba(0.25, 0.08, 0.08, 0.9));
            });
        });
}

fn spawn_weapon_card(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    weapon_type: WeaponType,
    name: &str,
    desc: &str,
    selected: WeaponType,
    is_available: bool,
) {
    let is_selected = is_available && weapon_type == selected;
    let bg = if is_selected { CARD_SELECTED_BG } else { CARD_UNSELECTED_BG };
    let border = if is_selected { CARD_SELECTED_BORDER } else { CARD_UNSELECTED_BORDER };
    let name_color = if is_available { Color::WHITE } else { Color::srgba(0.45, 0.45, 0.5, 1.0) };
    let desc_text = if is_available { desc.to_string() } else { "Complete Planet 1".to_string() };
    let desc_color = if is_available { Color::srgba(0.75, 0.75, 0.85, 1.0) } else { Color::srgba(0.5, 0.4, 0.25, 1.0) };

    parent
        .spawn((
            Button,
            WeaponCard(weapon_type),
            Node {
                width: Val::Px(185.0),
                height: Val::Px(110.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor(border),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(name),
                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                TextColor(name_color),
            ));
            card.spawn((
                Text::new(desc_text),
                TextFont { font, font_size: 14.0, ..default() },
                TextColor(desc_color),
            ));
        });
}

fn spawn_cycle_button(parent: &mut ChildSpawnerCommands, font: Handle<Font>, label: &str, btn: RunCycleButton) {
    parent
        .spawn((
            Button,
            btn,
            Node {
                width: Val::Px(38.0),
                height: Val::Px(38.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.3, 0.9)),
            BorderColor(Color::srgba(0.4, 0.4, 0.7, 0.6)),
            BorderRadius::all(Val::Px(5.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont { font, font_size: 22.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, font: Handle<Font>, label: &str, btn: SetupActionButton, bg: Color) {
    parent
        .spawn((
            Button,
            btn,
            Node {
                width: Val::Px(260.0),
                height: Val::Px(52.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg),
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont { font, font_size: 24.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}

fn handle_weapon_buttons(
    mut selected: ResMut<SelectedWeapon>,
    unlocked: Res<BeamRifleUnlocked>,
    interactions: Query<(&Interaction, &WeaponCard), (Changed<Interaction>, With<Button>)>,
    mut cards: Query<(&WeaponCard, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (interaction, card) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // Ignore clicks on locked weapons.
        if card.0 == WeaponType::BeamRifle && !unlocked.0 {
            continue;
        }
        selected.0 = card.0;
    }

    if !selected.is_changed() {
        return;
    }

    for (card, mut bg, mut border) in &mut cards {
        let is_locked = card.0 == WeaponType::BeamRifle && !unlocked.0;
        let is_selected = !is_locked && card.0 == selected.0;
        if is_selected {
            *bg = BackgroundColor(CARD_SELECTED_BG);
            *border = BorderColor(CARD_SELECTED_BORDER);
        } else {
            *bg = BackgroundColor(CARD_UNSELECTED_BG);
            *border = BorderColor(CARD_UNSELECTED_BORDER);
        }
    }
}

fn handle_run_buttons(
    selected: Res<SelectedRun>,
    interactions: Query<(&Interaction, &RunCycleButton), (Changed<Interaction>, With<Button>)>,
    mut label_q: Query<&mut Text, With<RunLabel>>,
) {
    for (interaction, btn) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // Only "Run 1" exists for now — both buttons are no-ops.
        let _ = (btn, &selected);
        if let Ok(mut text) = label_q.single_mut() {
            text.0 = format!("Run {}", selected.0 + 1);
        }
    }
}

fn handle_action_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &SetupActionButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, btn) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match btn {
            SetupActionButton::StartRun => {
                commands.insert_resource(StationLevel(0));
                commands.insert_resource(PlanetCount(0));
                commands.remove_resource::<SavedPlayerBuffs>();
                next_state.set(GameState::Loading);
            }
            SetupActionButton::Back => {
                next_state.set(GameState::Menu);
            }
        }
    }
}

fn cleanup_setup(mut commands: Commands, root_q: Query<Entity, With<SetupUI>>) {
    for e in &root_q {
        commands.entity(e).despawn();
    }
}
