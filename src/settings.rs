use bevy::prelude::*;
use bevy::audio::Volume;
use bevy::window::{WindowMode, PrimaryWindow, MonitorSelection, VideoModeSelection};
use crate::{GameMusicVolume, MusicTrack, FONT_PATH};
use std::path::PathBuf;

pub struct SettingsPlugin;

// ── SettingsOrigin ────────────────────────────────────────────────────────────

/// Tracks whether the settings panel is open, and what context to return to.
#[derive(Resource, PartialEq, Eq)]
pub enum SettingsOrigin {
    MainMenu,
    Paused,
}

// ── GameWindowMode ────────────────────────────────────────────────────────────

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameWindowMode {
    Windowed,
    #[default]
    BorderlessFullscreen,
    Fullscreen,
}

impl GameWindowMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Windowed => "Windowed",
            Self::BorderlessFullscreen => "Borderless FS",
            Self::Fullscreen => "Fullscreen",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Windowed => Self::BorderlessFullscreen,
            Self::BorderlessFullscreen => Self::Fullscreen,
            Self::Fullscreen => Self::Windowed,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Windowed => Self::Fullscreen,
            Self::BorderlessFullscreen => Self::Windowed,
            Self::Fullscreen => Self::BorderlessFullscreen,
        }
    }

    pub fn to_bevy(self) -> WindowMode {
        match self {
            Self::Windowed => WindowMode::Windowed,
            Self::BorderlessFullscreen => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            Self::Fullscreen => WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current),
        }
    }
}

// ── KeyBindings ───────────────────────────────────────────────────────────────

/// All remappable player actions. Each field stores the KeyCode currently bound to that action.
/// Serialized into config.ron so bindings persist between sessions.
#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyBindings {
    pub move_left:      KeyCode,
    pub move_right:     KeyCode,
    pub move_up:        KeyCode,
    pub move_down:      KeyCode,
    pub dash:           KeyCode,
    pub shoot:          KeyCode,
    pub swap_weapon:    KeyCode,
    pub interact:       KeyCode,
    pub toggle_minimap:    KeyCode,
    pub toggle_music:      KeyCode,
    pub pause:             KeyCode,
    pub toggle_inventory:  KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            move_left:      KeyCode::KeyA,
            move_right:     KeyCode::KeyD,
            move_up:        KeyCode::KeyW,
            move_down:      KeyCode::KeyS,
            dash:           KeyCode::ShiftLeft,
            shoot:          KeyCode::Space,
            swap_weapon:    KeyCode::KeyQ,
            interact:       KeyCode::KeyE,
            toggle_minimap:   KeyCode::Tab,
            toggle_music:     KeyCode::KeyM,
            pause:            KeyCode::Escape,
            toggle_inventory: KeyCode::KeyI,
        }
    }
}

impl KeyBindings {
    /// Returns the key bound to the given action.
    pub fn key_for(&self, action: BindableAction) -> KeyCode {
        match action {
            BindableAction::MoveLeft      => self.move_left,
            BindableAction::MoveRight     => self.move_right,
            BindableAction::MoveUp        => self.move_up,
            BindableAction::MoveDown      => self.move_down,
            BindableAction::Dash          => self.dash,
            BindableAction::Shoot         => self.shoot,
            BindableAction::SwapWeapon    => self.swap_weapon,
            BindableAction::Interact      => self.interact,
            BindableAction::ToggleMinimap   => self.toggle_minimap,
            BindableAction::ToggleMusic     => self.toggle_music,
            BindableAction::Pause           => self.pause,
            BindableAction::ToggleInventory => self.toggle_inventory,
        }
    }

    /// Rebinds the given action to a new key.
    pub fn set_key(&mut self, action: BindableAction, key: KeyCode) {
        match action {
            BindableAction::MoveLeft        => self.move_left        = key,
            BindableAction::MoveRight       => self.move_right       = key,
            BindableAction::MoveUp          => self.move_up          = key,
            BindableAction::MoveDown        => self.move_down        = key,
            BindableAction::Dash            => self.dash             = key,
            BindableAction::Shoot           => self.shoot            = key,
            BindableAction::SwapWeapon      => self.swap_weapon      = key,
            BindableAction::Interact        => self.interact         = key,
            BindableAction::ToggleMinimap   => self.toggle_minimap   = key,
            BindableAction::ToggleMusic     => self.toggle_music     = key,
            BindableAction::Pause           => self.pause            = key,
            BindableAction::ToggleInventory => self.toggle_inventory = key,
        }
    }

