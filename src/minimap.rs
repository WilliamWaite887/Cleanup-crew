use bevy::prelude::*;
use std::collections::HashSet;
use crate::map::{LevelRes, MapGridMeta};
use crate::player::{Player, WeaponBuffStacks};
use crate::room::{LevelState, RoomVec};
use crate::station_code::StationCodes;
use crate::station_color::StationColors;
use crate::station_symbol::StationSymbols;
use crate::planet::PlanetSignals;
use crate::weapons::WeaponInventory;
use crate::{GameEntity, GameState, TILE_SIZE};

const MINIMAP_W: f32 = 420.0;
const MINIMAP_H: f32 = 420.0;

/// Coarse tile resolution for hallway fog-of-war (N tiles per tracked cell).
const HALLWAY_CELL: usize = 3;
/// Radius in coarse cells that the player "reveals" around themselves each frame.
const REVEAL_RADIUS: i32 = 3;

// Components

#[derive(Component)]
pub struct MinimapRoot;

#[derive(Component)]
struct MinimapRoomNode {
    room_index: usize,
}

#[derive(Component)]
struct MinimapHallwayNode {
    cell_col: i32,
    cell_row: i32,
}

#[derive(Component)]
struct MinimapPlayerDot;

/// Marker for each weapon text row in the inventory panel (index = slot in WeaponInventory).
#[derive(Component)]
struct InventoryWeaponLine(usize);

/// Marker for each buff text row.
#[derive(Component)]
enum InventoryBuffLine { AtkSpeed, Damage, Piercing }

/// Marker for each station clue row in the inventory panel.
#[derive(Component)]
enum InventoryClueRow { Code, Color, Symbol, Signal }

/// Marker for the key status row in the inventory panel.
#[derive(Component)]
struct InventoryKeyRow;

// Resources

#[derive(Resource, Default)]
pub struct MinimapVisible(pub bool);

/// Coarse cells (col/HALLWAY_CELL, row/HALLWAY_CELL) that the player has explored.
#[derive(Resource, Default)]
pub struct VisitedCells(pub HashSet<(i32, i32)>);

// Plugin

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapVisible>()
            .init_resource::<VisitedCells>()
            .add_systems(OnEnter(GameState::Playing), (clear_visited_cells, setup_minimap).chain())
            .add_systems(OnExit(GameState::Playing), clear_visited_cells)
            .add_systems(
                Update,
                toggle_minimap.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                update_visited_cells.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (update_minimap, update_inventory_panel)
                    .run_if(in_state(GameState::Playing))
                    .run_if(|vis: Res<MinimapVisible>| vis.0),
            );
    }
}

// Setup

fn clear_visited_cells(mut visited: ResMut<VisitedCells>) {
    visited.0.clear();
}

