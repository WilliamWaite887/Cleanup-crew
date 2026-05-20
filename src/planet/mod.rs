use bevy::prelude::*;
use crate::room::RoomVec;
use crate::{GameState, PlanetLevelMarker, StationLevel};
use crate::procgen::ProcgenSet;

mod shared;
mod planet1;
mod planet2;
mod planet3;

// ── Components ───────────────────────────────────────────────────────────────

#[derive(Component)]
pub(super) struct BackgroundSprite;

#[derive(Resource)]
pub(super) struct BackgroundRes {
    pub(super) stars: Handle<Image>,
    pub(super) planet_station: Handle<Image>,
}

#[derive(Component)]
pub struct FinalBoss;

#[derive(Component)]
pub(super) struct PlanetWinScreen;

#[derive(Component)]
pub(super) struct BossHealthBarRoot;

#[derive(Component)]
pub(super) struct BossHealthBarFill;

/// A door on the planet level that requires the 3-digit station code to open.
#[derive(Component)]
pub struct CodeDoor {
    pub unlocked: bool,
}

/// Marker for the floating "[E] Enter Code" prompt near a locked door.
#[derive(Component)]
pub(super) struct CodeDoorPrompt;

/// Marker for the keypad UI overlay.
#[derive(Component)]
pub(super) struct CodeEntryUi;

/// Marker for the individual digit Text nodes inside the keypad.
#[derive(Component)]
pub(super) struct CodeDigitSlot(pub(super) usize);

/// Marker for the keypad status line ("INCORRECT CODE" / "ENTER CODE").
#[derive(Component)]
pub(super) struct CodeStatusText;

/// Tracks whether the boss arena has been entered and the boss spawned.
#[derive(Resource, PartialEq, Eq)]
pub(super) enum BossArenaState {
    Idle,
    Active,
}

/// Collidable wall that seals the exit corridor until the boss is defeated.
#[derive(Component)]
pub(super) struct BossExitDoor;

/// The "Leave Planet" interactable that spawns in the exit room after the boss dies.
#[derive(Component)]
pub(super) struct PlanetExitBeacon;

/// Active code-entry session.
#[derive(Resource)]
pub struct CodeEntryState {
    pub(super) door_entity: Entity,
    pub(super) entered: [u8; 3],
    pub(super) cursor: usize,
    pub(super) wrong_timer: Option<Timer>,
}

/// Color terminal — requires the 3 station color clues to unlock.
#[derive(Component)]
pub struct ColorTerminal {
    pub unlocked: bool,
}

/// Symbol terminal — requires the 3 station symbol clues to unlock.
#[derive(Component)]
pub struct SymbolTerminal {
    pub unlocked: bool,
}

/// Frequency Master — requires all 3 signal strengths; opens the boss arena gate.
#[derive(Component)]
pub struct FreqMaster {
    pub unlocked: bool,
}

