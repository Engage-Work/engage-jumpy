//! Map and navigation mesh implementation.

use super::navigation::{Movement, Navigation};

use super::physics::collisions::{CollisionWorld, TileCollisionKind, TileDynamicCollider};
use crate::prelude::*;

pub fn install(session: &mut Session) {
    session
        .stages
        .add_system_to_stage(CoreStage::First, spawn_map)
        .add_system_to_stage(CoreStage::First, handle_out_of_bounds_players);
}

/// Resource containing the map metadata for this game session.
#[derive(Clone, HasSchema, Deref, DerefMut, Default)]
pub struct LoadedMap(pub Arc<MapMeta>);

/// Resource indicating whether the map has been spawned.
#[derive(Clone, HasSchema, Default, Deref, DerefMut)]
pub struct MapSpawned(pub bool);

/// The Z depth of the deepest map layer.
pub const MAP_LAYERS_MIN_DEPTH: f32 = -900.0;
/// The Z depth in between each map layer.
pub const MAP_LAYERS_GAP_DEPTH: f32 = 10.0;

/// Helper for getting the z-depth of the map layer with the given index.
pub fn z_depth_for_map_layer(layer_idx: u32) -> f32 {
    // We start map layers at -900 and for ever layer we place a gap of 10 units in between
    MAP_LAYERS_MIN_DEPTH + layer_idx as f32 * MAP_LAYERS_GAP_DEPTH
}

/// Resource containing essential the map metadata for the map once spawned. This allows the
/// complete map metadata to be re-constructed from the world after the map has been spawned and
/// potentially modified.
#[derive(HasSchema, Clone)]
pub struct SpawnedMapMeta {
    pub name: Ustr,
    pub background: Arc<BackgroundMeta>,
    pub background_color: Color,
    pub grid_size: UVec2,
    pub tile_size: Vec2,
    pub layer_names: Arc<[Ustr]>,
}

impl Default for SpawnedMapMeta {
    fn default() -> Self {
        Self {
            name: "".into(),
            background: default(),
            background_color: default(),
            grid_size: default(),
            tile_size: default(),
            layer_names: Arc::new([]),
        }
    }
}

/// Component containing the map layer that an entity is associated to.
///
/// This is used when exporting the world to `MapMeta` to decide which layer to put an element or
/// tile layer in.
#[derive(HasSchema, Clone, Copy, Default)]
pub struct SpawnedMapLayerMeta {
    /// The layer index of the layer that the element belongs to in the map.
    pub layer_idx: u32,
}

