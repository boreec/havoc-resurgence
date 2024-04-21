mod constants;

use constants::*;

use crate::prelude::*;
use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InitializingMap), initialize_map)
            .add_systems(
                Update,
                (check_if_player_exit_map)
                    .run_if(in_state(GameState::PlayerTurn)),
            )
            .add_systems(OnEnter(GameState::CleanupMap), cleanup_map);
    }
}

/// Removes all entities (`Map`, `Tile`, etc) related to the current map.
pub fn cleanup_map(
    mut commands: Commands,
    query_map: Query<(Entity, &MapNumber), With<Map>>,
    query_tile: Query<(Entity, &MapNumber), With<Tile>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    current_map_number: Res<CurrentMapNumber>,
) {
    for (entity, map_number) in &query_map {
        if map_number.0 == current_map_number.0 {
            commands.entity(entity).despawn();
        }
    }
    for (entity, map_number) in &query_tile {
        if map_number.0 == current_map_number.0 {
            commands.entity(entity).despawn();
        }
    }
    next_game_state.set(GameState::CleanupActors);
}

/// Checks if a player is on an exit tile. In that case, the game state is
/// switched to `GameState::CleanupMap`.
pub fn check_if_player_exit_map(
    query_map: Query<&Map>,
    query_player: Query<(&Player, &MapPosition)>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    let map = query_map.single();
    let (_, player_position) = query_player.single();
    for exit_position in &map.exits {
        if player_position == exit_position {
            next_game_state.set(GameState::CleanupMap);
        }
    }
}

#[derive(Bundle)]
pub struct MapBundle {
    map: Map,
    map_number: MapNumber,
}

#[derive(Component)]
pub struct MapNumber(pub usize);

/// Represents the environment where the actors interact together. A map is
/// made of tiles which has different properties for the actors.
#[derive(Component)]
pub struct Map {
    /// The map's width.
    pub width: usize,
    /// The map's height.
    pub height: usize,
    /// All tiles for the map, the vector index corresponds to the tile
    /// coordinates.
    pub tiles: Vec<TileType>,
    /// The exits positions for the map.
    pub exits: Vec<MapPosition>,
}

/// Initialize a map by spawning tile entities depending on the map dimensions,
/// the tile placement algorithm, etc.
/// Lastly, the map entity is spawned.
fn initialize_map(
    mut commands: Commands,
    mut game_next_state: ResMut<NextState<GameState>>,
    tileset: Res<TilesetTerrain>,
) {
    let m = if rand::thread_rng().gen_bool(0.5) {
        Map::from((PerlinNoise::new(), MAP_WIDTH, MAP_HEIGHT))
    } else {
        let mut ca = CellularAutomaton::new(MAP_WIDTH, MAP_HEIGHT, 0.5);
        for _ in 0..50 {
            ca.transition();
        }
        ca.smooth();
        Map::from(ca)
    };

    for (i, tile) in m.tiles.iter().enumerate() {
        let tile_position = MapPosition {
            x: i % m.width,
            y: i / m.width,
        };
        let (sprite_x, sprite_y) = calculate_sprite_position(&tile_position);
        commands.spawn(TileBundle {
            tile: Tile,
            r#type: tile.clone(),
            sprite: SpriteSheetBundle {
                transform: Transform::from_xyz(
                    sprite_x,
                    sprite_y,
                    Z_INDEX_TILE,
                ),
                sprite: Sprite::default(),
                texture: tileset.1.clone(),
                atlas: TextureAtlas {
                    layout: tileset.0.clone(),
                    index: TileType::to_sprite_idx(tile),
                },
                ..Default::default()
            },
            map_number: MapNumber(0),
            map_position: tile_position,
        });
    }

    commands.spawn(MapBundle {
        map: m,
        map_number: MapNumber(0),
    });

    game_next_state.set(GameState::InitializingActors);
}

impl Map {
    /// Returns a randown position where an actor can spawn, i.e. there's no
    /// obstacle such as a rock or a tree on that position.
    pub fn generate_random_spawning_position(&self) -> MapPosition {
        let spawnable_positions: Vec<_> = self
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, tile)| tile.is_walkable())
            .map(|(index, _)| index)
            .collect();

        if spawnable_positions.is_empty() {
            panic!("There are no spawnable positions");
        }

        let mut rng = rand::thread_rng();
        let index = *spawnable_positions.choose(&mut rng).unwrap();

        MapPosition::new(index % self.width, index / self.height)
    }

    /// Adds an exit tile on the right side of the map. The position is
    /// selected randomly.
    pub fn add_exit_tile(&mut self) {
        let spawnable_positions: Vec<_> = self
            .tiles
            .iter()
            .enumerate()
            .filter(|(index, tile)| {
                tile.is_walkable() && *index % self.width == self.width - 1
            })
            .map(|(index, _)| index)
            .collect();

        if spawnable_positions.is_empty() {
            panic!("There are no spawnable positions");
        }

        let mut rng = rand::thread_rng();
        let index = *spawnable_positions.choose(&mut rng).unwrap();

        self.tiles[index] = TileType::LevelExit;

        let exit_position = MapPosition {
            x: index % self.width,
            y: index / self.width,
        };

        self.exits.push(exit_position);
    }
}

impl From<CellularAutomaton> for Map {
    /// Constructs a `Map` using a cellular automaton.
    ///
    /// # Arguments
    ///
    /// - `ca`: A `CellularAutomaton` initialized struct.
    ///
    /// # Returns
    ///
    /// A `Map` where the tiles are determined by the cellular automaton state.    
    fn from(ca: CellularAutomaton) -> Self {
        let mut map = Self {
            width: ca.width,
            height: ca.height,
            tiles: ca
                .cells
                .iter()
                .map(|cellular_state| match cellular_state {
                    CellularState::Alive => TileType::GrassWithStone,
                    CellularState::Dead => TileType::Grass,
                })
                .collect(),
            exits: vec![],
        };
        map.add_exit_tile();
        map
    }
}

impl From<(PerlinNoise, usize, usize)> for Map {
    /// Constructs a `Map` using Perlin noise.
    ///
    /// # Arguments
    ///
    /// - `tuple`: A tuple with three parameters representing a `PerlinNoise` struct,
    ///            the width, and the height of the map.
    ///
    /// # Returns
    ///
    /// A `Map` where the tiles are determined by Perlin noise.
    fn from(tuple: (PerlinNoise, usize, usize)) -> Self {
        let mut cells: Vec<TileType> = Vec::new();
        for i in 0..tuple.1 {
            for j in 0..tuple.2 {
                let x_scaled = i as f64 * PERLIN_NOISE_SCALE;
                let y_scaled = j as f64 * PERLIN_NOISE_SCALE;
                let noise_value = tuple.0.perlin_noise(x_scaled, y_scaled);
                if noise_value > 2.2 {
                    cells.push(TileType::GrassWithFlower);
                } else if noise_value > -0.25 {
                    cells.push(TileType::Grass);
                } else {
                    cells.push(TileType::GrassWithStone);
                }
            }
        }

        let mut map = Self {
            width: tuple.1,
            height: tuple.2,
            tiles: cells,
            exits: vec![],
        };

        map.add_exit_tile();
        map
    }
}

/// Represents a position in a `Map`.
#[derive(Component, Debug, Eq, PartialEq)]
pub struct MapPosition {
    pub x: usize,
    pub y: usize,
}

impl MapPosition {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}