    /// Short display string for a key, used in the Controls UI.
    pub fn display_name(key: KeyCode) -> &'static str {
        match key {
            KeyCode::KeyA => "A", KeyCode::KeyB => "B", KeyCode::KeyC => "C",
            KeyCode::KeyD => "D", KeyCode::KeyE => "E", KeyCode::KeyF => "F",
            KeyCode::KeyG => "G", KeyCode::KeyH => "H", KeyCode::KeyI => "I",
            KeyCode::KeyJ => "J", KeyCode::KeyK => "K", KeyCode::KeyL => "L",
            KeyCode::KeyM => "M", KeyCode::KeyN => "N", KeyCode::KeyO => "O",
            KeyCode::KeyP => "P", KeyCode::KeyQ => "Q", KeyCode::KeyR => "R",
            KeyCode::KeyS => "S", KeyCode::KeyT => "T", KeyCode::KeyU => "U",
            KeyCode::KeyV => "V", KeyCode::KeyW => "W", KeyCode::KeyX => "X",
            KeyCode::KeyY => "Y", KeyCode::KeyZ => "Z",
            KeyCode::Space        => "Space",
            KeyCode::Tab          => "Tab",
            KeyCode::Escape       => "Escape",
            KeyCode::ShiftLeft    => "Shift",
            KeyCode::ShiftRight   => "RShift",
            KeyCode::ControlLeft  => "Ctrl",
            KeyCode::ControlRight => "RCtrl",
            KeyCode::AltLeft      => "Alt",
            KeyCode::AltRight     => "RAlt",
            KeyCode::Digit0 => "0", KeyCode::Digit1 => "1", KeyCode::Digit2 => "2",
            KeyCode::Digit3 => "3", KeyCode::Digit4 => "4", KeyCode::Digit5 => "5",
            KeyCode::Digit6 => "6", KeyCode::Digit7 => "7", KeyCode::Digit8 => "8",
            KeyCode::Digit9 => "9",
            KeyCode::ArrowLeft  => "Left",  KeyCode::ArrowRight => "Right",
            KeyCode::ArrowUp    => "Up",    KeyCode::ArrowDown  => "Down",
            _ => "?",
        }
    }
}

/// All keys that can be bound to an action.
pub const REBINDABLE_KEYS: &[KeyCode] = &[
    KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD, KeyCode::KeyE,
    KeyCode::KeyF, KeyCode::KeyG, KeyCode::KeyH, KeyCode::KeyI, KeyCode::KeyJ,
    KeyCode::KeyK, KeyCode::KeyL, KeyCode::KeyM, KeyCode::KeyN, KeyCode::KeyO,
    KeyCode::KeyP, KeyCode::KeyQ, KeyCode::KeyR, KeyCode::KeyS, KeyCode::KeyT,
    KeyCode::KeyU, KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX, KeyCode::KeyY,
    KeyCode::KeyZ,
    KeyCode::Space, KeyCode::Tab, KeyCode::Escape,
    KeyCode::ShiftLeft, KeyCode::ShiftRight,
    KeyCode::ControlLeft, KeyCode::ControlRight,
    KeyCode::AltLeft, KeyCode::AltRight,
    KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4,
    KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
    KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp, KeyCode::ArrowDown,
];

// ── BindableAction + BindingState ─────────────────────────────────────────────

/// Each variant mirrors one field in KeyBindings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BindableAction {
    MoveLeft, MoveRight, MoveUp, MoveDown,
    Dash, Shoot, SwapWeapon, Interact,
    ToggleMinimap, ToggleMusic, Pause, ToggleInventory,
}

impl BindableAction {
    fn label(self) -> &'static str {
        match self {
            Self::MoveLeft      => "Move Left",
            Self::MoveRight     => "Move Right",
            Self::MoveUp        => "Move Up",
            Self::MoveDown      => "Move Down",
            Self::Dash          => "Dash",
            Self::Shoot         => "Shoot",
            Self::SwapWeapon    => "Swap Weapon",
            Self::Interact      => "Interact",
            Self::ToggleMinimap   => "Toggle Minimap",
            Self::ToggleMusic     => "Toggle Music",
            Self::Pause           => "Pause",
            Self::ToggleInventory => "Toggle Inventory",
        }
    }
}