fn setup_minimap(
    mut commands: Commands,
    rooms: Res<RoomVec>,
    level: Res<LevelRes>,
    grid: Res<MapGridMeta>,
    asset_server: Res<AssetServer>,
) {
    let map_px_w = grid.cols as f32 * TILE_SIZE;
    let map_px_h = grid.rows as f32 * TILE_SIZE;
    let world_min_x = -map_px_w * 0.5;
    let world_max_y = map_px_h * 0.5;

    let cols = grid.cols;
    let rows = grid.rows;

    let font: Handle<Font> = asset_server.load(
        "fonts/BitcountSingleInk-VariableFont_CRSV,ELSH,ELXP,SZP1,SZP2,XPN1,XPN2,YPN1,YPN2,slnt,wght.ttf",
    );

    // Precompute which coarse cells are hallway cells (floor tile outside every room).
    let mut hallway_cells: HashSet<(i32, i32)> = HashSet::new();
    let level_rows = level.level.len();
    let level_cols = level.level.first().map(|r| r.len()).unwrap_or(0);

    for cell_row in 0..(rows / HALLWAY_CELL + 1) {
        for cell_col in 0..(cols / HALLWAY_CELL + 1) {
            let tile_col = cell_col * HALLWAY_CELL + HALLWAY_CELL / 2;
            let tile_row = cell_row * HALLWAY_CELL + HALLWAY_CELL / 2;

            if tile_row >= level_rows || tile_col >= level_cols {
                continue;
            }

            let ch = level.level[tile_row].as_bytes().get(tile_col).copied().unwrap_or(b'.');
            if ch != b'#' {
                continue;
            }

            // Skip if the coarse cell's tile AABB overlaps any room (not just center tile).
            let cell_x1 = cell_col * HALLWAY_CELL;
            let cell_y1 = cell_row * HALLWAY_CELL;
            let cell_x2 = cell_x1 + HALLWAY_CELL - 1;
            let cell_y2 = cell_y1 + HALLWAY_CELL - 1;
            let in_room = rooms.0.iter().any(|r| {
                let rx1 = r.tile_top_left_corner.x as usize;
                let ry1 = r.tile_top_left_corner.y as usize;
                let rx2 = r.tile_bot_right_corner.x as usize;
                let ry2 = r.tile_bot_right_corner.y as usize;
                cell_x1 <= rx2 && cell_x2 >= rx1 && cell_y1 <= ry2 && cell_y2 >= ry1
            });

            if !in_room {
                hallway_cells.insert((cell_col as i32, cell_row as i32));
            }
        }
    }

    let cell_w = (HALLWAY_CELL as f32 / cols as f32 * MINIMAP_W).max(2.0);
    let cell_h = (HALLWAY_CELL as f32 / rows as f32 * MINIMAP_H).max(2.0);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(32.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
            Visibility::Hidden,
            ZIndex(50),
            MinimapRoot,
            GameEntity,
        ))
        .with_children(|root| {
            // ── Left column: map ─────────────────────────────────────────────
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
            ))
            .with_children(|left| {
                // Title
                left.spawn((
                    Text::new("MAP   [TAB] to close"),
                    TextFont { font: font.clone(), font_size: 18.0, ..default() },
                    TextColor(Color::srgb(0.65, 0.65, 0.65)),
                ));

                // Legend row
                left.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(18.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|leg| {
                    legend_item(leg, Color::srgba(0.2, 0.2, 0.2, 0.6), "Unexplored");
                    legend_item(leg, Color::srgb(1.0, 0.9, 0.0),        "Current");
                    legend_item(leg, Color::srgb(0.15, 0.65, 0.15),     "Cleared");
                });

                // Map panel
                left.spawn((
                    Node {
                        width: Val::Px(MINIMAP_W),
                        height: Val::Px(MINIMAP_H),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.04, 0.12, 1.0)),
                ))
                .with_children(|panel| {
                    for (i, room) in rooms.0.iter().enumerate() {
                        let mini_x = (room.top_left_corner.x - world_min_x) / map_px_w * MINIMAP_W;
                        let mini_y = (world_max_y - room.top_left_corner.y) / map_px_h * MINIMAP_H;
                        let mini_w = ((room.bot_right_corner.x - room.top_left_corner.x).abs()
                            / map_px_w * MINIMAP_W).max(6.0);
                        let mini_h = ((room.top_left_corner.y - room.bot_right_corner.y).abs()
                            / map_px_h * MINIMAP_H).max(6.0);

                        panel.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(mini_x),
                                top: Val::Px(mini_y),
                                width: Val::Px(mini_w),
                                height: Val::Px(mini_h),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.5)),
                            MinimapRoomNode { room_index: i },
                        ));
                    }

                    for (cell_col, cell_row) in &hallway_cells {
                        let tile_col = *cell_col as usize * HALLWAY_CELL + HALLWAY_CELL / 2;
                        let tile_row = *cell_row as usize * HALLWAY_CELL + HALLWAY_CELL / 2;

                        let mini_x = (tile_col as f32 / cols as f32 * MINIMAP_W - cell_w * 0.5).max(0.0);
                        let mini_y = (tile_row as f32 / rows as f32 * MINIMAP_H - cell_h * 0.5).max(0.0);

                        panel.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(mini_x),
                                top: Val::Px(mini_y),
                                width: Val::Px(cell_w),
                                height: Val::Px(cell_h),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                            Visibility::Hidden,
                            MinimapHallwayNode { cell_col: *cell_col, cell_row: *cell_row },
                        ));
                    }

                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(8.0),
                            height: Val::Px(8.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(1.0, 1.0, 0.0)),
                        ZIndex(1),
                        MinimapPlayerDot,
                    ));
                });
            });

            // ── Right column: inventory ───────────────────────────────────────
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: Val::Px(4.0),
                    width: Val::Px(360.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    align_self: AlignSelf::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.06, 0.14, 0.95)),
                BorderColor(Color::srgba(0.3, 0.5, 0.8, 0.6)),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|inv| {
                inv.spawn((
                    Text::new("INVENTORY"),
                    TextFont { font: font.clone(), font_size: 22.0, ..default() },
                    TextColor(Color::WHITE),
                    Node { align_self: AlignSelf::Center, margin: UiRect::bottom(Val::Px(8.0)), ..default() },
                ));

                inv_section_header(inv, &font, "WEAPONS");
                // Placeholder rows — updated each frame by update_inventory_panel.
                // Spawn up to 4 weapon slots (expand as needed).
                for i in 0..4usize {
                    inv.spawn((
                        Text::new(""),
                        TextFont { font: font.clone(), font_size: 15.0, ..default() },
                        TextColor(Color::srgb(0.6, 0.6, 0.6)),
                        InventoryWeaponLine(i),
                    ));
                }

                inv_section_header(inv, &font, "BUFFS");
                inv.spawn((
                    Text::new(""),
                    TextFont { font: font.clone(), font_size: 15.0, ..default() },
                    TextColor(Color::srgb(0.4, 1.0, 0.5)),
                    InventoryBuffLine::AtkSpeed,
                ));
                inv.spawn((
                    Text::new(""),
                    TextFont { font: font.clone(), font_size: 15.0, ..default() },
                    TextColor(Color::srgb(0.4, 1.0, 0.5)),
                    InventoryBuffLine::Damage,
                ));
                inv.spawn((
                    Text::new(""),
                    TextFont { font: font.clone(), font_size: 15.0, ..default() },
                    TextColor(Color::srgb(0.4, 1.0, 0.5)),
                    InventoryBuffLine::Piercing,
                ));

                inv_section_header(inv, &font, "STATION CLUES");
                inv.spawn((
                    Text::new(""),
                    TextFont { font: font.clone(), font_size: 15.0, ..default() },
                    TextColor(Color::srgb(0.2, 1.0, 1.0)),
                    InventoryClueRow::Code,
                ));
                inv.spawn((
                    Text::new(""),
                    TextFont { font: font.clone(), font_size: 15.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.5, 0.3)),
                    InventoryClueRow::Color,
                ));
                inv.spawn((
                    Text::new(""),
                    TextFont {
                        font: asset_server.load(crate::SYMBOL_FONT_PATH),
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.4, 1.0)),
                    InventoryClueRow::Symbol,
                ));
                inv.spawn((
                    Text::new(""),
                    TextFont { font: font.clone(), font_size: 15.0, ..default() },
                    TextColor(Color::srgb(0.2, 1.0, 0.4)),
                    InventoryClueRow::Signal,
                ));

                inv_section_header(inv, &font, "KEY");
                inv.spawn((
                    Text::new(""),
                    TextFont { font: font.clone(), font_size: 15.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.85, 0.2)),
                    InventoryKeyRow,
                ));
            });
        });
}