/// Signal strengths revealed by solving the 3 sub-puzzles.
/// [0] = Signal A (CodeDoor), [1] = Signal B (ColorTerminal), [2] = Signal C (SymbolTerminal).
/// Generated fresh each planet; not persisted.
#[derive(Resource, Default)]
pub struct PlanetSignals {
    pub signals: [Option<u8>; 3],
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum TerminalKind {
    Color,
    Symbol,
    Freq,
}

/// Active terminal session for Color / Symbol / Freq Master keypads.
#[derive(Resource)]
pub struct TerminalSession {
    pub(super) terminal_entity: Entity,
    pub(super) kind: TerminalKind,
    pub(super) entered: [u8; 3],
    pub(super) cursor: usize,
    pub(super) wrong_timer: Option<Timer>,
    pub(super) planet_idx: u32,
    pub(super) font: Handle<Font>,
}

/// Marker for the floating prompt near a terminal.
#[derive(Component)]
pub(super) struct TerminalPrompt;

/// Marker for the terminal keypad UI overlay.
#[derive(Component)]
pub(super) struct TerminalUi;

/// Marker for individual value slots inside the terminal keypad.
#[derive(Component)]
pub(super) struct TerminalSlot(pub(super) usize);

/// Marker for the terminal status text line.
#[derive(Component)]
pub(super) struct TerminalStatusText;

// ── Dial system (Planet 2 & 3) ───────────────────────────────────────────────

/// Dial targets generated when each terminal is solved on P2/P3.
/// [0]=code dial target (0-9), [1]=color dial target (0-3), [2]=symbol dial target (0-5).
#[derive(Resource, Default)]
pub struct DialTargets {
    pub targets: [Option<u8>; 3],
}

#[derive(Clone, Copy, PartialEq)]
pub enum DialType {
    Code,
    Color,
    Symbol,
}

/// A physical dial in an enemy room. dial_idx matches DialTargets index.
#[derive(Component)]
pub struct DialButton {
    pub dial_idx:  usize,
    pub dial_type: DialType,
    pub current:   u8,
}

/// Active dial interaction session (resource present while dial UI is open).
#[derive(Resource)]
pub struct DialInteractState {
    pub(super) dial_entity: Entity,
    pub(super) dial_idx:    usize,
    pub(super) dial_type:   DialType,
    pub(super) current:     u8,
}

/// Marker for the dial UI overlay panel.
#[derive(Component)]
pub(super) struct DialUi;

/// Marker for the cycling value text inside the dial UI.
#[derive(Component)]
pub(super) struct DialCurrentText;

/// Marker for the floating prompt near a dial.
#[derive(Component)]
pub(super) struct DialPrompt;

/// Planet 2 boss door — E press opens when all dials are correctly set.
#[derive(Component)]
pub struct PlanetBossDoor {
    pub ready: bool,
}

/// Floating prompt near the Planet 2 boss door.
#[derive(Component)]
pub(super) struct PlanetBossDoorPrompt;

/// Mini boss arena gate (Planet 3) — collidable wall removed when all dials correct.
#[derive(Component)]
pub struct MiniBossGate;

/// Mini boss entity (Planet 3 only).
#[derive(Component)]
pub struct MiniBoss;

/// Tracks the mini boss arena state on Planet 3.
#[derive(Resource, PartialEq, Eq)]
pub(super) enum MiniBossArenaState {
    Idle,
    Active,
    Done,
}

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, shared::load_background_assets)
            .add_systems(
                OnEnter(GameState::Loading),
                planet1::setup_planet_level
                    .in_set(ProcgenSet::BuildFullLevel)
                    .after(ProcgenSet::LoadRooms)
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                shared::tint_station_background
                    .run_if(|sl: Res<StationLevel>, m: Option<Res<PlanetLevelMarker>>|
                        sl.0 % 3 == 2 && m.is_none()),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                shared::spawn_stars_background
                    .run_if(|sl: Res<StationLevel>, m: Option<Res<PlanetLevelMarker>>|
                        sl.0 % 3 != 2 && m.is_none()),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                shared::spawn_planet_station_background
                    .run_if(|sl: Res<StationLevel>, m: Option<Res<PlanetLevelMarker>>|
                        sl.0 % 3 == 2 && m.is_none()),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                (
                    shared::tint_planet_background,
                    planet1::init_boss_arena_state,
                    shared::spawn_vault_rewards,
                    planet1::spawn_boss_exit_door,
                    planet1::inject_test_planet_clues,
                )
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                shared::update_background_position.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    planet1::boss_arena_trigger,
                    shared::spawn_boss_chest,
                    shared::interact_with_exit_beacon,
                    shared::update_boss_health_bar,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                (planet1::code_door_proximity, planet1::update_code_entry_ui)
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                planet1::init_planet_resources
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                planet1::terminal_proximity
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                planet1::update_terminal_ui
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                (
                    planet2::dial_proximity,
                    planet2::update_dial_ui,
                    planet2::check_all_dials,
                    planet2::boss_door_proximity,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                (planet3::mini_boss_arena_trigger, planet3::watch_mini_boss_death)
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(OnEnter(GameState::PlanetWin), shared::setup_planet_win_screen)
            .add_systems(OnExit(GameState::PlanetWin), shared::cleanup_planet_win_screen)
            .add_systems(OnExit(GameState::Playing), shared::restore_background);
    }
}

// ── Per-planet dispatch ───────────────────────────────────────────────────────

pub(super) fn planet_map_file(planet_idx: usize) -> &'static str {
    match planet_idx {
        0 => "assets/planet/planet1_level.txt",
        1 => "assets/planet/planet2_level.txt",
        _ => "assets/planet/planet3_level.txt",
    }
}

pub(super) fn planet_boss_spawn(_planet_idx: usize) -> Vec3 {
    planet1::P1_BOSS_SPAWN
}

pub(super) fn planet_vault_rewards(_planet_idx: usize) -> &'static [Vec3] {
    &planet1::P1_VAULT_REWARDS
}

pub(super) fn build_planet_rooms(planet_idx: usize) -> RoomVec {
    match planet_idx {
        1 => planet2::build_planet2_rooms(),
        2 => planet3::build_planet3_rooms(),
        _ => planet1::build_planet1_rooms(),
    }
}