/// `None` = idle. `Some(action)` = waiting for a keypress to rebind that action.
#[derive(Resource, Default)]
pub struct BindingState {
    pub listening_for: Option<BindableAction>,
}

// ── UI marker components ──────────────────────────────────────────────────────

#[derive(Component)] pub struct SettingsUI;
#[derive(Component)] pub struct ControlsUI;
#[derive(Component)] pub struct BindingButton(pub BindableAction);
#[derive(Component)] pub struct BindingLabel(pub BindableAction);
#[derive(Component)] struct ControlsBackButton;
#[derive(Component)] struct VolumeDisplay;
#[derive(Component)] struct WindowModeDisplay;

#[derive(Component)]
enum SettingsButton {
    VolumeDown,
    VolumeUp,
    WindowModeLeft,
    WindowModeRight,
    Controls,
    Back,
}

// ── Config + persistence ──────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
struct Config {
    volume: f32,
    window_mode_index: u8,
    #[serde(default)]
    key_bindings: KeyBindings,
}

impl Default for Config {
    fn default() -> Self {
        Self { volume: 0.5, window_mode_index: 1, key_bindings: KeyBindings::default() }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cleanup_crew")
        .join("config.ron")
}

pub fn load_config() -> (f32, GameWindowMode, KeyBindings) {
    let path = config_path();
    let cfg: Config = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| ron::from_str(&s).ok())
        .unwrap_or_default();

    let mode = match cfg.window_mode_index {
        0 => GameWindowMode::Windowed,
        2 => GameWindowMode::Fullscreen,
        _ => GameWindowMode::BorderlessFullscreen,
    };
    (cfg.volume, mode, cfg.key_bindings)
}

fn save_config(volume: f32, mode: GameWindowMode, bindings: &KeyBindings) {
    let cfg = Config {
        volume,
        window_mode_index: match mode {
            GameWindowMode::Windowed => 0,
            GameWindowMode::BorderlessFullscreen => 1,
            GameWindowMode::Fullscreen => 2,
        },
        key_bindings: bindings.clone(),
    };
    if let Ok(s) = ron::to_string(&cfg) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, s);
    }
}

// ── Plugin ────────────────────────────────────────────────────────────────────

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<BindingState>()
            .add_systems(Update, handle_settings_buttons.run_if(resource_exists::<SettingsOrigin>))
            .add_systems(Update, update_volume_display)
            .add_systems(Update, update_window_mode_display)
            .add_systems(Update, sync_volume_to_sinks)
            .add_systems(Update, sync_window_mode)
            .add_systems(Update, handle_controls_buttons.run_if(any_with_component::<ControlsUI>))
            .add_systems(Update, listen_for_key.run_if(any_with_component::<ControlsUI>))
            .add_systems(Update, sync_controls_ui.run_if(any_with_component::<ControlsUI>));
    }
}

// ── Settings overlay ──────────────────────────────────────────────────────────

