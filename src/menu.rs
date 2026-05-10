use bevy::prelude::*;
use bevy::app::AppExit;

use crate::{GameState, GameMusicVolume, MusicTrack, PlanetLevelMarker};
use crate::settings;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(OnEnter(GameState::Menu), start_menu_music)
            .add_systems(Update, handle_buttons.run_if(in_state(GameState::Menu)))
            .add_systems(OnExit(GameState::Menu), cleanup_menu)
            .add_systems(OnExit(GameState::Menu), stop_menu_music);
    }
}

#[derive(Component)]
struct MenuUI;

#[derive(Component)]
enum MenuButton {
    Play,
    // PlayPlanet,
    Credits,
    Settings,
    Quit,
}

#[derive(Component)]
struct MenuMusic;

fn setup_menu(
    mut commands: Commands,
    assets: Res<AssetServer>,
) {
    // Root canvas
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ZIndex(100), // on top of world
            MenuUI,
        ))
        .with_children(|root| {
            // Background
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(0.0),
                    top: Val::Percent(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(assets.load("menu/Title_BG.png")),
            ));

            // Cleanup Crew Title
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(0.0),
                    top: Val::Percent(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(assets.load("menu/Title_Text.png")),
            ));

            root
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(40.0),
                        margin: UiRect {
                            top: Val::Px(100.0),
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|col| {
                    // Play
                    col.spawn((
                        Button,
                        MenuButton::Play,
                        ImageNode::new(assets.load("menu/Title_Play.png")),
                    ));

                    // Planet Test Button
                    // col.spawn((
                    //     Button,
                    //     MenuButton::PlayPlanet,
                    //     Node {
                    //         width: Val::Px(420.0),
                    //         height: Val::Px(60.0),
                    //         justify_content: JustifyContent::Center,
                    //         align_items: AlignItems::Center,
                    //         padding: UiRect::all(Val::Px(8.0)),
                    //         ..default()
                    //     },
                    //     BackgroundColor(Color::srgba(0.05, 0.2, 0.05, 0.8)),
                    //     BorderColor(Color::srgba(0.3, 1.0, 0.3, 0.5)),
                    //     BorderRadius::all(Val::Px(6.0)),
                    // ))
                    // .with_children(|b| {
                    //     b.spawn((
                    //         Text::new("Test Planet"),
                    //         TextFont { font_size: 28.0, ..default() },
                    //     ));
                    // });

                    // Credits
                    col.spawn((
                        Button,
                        MenuButton::Credits,
                        ImageNode::new(assets.load("menu/Title_Credits.png")),
                    ));

                    // Settings
                    col.spawn((
                        Button,
                        MenuButton::Settings,
                        Node {
                            width: Val::Px(420.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.1, 0.3, 0.8)),
                        BorderColor(Color::srgba(0.4, 0.4, 1.0, 0.5)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Settings"),
                            TextFont { font_size: 28.0, ..default() },
                        ));
                    });

                    // Quit
                    col.spawn((
                        Button,
                        MenuButton::Quit,
                        Node {
                            width: Val::Px(420.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.05, 0.05, 0.8)),
                        BorderColor(Color::srgba(1.0, 0.3, 0.3, 0.5)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Quit"),
                            TextFont { font_size: 28.0, ..default() },
                        ));
                    });

                    // Controls reference
                    col.spawn((
                        Node {
                            width: Val::Px(540.0),
                            padding: UiRect::all(Val::Px(12.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|panel| {
                        for line in [
                            "Controls",
                            "WASD — Move          Shift — Dash",
                            "Left Click — Shoot",
                            "B — Broom (sweep, deflect bullets, fix windows)",
                            "Tab — Toggle Minimap",
                            "M — Toggle Music       Esc — Pause",
                        ] {
                            let size = if line == "Controls" { 18.0 } else { 15.0 };
                            panel.spawn((
                                Text::new(line),
                                TextFont { font_size: size, ..default() },
                                TextColor(if line == "Controls" {
                                    Color::srgba(1.0, 1.0, 0.5, 1.0)
                                } else {
                                    Color::WHITE
                                }),
                            ));
                        }
                    });
                });
        });
}

fn start_menu_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let music_handle = asset_server.load("audio/menu_music.ogg");
    
    commands.spawn((
        AudioPlayer::new(music_handle),
        PlaybackSettings::LOOP,
        MenuMusic,
        MusicTrack,
    ));
    
    debug!("Menu music started");
}

fn stop_menu_music(
    mut commands: Commands,
    music_query: Query<Entity, With<MenuMusic>>,
) {
    for entity in &music_query {
        commands.entity(entity).despawn();
        debug!("Menu music stopped");
    }
}

fn handle_buttons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    volume: Res<GameMusicVolume>,
    window_mode: Res<settings::GameWindowMode>,
    mut interactions: Query<(&Interaction, &MenuButton, Entity), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: EventWriter<AppExit>,
) {
    for (interaction, which, _button_entity) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match which {
            MenuButton::Play => {
                next_state.set(GameState::Loading);
            }
            // MenuButton::PlayPlanet => {
            //     commands.insert_resource(PlanetLevelMarker);
            //     next_state.set(GameState::Loading);
            // }
            MenuButton::Credits => {
                next_state.set(GameState::EndCredits);
            }
            MenuButton::Settings => {
                settings::open_settings(
                    &mut commands,
                    &asset_server,
                    volume.0,
                    *window_mode,
                    settings::SettingsOrigin::MainMenu,
                );
            }
            MenuButton::Quit => {
                app_exit.write(AppExit::Success);
            }
        }
    }
}

fn cleanup_menu(mut commands: Commands, root_q: Query<Entity, With<MenuUI>>) {
    for e in &root_q {
        commands.entity(e).despawn();
    }
}
