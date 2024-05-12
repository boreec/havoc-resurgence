mod cellular_automaton;
mod constants;
mod movement;
mod noise;
mod tile;

use cellular_automaton::*;
pub use constants::*;
pub use movement::*;
use noise::*;
use tile::*;

use crate::prelude::*;
use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InitializingMap), initialize_map)
            .add_systems(
                OnEnter(GameState::PlayerTurn),
                check_if_player_exit_map.run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnEnter(GameState::CleanupMap), cleanup_map)
            .add_systems(OnEnter(GameState::EnemyTurn), move_randomly);
    }
}

/// Move mob actors to a random reachable position.
pub fn move_randomly(
    mut query_actors: Query<(&mut MapPosition, &MapNumber, &ActorType)>,
    query_map: Query<(&Map, &MapNumber)>,
    current_map_number: Res<CurrentMapNumber>,
) {
    let pos_occupied: Vec<MapPosition> = query_actors
        .iter_mut()
        .filter(|(_, m_n, _)| m_n.0 == current_map_number.0)
        .map(|(p, _, _)| *p)
        .collect();

    let (map, _) = query_map
        .iter()
        .filter(|(_, m_n)| m_n.0 == current_map_number.0)
        .last()
        .expect("no map found");

    for (mut pos_mob, mob_map_number, actor) in query_actors.iter_mut() {
        if mob_map_number.0 == current_map_number.0 && !actor.is_player() {
            let pos_reachable = enumerate_reachable_positions(
                &pos_mob.clone(),
                map,
                &pos_occupied,
            );

            if !pos_reachable.is_empty() {
                let pos_random = pos_reachable
                    [rand::thread_rng().gen_range(0..pos_reachable.len())];
                pos_mob.x = pos_random.x;
                pos_mob.y = pos_random.y;
            }
        }
    }
}

/// Removes all entities (`Map`, `Tile`, etc) related to the current map.
pub fn cleanup_map(
    mut commands: Commands,
    query: Query<(Entity, &MapNumber), Or<(With<Map>, With<Tile>)>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut current_map_number: ResMut<CurrentMapNumber>,
) {
    for (entity, map_number) in &query {
        if map_number.0 == current_map_number.0 {
            commands.entity(entity).despawn();
        }
    }
    next_game_state.set(GameState::InitializingMap);
    current_map_number.0 += 1;
}

/// Checks if a player is on an exit tile. In that case, the game state is
/// switched to `GameState::CleanupMap`.
pub fn check_if_player_exit_map(
    query_map: Query<(&Map, &MapNumber)>,
    query_actors: Query<(&MapPosition, &MapNumber, &ActorType)>,
    mut next_game_state: ResMut<NextState<GameState>>,
    current_map_number: Res<CurrentMapNumber>,
) {
    let (map, _) = query_map
        .iter()
        .filter(|(_, m_n)| m_n.0 == current_map_number.0)
        .last()
        .expect("no map found");

    let (player_position, _, _) = query_actors
        .iter()
        .filter(|(_, m_n, actor)| {
            m_n.0 == current_map_number.0 && actor.is_player()
        })
        .last()
        .expect("player position not found");

    if map.exits.contains(player_position) {
        next_game_state.set(GameState::CleanupActors);
    }
}

/// Bundle for creating an entity representing a map.
#[derive(Bundle)]
pub struct MapBundle {
    /// Marker component a map entity.
    map: Map,
    /// The number to be associated with the map.
    map_number: MapNumber,
}

/// Represents a number to identify a map.
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
    current_map_number: Res<CurrentMapNumber>,
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
        commands.spawn(TileBundle::new(
            tile_position,
            current_map_number.0,
            &tileset,
            *tile,
        ));
    }

    commands.spawn(MapBundle {
        map: m,
        map_number: MapNumber(current_map_number.0),
    });

    game_next_state.set(GameState::InitializingActors);
}

impl Map {
    /// Returns random positions where an actor can spawn, meaning a position
    /// with no other actors and that can be walkable.
    pub fn generate_random_positions(
        &self,
        quantity: usize,
        occupied_positions: &[MapPosition],
    ) -> Result<Vec<MapPosition>, Box<dyn std::error::Error>> {
        let mut spawnable_positions: Vec<_> = self
            .tiles
            .iter()
            .enumerate()
            .filter(|(index, tile)| {
                tile.is_walkable()
                    && !occupied_positions.contains(&MapPosition {
                        x: index % self.width,
                        y: index / self.width,
                    })
            })
            .map(|(index, _)| MapPosition {
                x: index % self.width,
                y: index / self.width,
            })
            .collect();

        if spawnable_positions.is_empty() {
            return Err("no spawnable positions".into());
        }

        let mut rng = rand::thread_rng();
        spawnable_positions.shuffle(&mut rng);

        Ok(spawnable_positions[0..quantity].to_vec())
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

        assert!(
            !spawnable_positions.is_empty(),
            "there are no available positions for the exit tile",
        );

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
#[derive(Clone, Component, Copy, Debug, Eq, PartialEq)]
pub struct MapPosition {
    pub x: usize,
    pub y: usize,
}

impl MapPosition {
    pub const fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn as_sprite_coordinates(&self) -> (f32, f32) {
        (
            (self.x as f32).mul_add(SPRITE_TILE_WIDTH, SPRITE_TILE_WIDTH / 2.0),
            (-1f32 * self.y as f32)
                .mul_add(SPRITE_TILE_HEIGHT, -(SPRITE_TILE_HEIGHT / 2.0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_spawning_position_success() {
        let map1x1 = Map {
            width: 1,
            height: 1,
            tiles: vec![TileType::Grass],
            exits: vec![],
        };

        let spawn = map1x1.generate_random_positions(1, &vec![]);

        assert!(spawn.is_ok());
        assert_eq!(vec![MapPosition::new(0, 0)], spawn.unwrap());
    }

    #[test]
    fn test_generate_random_spawning_position_failure() {
        let mut map1x1 = Map {
            width: 1,
            height: 1,
            tiles: vec![TileType::GrassWithStone],
            exits: vec![],
        };

        let spawn = map1x1.generate_random_positions(1, &vec![]);
        assert!(spawn.is_err());

        map1x1.tiles = vec![TileType::Grass];

        let spawn = map1x1.generate_random_positions(1, &vec![]);
        assert!(spawn.is_ok());

        let spawn = map1x1
            .generate_random_positions(1, &vec![MapPosition { x: 0, y: 0 }]);
        assert!(spawn.is_err());
    }
}