pub fn open_settings(
    commands: &mut Commands,
    assets: &AssetServer,
    current_volume: f32,
    current_window_mode: GameWindowMode,
    origin: SettingsOrigin,
) {
    commands.insert_resource(origin);

    let font: Handle<Font> = assets.load(FONT_PATH);
    let vol_text = format!("{}%", (current_volume * 100.0).round() as u32);

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            ZIndex(300),
            SettingsUI,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(380.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(22.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.14, 0.97)),
                BorderColor(Color::srgba(0.3, 0.3, 0.7, 0.8)),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|panel| {
                // Title
                panel.spawn((Node::default(),)).with_children(|c| {
                    c.spawn((
                        Text::new("SETTINGS"),
                        TextFont { font: font.clone(), font_size: 34.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });

                // Music Volume row
                panel
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },))
                    .with_children(|row| {
                        row.spawn((Node::default(),)).with_children(|c| {
                            c.spawn((
                                Text::new("Music Volume"),
                                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                                TextColor(Color::srgb(0.85, 0.85, 0.85)),
                            ));
                        });

                        row.spawn((Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },))
                        .with_children(|ctrl| {
                            spawn_small_button(ctrl, font.clone(), "-", SettingsButton::VolumeDown);
                            ctrl.spawn((Node {
                                width: Val::Px(58.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },))
                            .with_children(|c| {
                                c.spawn((
                                    Text::new(vol_text),
                                    TextFont { font: font.clone(), font_size: 22.0, ..default() },
                                    TextColor(Color::WHITE),
                                    VolumeDisplay,
                                ));
                            });
                            spawn_small_button(ctrl, font.clone(), "+", SettingsButton::VolumeUp);
                        });
                    });

                // Window Mode row
                panel
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },))
                    .with_children(|row| {
                        row.spawn((Node::default(),)).with_children(|c| {
                            c.spawn((
                                Text::new("Window Mode"),
                                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                                TextColor(Color::srgb(0.85, 0.85, 0.85)),
                            ));
                        });

                        row.spawn((Node {
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },))
                        .with_children(|ctrl| {
                            spawn_small_button(ctrl, font.clone(), "<", SettingsButton::WindowModeLeft);
                            ctrl.spawn((Node {
                                width: Val::Px(110.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },))
                            .with_children(|c| {
                                c.spawn((
                                    Text::new(current_window_mode.label()),
                                    TextFont { font: font.clone(), font_size: 18.0, ..default() },
                                    TextColor(Color::WHITE),
                                    WindowModeDisplay,
                                ));
                            });
                            spawn_small_button(ctrl, font.clone(), ">", SettingsButton::WindowModeRight);
                        });
                    });

                // Controls button
                panel
                    .spawn((
                        Button,
                        SettingsButton::Controls,
                        Node {
                            width: Val::Px(240.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.2, 0.35, 0.9)),
                        BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Controls"),
                            TextFont { font: font.clone(), font_size: 24.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                // Back button
                panel
                    .spawn((
                        Button,
                        SettingsButton::Back,
                        Node {
                            width: Val::Px(240.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect { top: Val::Px(8.0), ..default() },
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.25, 0.1, 0.1, 0.9)),
                        BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Back"),
                            TextFont { font: font.clone(), font_size: 24.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
            });
        });
}

fn spawn_small_button(parent: &mut ChildSpawnerCommands, font: Handle<Font>, label: &str, button: SettingsButton) {
    parent
        .spawn((
            Button,
            button,
            Node {
                width: Val::Px(38.0),
                height: Val::Px(38.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9)),
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont { font, font_size: 26.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
}

fn handle_settings_buttons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut interactions: Query<(&Interaction, &SettingsButton), (Changed<Interaction>, With<Button>)>,
    mut volume: ResMut<GameMusicVolume>,
    mut window_mode: ResMut<GameWindowMode>,
    bindings: Res<KeyBindings>,
    ui_q: Query<Entity, With<SettingsUI>>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            SettingsButton::VolumeDown => {
                volume.0 = (volume.0 - 0.1).max(0.0);
            }
            SettingsButton::VolumeUp => {
                volume.0 = (volume.0 + 0.1).min(1.0);
            }
            SettingsButton::WindowModeLeft => {
                *window_mode = window_mode.prev();
            }
            SettingsButton::WindowModeRight => {
                *window_mode = window_mode.next();
            }
            SettingsButton::Controls => {
                open_controls(&mut commands, &asset_server, &bindings);
            }
            SettingsButton::Back => {
                save_config(volume.0, *window_mode, &bindings);
                commands.remove_resource::<SettingsOrigin>();
                for e in &ui_q {
                    commands.entity(e).despawn();
                }
            }
        }
    }
}

fn update_volume_display(
    volume: Res<GameMusicVolume>,
    mut text_q: Query<&mut Text, With<VolumeDisplay>>,
) {
    if !volume.is_changed() { return; }
    let label = format!("{}%", (volume.0 * 100.0).round() as u32);
    for mut t in &mut text_q {
        *t = Text::new(&label);
    }
}

fn update_window_mode_display(
    mode: Res<GameWindowMode>,
    mut text_q: Query<&mut Text, With<WindowModeDisplay>>,
) {
    if !mode.is_changed() { return; }
    for mut t in &mut text_q {
        *t = Text::new(mode.label());
    }
}

fn sync_volume_to_sinks(
    volume: Res<GameMusicVolume>,
    mut sinks: Query<&mut AudioSink, With<MusicTrack>>,
) {
    if !volume.is_changed() { return; }
    for mut sink in &mut sinks {
        sink.set_volume(Volume::Linear(volume.0));
    }
}

fn sync_window_mode(
    mode: Res<GameWindowMode>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !mode.is_changed() { return; }
    if let Ok(mut window) = windows.single_mut() {
        window.mode = mode.to_bevy();
    }
}

// ── Controls overlay ──────────────────────────────────────────────────────────

const ALL_ACTIONS: &[BindableAction] = &[
    BindableAction::MoveLeft,  BindableAction::MoveRight,
    BindableAction::MoveUp,    BindableAction::MoveDown,
    BindableAction::Dash,      BindableAction::Shoot,
    BindableAction::SwapWeapon, BindableAction::Interact,
    BindableAction::ToggleMinimap, BindableAction::ToggleMusic,
    BindableAction::Pause, BindableAction::ToggleInventory,
];

pub fn open_controls(commands: &mut Commands, assets: &AssetServer, bindings: &KeyBindings) {
    let font: Handle<Font> = assets.load(FONT_PATH);

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            ZIndex(400),
            ControlsUI,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(480.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.14, 0.97)),
                BorderColor(Color::srgba(0.3, 0.3, 0.7, 0.8)),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|panel| {
                // Title
                panel.spawn((Node::default(),)).with_children(|c| {
                    c.spawn((
                        Text::new("CONTROLS"),
                        TextFont { font: font.clone(), font_size: 30.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });

                // One row per action
                for &action in ALL_ACTIONS {
                    spawn_binding_row(panel, font.clone(), action, bindings.key_for(action));
                }

                // Back button
                panel
                    .spawn((
                        Button,
                        ControlsBackButton,
                        Node {
                            width: Val::Px(240.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect { top: Val::Px(8.0), ..default() },
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.25, 0.1, 0.1, 0.9)),
                        BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Back"),
                            TextFont { font: font.clone(), font_size: 24.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
            });
        });
}

fn spawn_binding_row(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    action: BindableAction,
    current_key: KeyCode,
) {
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|row| {
            // Action label
            row.spawn((Node {
                width: Val::Px(180.0),
                ..default()
            },))
            .with_children(|c| {
                c.spawn((
                    Text::new(action.label()),
                    TextFont { font: font.clone(), font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                ));
            });

            // Key button
            row.spawn((
                Button,
                BindingButton(action),
                Node {
                    width: Val::Px(130.0),
                    height: Val::Px(38.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9)),
                BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new(KeyBindings::display_name(current_key)),
                    TextFont { font, font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                    BindingLabel(action),
                ));
            });
        });
}

fn handle_controls_buttons(
    mut commands: Commands,
    mut binding_state: ResMut<BindingState>,
    btn_q: Query<(&Interaction, &BindingButton), (Changed<Interaction>, With<Button>)>,
    back_q: Query<&Interaction, (Changed<Interaction>, With<ControlsBackButton>)>,
    controls_ui_q: Query<Entity, With<ControlsUI>>,
    volume: Res<GameMusicVolume>,
    window_mode: Res<GameWindowMode>,
    bindings: Res<KeyBindings>,
) {
    // Start listening when a binding button is pressed
    for (interaction, btn) in &btn_q {
        if *interaction == Interaction::Pressed {
            binding_state.listening_for = Some(btn.0);
        }
    }

    // Back: save and close
    for interaction in &back_q {
        if *interaction == Interaction::Pressed {
            save_config(volume.0, *window_mode, &bindings);
            binding_state.listening_for = None;
            for e in &controls_ui_q {
                commands.entity(e).despawn();
            }
        }
    }
}

fn listen_for_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut binding_state: ResMut<BindingState>,
    mut bindings: ResMut<KeyBindings>,
) {
    let Some(action) = binding_state.listening_for else { return };

    for &key in keys.get_just_pressed() {
        if REBINDABLE_KEYS.contains(&key) {
            bindings.set_key(action, key);
            binding_state.listening_for = None;
            return;
        }
    }
}

/// Keeps all binding button labels and highlight colors in sync with current state.
fn sync_controls_ui(
    binding_state: Res<BindingState>,
    bindings: Res<KeyBindings>,
    mut label_q: Query<(&BindingLabel, &mut Text)>,
    mut button_q: Query<(&BindingButton, &mut BackgroundColor)>,
) {
    if !binding_state.is_changed() && !bindings.is_changed() { return; }

    for (lbl, mut text) in &mut label_q {
        if binding_state.listening_for == Some(lbl.0) {
            *text = Text::new("...");
        } else {
            *text = Text::new(KeyBindings::display_name(bindings.key_for(lbl.0)));
        }
    }

    for (btn, mut bg) in &mut button_q {
        *bg = if binding_state.listening_for == Some(btn.0) {
            BackgroundColor(Color::srgba(0.4, 0.25, 0.05, 0.9))
        } else {
            BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9))
        };
    }
}