fn inv_section_header(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, title: &str) {
    parent.spawn((
        Text::new(format!("── {} ──", title)),
        TextFont { font: font.clone(), font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.5, 0.7, 1.0)),
        Node { margin: UiRect::top(Val::Px(6.0)), ..default() },
    ));
}

fn legend_item(parent: &mut ChildSpawnerCommands, color: Color, label: &str) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Node { width: Val::Px(12.0), height: Val::Px(12.0), ..default() },
                BackgroundColor(color),
            ));
            row.spawn((
                Text::new(label),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.75, 0.75, 0.75)),
            ));
        });
}

// Systems

fn toggle_minimap(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<MinimapVisible>,
    mut root_q: Query<&mut Visibility, With<MinimapRoot>>,
    bindings: Res<crate::settings::KeyBindings>,
) {
    if !keys.just_pressed(bindings.toggle_minimap) {
        return;
    }
    visible.0 = !visible.0;
    if let Ok(mut vis) = root_q.single_mut() {
        *vis = if visible.0 { Visibility::Visible } else { Visibility::Hidden };
    }
}

fn update_inventory_panel(
    player_q: Query<(&WeaponInventory, &WeaponBuffStacks), With<Player>>,
    codes: Res<StationCodes>,
    colors: Res<StationColors>,
    symbols: Res<StationSymbols>,
    signals: Option<Res<PlanetSignals>>,
    key_state: Option<Res<crate::key_chest::LevelKeyState>>,
    mut weapon_lines: Query<(&InventoryWeaponLine, &mut Text, &mut TextColor), (Without<InventoryBuffLine>, Without<InventoryClueRow>, Without<InventoryKeyRow>)>,
    mut buff_lines: Query<(&InventoryBuffLine, &mut Text, &mut TextColor), (Without<InventoryWeaponLine>, Without<InventoryClueRow>, Without<InventoryKeyRow>)>,
    mut clue_rows: Query<(&InventoryClueRow, &mut Text, &mut TextColor), (Without<InventoryBuffLine>, Without<InventoryWeaponLine>, Without<InventoryKeyRow>)>,
    mut key_rows: Query<(&mut Text, &mut TextColor), (With<InventoryKeyRow>, Without<InventoryBuffLine>, Without<InventoryWeaponLine>, Without<InventoryClueRow>)>,
) {
    let Ok((inv, buffs)) = player_q.single() else { return };

    for (slot, mut text, mut color) in &mut weapon_lines {
        if let Some(weapon) = inv.weapons.get(slot.0) {
            let equipped = slot.0 == inv.equipped;
            let prefix = if equipped { "► " } else { "  " };
            let pierce = weapon.effective_pierce_count();
            *text = Text::new(format!(
                "{}{:<12}  dmg:{:.0}  pierce:{}",
                prefix, weapon.weapon_type.name(), weapon.damage, pierce,
            ));
            *color = TextColor(if equipped { Color::WHITE } else { Color::srgb(0.6, 0.6, 0.6) });
        } else {
            *text = Text::new("");
        }
    }

    for (buff, mut text, mut color) in &mut buff_lines {
        let (label, count) = match buff {
            InventoryBuffLine::AtkSpeed => ("Atk Speed", buffs.atk_speed),
            InventoryBuffLine::Damage   => ("Damage",    buffs.damage),
            InventoryBuffLine::Piercing => ("Piercing",  buffs.piercing),
        };
        *text = Text::new(format!("  {:<12}  +{}", label, count));
        *color = TextColor(if count > 0 { Color::srgb(0.4, 1.0, 0.5) } else { Color::srgb(0.5, 0.5, 0.5) });
    }

    let color_names = ["RED", "GRN", "BLU", "YLW"];
    let symbol_chars = crate::station_symbol::SYMBOL_CHARS;

    for (row, mut text, _) in &mut clue_rows {
        match row {
            InventoryClueRow::Code => {
                let slots: Vec<String> = codes.codes.iter().map(|c| {
                    c.map_or("[?]".to_string(), |d| format!("[{}]", d))
                }).collect();
                *text = Text::new(format!("  CODE  {}", slots.join(" ")));
            }
            InventoryClueRow::Color => {
                let slots: Vec<String> = colors.colors.iter().map(|c| {
                    c.map_or("[?  ]".to_string(), |d| format!("[{}]", color_names[d as usize]))
                }).collect();
                *text = Text::new(format!("  CLR   {}", slots.join(" ")));
            }
            InventoryClueRow::Symbol => {
                let slots: Vec<String> = symbols.symbols.iter().map(|s| {
                    s.map_or("[?]".to_string(), |d| format!("[{}]", symbol_chars[d as usize]))
                }).collect();
                *text = Text::new(format!("  SYM   {}", slots.join(" ")));
            }
            InventoryClueRow::Signal => {
                if let Some(ref sigs) = signals {
                    let slots: Vec<String> = sigs.signals.iter().map(|s| {
                        s.map_or("[?]".to_string(), |v| format!("[{}]", v))
                    }).collect();
                    *text = Text::new(format!("  SIG   {}", slots.join(" ")));
                } else {
                    *text = Text::new("  SIG   — — —");
                }
            }
        }
    }

    if let Ok((mut text, mut color)) = key_rows.single_mut() {
        let has_key = key_state.map_or(false, |k| k.has_key);
        *text = Text::new(if has_key { "  KEY   [found]" } else { "  KEY   —" });
        *color = TextColor(if has_key {
            Color::srgb(1.0, 0.85, 0.2)
        } else {
            Color::srgb(0.4, 0.4, 0.4)
        });
    }
}