/// Shared platform graph; shipped player skins have the same size and movement stats.
#[derive(Clone, HasSchema, Default)]
pub struct NavGraph(#[schema(opaque)] pub Arc<Navigation>);

fn spawn_map(
    mut commands: Commands,
    mut entities: ResMutInit<Entities>,
    mut clear_color: ResMutInit<ClearColor>,
    assets: Res<AssetServer>,
    game_meta: Root<GameMeta>,
    map: Res<LoadedMap>,
    mut map_spawned: ResMutInit<MapSpawned>,
    mut tiles: CompMut<Tile>,
    mut tile_layers: CompMut<TileLayer>,
    mut transforms: CompMut<Transform>,
    mut element_handles: CompMut<ElementHandle>,
    mut tile_collisions: CompMut<TileCollisionKind>,
    mut tile_dynamic_colliders: CompMut<TileDynamicCollider>,
    mut parallax_bg_sprites: CompMut<ParallaxBackgroundSprite>,
    mut sprites: CompMut<Sprite>,
    mut nav_graph: ResMutInit<NavGraph>,
    mut cameras: CompMut<Camera>,
    mut camera_shakes: CompMut<CameraShake>,
    mut camera_states: CompMut<CameraState>,
    mut spawned_map_layer_metas: CompMut<SpawnedMapLayerMeta>,
    mut spawned_map_meta: ResMutInit<SpawnedMapMeta>,
) {
    if map_spawned.0 {
        return;
    }

    // Fill in the spawned map metadata
    *spawned_map_meta = SpawnedMapMeta {
        name: map.name,
        background: Arc::new(map.background.clone()),
        background_color: map.background_color,
        grid_size: map.grid_size,
        tile_size: map.tile_size,
        layer_names: map.layers.iter().map(|x| x.id).collect(),
    };

    // Spawn the camera
    {
        let ent = entities.create();
        camera_shakes.insert(
            ent,
            CameraShake {
                center: (map.tile_size * (map.grid_size / 2).as_vec2()).extend(0.0),
                ..CameraShake::new(2.0, vec2(20.0, 20.0), 1.0, 1.0)
            },
        );
        cameras.insert(ent, default());
        transforms.insert(ent, default());
        camera_states.insert(ent, default());
    }

    map_spawned.0 = true;
    **clear_color = map.background_color;

    // Load the navigation graph
    nav_graph.0 = Arc::new(Navigation::build(
        &map,
        Movement::from_player(
            &assets.get(game_meta.core.players[0]),
            game_meta.core.physics.terminal_velocity,
        ),
    ));

    // Spawn parallax backgrounds
    for layer in &map.background.layers {
        for i in -1..=1 {
            let ent = entities.create();
            sprites.insert(
                ent,
                Sprite {
                    image: layer.image,
                    ..default()
                },
            );
            transforms.insert(ent, default());
            parallax_bg_sprites.insert(
                ent,
                ParallaxBackgroundSprite {
                    idx: i,
                    meta: layer.clone(),
                },
            );
        }
    }

    // Load tiles
    for (layer_idx, layer) in map.layers.iter().enumerate() {
        let layer_idx = layer_idx as u32;
        let layer_z = z_depth_for_map_layer(layer_idx);

        if let Set(tilemap) = layer.tilemap {
            let mut tile_layer = TileLayer::new(map.grid_size, map.tile_size, tilemap);

            for tile_meta in &layer.tiles {
                let tile_ent = entities.create();
                tile_layer.set(tile_meta.pos, Some(tile_ent));
                tiles.insert(
                    tile_ent,
                    Tile {
                        idx: tile_meta.idx,
                        ..default()
                    },
                );
                // TODO: Due to a bug in the way that collisions are handled in the
                // `physics/collisions.rs` file, having an empty collision isn't equivalent to not
                // having a collision component.
                //
                // We should fix this so that we can insert an empty tile collision kind and have that
                // work properly. For now, though, just not adding the tile collider component behaves
                // properly.
                if tile_meta.collision != TileCollisionKind::Empty {
                    tile_collisions.insert(tile_ent, tile_meta.collision);
                }

                let atlas = assets.get(tilemap);
                let optional_tile_colliders = &atlas.tile_collision;
                // If metadata has valid dynamics collider, create component for it.
                if let Some(extra_collider) =
                    optional_tile_colliders.get(&format!("{}", tile_meta.idx))
                {
                    if extra_collider.has_area() {
                        // clamp collider between 0.0, 1.0
                        let extra_collider = extra_collider.clamped_values();

                        // scale min/max between 0 and tile size.
                        let tile_size = spawned_map_meta.tile_size;
                        let min = extra_collider.min * tile_size;
                        let max = extra_collider.max * tile_size;

                        // compute offset from center of tile
                        let collider_center = min + (max - min) * 0.5;
                        let offset =
                            collider_center - Vec2::new(tile_size.x * 0.5, tile_size.y * 0.5);

                        let extents = max - min;
                        let shape = ColliderShape::Rectangle { size: extents };

                        tile_dynamic_colliders
                            .insert(tile_ent, TileDynamicCollider { shape, offset });
                    }
                }
            }
            let layer_ent = entities.create();
            spawned_map_layer_metas.insert(layer_ent, SpawnedMapLayerMeta { layer_idx });
            tile_layers.insert(layer_ent, tile_layer);
            transforms.insert(
                layer_ent,
                Transform::from_translation(Vec3::new(0.0, 0.0, layer_z)),
            );
        }

        // Spawn the elements
        for element_meta in &layer.elements {
            let element_ent = entities.create();

            spawned_map_layer_metas.insert(element_ent, SpawnedMapLayerMeta { layer_idx });
            transforms.insert(
                element_ent,
                Transform::from_translation(element_meta.pos.extend(layer_z)),
            );
            element_handles.insert(element_ent, ElementHandle(element_meta.element));
        }
    }

    // Update collision world with map tiles
    commands.add(|mut collision_world: CollisionWorld| {
        collision_world.update_tiles();
    });
}

fn handle_out_of_bounds_players(
    entities: Res<Entities>,
    mut commands: Commands,
    transforms: CompMut<Transform>,
    player_indexes: Comp<PlayerIdx>,
    map: Res<LoadedMap>,
) {
    for (player_ent, (_player_idx, transform)) in entities.iter_with((&player_indexes, &transforms))
    {
        if map.is_out_of_bounds(&transform.translation) {
            commands.add(PlayerCommand::kill(player_ent, None));
        }
    }
}