/// Marks coarse cells near the player as visited so hallways reveal on the minimap.
fn update_visited_cells(
    player_q: Query<&Transform, With<Player>>,
    grid: Res<MapGridMeta>,
    mut visited: ResMut<VisitedCells>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let px = player_tf.translation.x;
    let py = player_tf.translation.y;

    let cols = grid.cols as f32;
    let rows = grid.rows as f32;
    let _map_px_w = cols * TILE_SIZE;
    let _map_px_h = rows * TILE_SIZE;

    // Convert player world pos to tile coordinates.
    let tile_col = ((px - grid.x0) / TILE_SIZE).round() as i32;
    // y0 is the world Y of the bottom tile; row 0 is the top tile.
    let tile_row = (rows as i32 - 1) - ((py - grid.y0) / TILE_SIZE).round() as i32;

    let coarse_col = tile_col / HALLWAY_CELL as i32;
    let coarse_row = tile_row / HALLWAY_CELL as i32;

    for dr in -REVEAL_RADIUS..=REVEAL_RADIUS {
        for dc in -REVEAL_RADIUS..=REVEAL_RADIUS {
            visited.0.insert((coarse_col + dc, coarse_row + dr));
        }
    }
}

fn update_minimap(
    rooms: Res<RoomVec>,
    player_q: Query<&Transform, With<Player>>,
    grid: Res<MapGridMeta>,
    lvlstate: Res<LevelState>,
    visited: Res<VisitedCells>,
    mut room_nodes: Query<(&MinimapRoomNode, &mut BackgroundColor), Without<MinimapHallwayNode>>,
    mut hallway_nodes: Query<(&MinimapHallwayNode, &mut BackgroundColor, &mut Visibility)>,
    mut player_dot: Query<&mut Node, With<MinimapPlayerDot>>,
) {
    let map_px_w = grid.cols as f32 * TILE_SIZE;
    let map_px_h = grid.rows as f32 * TILE_SIZE;
    let world_min_x = -map_px_w * 0.5;
    let world_max_y = map_px_h * 0.5;

    let current_room = match *lvlstate {
        LevelState::InRoom(i, _, _) | LevelState::EnteredRoom(i) => Some(i),
        LevelState::NotRoom => None,
    };

    // Update room nodes
    for (node, mut bg) in &mut room_nodes {
        let Some(room) = rooms.0.get(node.room_index) else { continue };
        bg.0 = if current_room == Some(node.room_index) {
            Color::srgba(1.0, 0.9, 0.0, 1.0)
        } else if !room.visited {
            Color::srgba(0.04, 0.04, 0.12, 1.0)
        } else if room.cleared {
            Color::srgba(0.15, 0.65, 0.15, 0.9)
        } else {
            Color::srgba(0.35, 0.35, 0.55, 0.9)
        };
    }

    // Update hallway nodes
    for (node, mut bg, mut vis) in &mut hallway_nodes {
        if visited.0.contains(&(node.cell_col, node.cell_row)) {
            *vis = Visibility::Inherited; // inherits from parent MinimapRoot, hides when map closes
            bg.0 = Color::srgba(0.30, 0.30, 0.45, 0.85);
        }
    }

    // Update player dot
    let Ok(player_tf) = player_q.single() else { return };
    let px = player_tf.translation.x;
    let py = player_tf.translation.y;

    let dot_x = ((px - world_min_x) / map_px_w * MINIMAP_W - 4.0).clamp(0.0, MINIMAP_W - 8.0);
    let dot_y = ((world_max_y - py) / map_px_h * MINIMAP_H - 4.0).clamp(0.0, MINIMAP_H - 8.0);

    if let Ok(mut node) = player_dot.single_mut() {
        node.left = Val::Px(dot_x);
        node.top = Val::Px(dot_y);
    }
}
