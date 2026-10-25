//! Platform routes verified against the same Rapier shape casts used by player physics.
//!
//! Surface/trajectory approach: https://github.com/SnoringCatGames/surfacer
//! Implementation is specific to Jumpy's fixed-step movement and collision rules.
use crate::prelude::*;
use petgraph::graphmap::DiGraphMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NavNode(pub IVec2);
impl Ord for NavNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0.x, self.0.y).cmp(&(other.0.x, other.0.y))
    }
}
impl PartialOrd for NavNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveKind {
    Walk,
    Jump,
    Drop,
    Spring,
}

#[derive(Clone, Copy, Debug)]
pub struct NavEdge {
    pub kind: MoveKind,
    pub frames: u32,
}

#[derive(Clone, Debug)]
pub struct Movement {
    pub size: Vec2,
    pub stats: PlayerStatsMeta,
    pub gravity: f32,
    pub terminal_velocity: f32,
}

impl Movement {
    pub fn from_player(player: &PlayerMeta, terminal_velocity: f32) -> Self {
        Self {
            size: player.body_size,
            stats: player.stats.clone(),
            gravity: player.gravity,
            terminal_velocity,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub pos: Vec2,
    pub velocity: Vec2,
    pub grounded: bool,
}

#[derive(Clone, Default)]
pub struct Navigation {
    pub graph: DiGraphMap<NavNode, NavEdge>,
    pub positions: HashMap<NavNode, Vec2>,
    pub movement: Option<Movement>,
    springs: HashMap<NavNode, f32>,
    bodies: rapier::RigidBodySet,
    colliders: rapier::ColliderSet,
    query: rapier::QueryPipeline,
}

impl Navigation {
    pub fn build(map: &MapMeta, movement: Movement) -> Self {
        let mut nav = Self {
            movement: Some(movement),
            ..default()
        };
        let mut tiles = HashMap::default();
        for layer in &map.layers {
            for tile in &layer.tiles {
                if tile.collision != TileCollisionKind::Empty {
                    let kind = tiles.entry(tile.pos).or_insert(tile.collision);
                    if tile.collision == TileCollisionKind::Solid {
                        *kind = tile.collision;
                    }
                }
            }
        }
        let mut ordered = tiles.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(pos, _)| (pos.x, pos.y));
        for (pos, kind) in &ordered {
            let center = pos.as_vec2() * map.tile_size + map.tile_size / 2.0;
            nav.colliders.insert(
                rapier::ColliderBuilder::cuboid(map.tile_size.x / 2.0, map.tile_size.y / 2.0)
                    .translation(center.into())
                    .user_data(if **kind == TileCollisionKind::Solid {
                        1
                    } else {
                        2
                    })
                    .build(),
            );
        }
        nav.query.update(&nav.bodies, &nav.colliders);
        let size = nav.movement.as_ref().unwrap().size;
        let shape = rapier::Cuboid::new((size / 2.0).into());
        for (tile, _) in ordered {
            let node = NavNode(tile.as_ivec2() + IVec2::Y);
            let center = vec2(
                (tile.x as f32 + 0.5) * map.tile_size.x,
                (tile.y as f32 + 1.0) * map.tile_size.y + size.y / 2.0 + 0.1,
            );
            // Rapier treats a shape tangent to a wall as a blocked cast. Real movement leaves a
            // 0.1px skin, so launch positions must have clearance too, rather than touch the wall.
            let position = [0.0, -0.2, 0.2]
                .into_iter()
                .map(|offset| center + vec2(offset, 0.0))
                .find(|pos| {
                    nav.query
                        .intersection_with_shape(
                            &nav.bodies,
                            &nav.colliders,
                            &rapier::Isometry::translation(pos.x, pos.y),
                            &shape,
                            rapier::QueryFilter::new()
                                .predicate(&|_, collider| collider.user_data == 1),
                        )
                        .is_none()
                });
            if let Some(pos) = position {
                if pos.y + size.y / 2.0 < map.grid_size.y as f32 * map.tile_size.y {
                    nav.graph.add_node(node);
                    nav.positions.insert(node, pos);
                }
            }
        }
        let mut nodes = nav.graph.nodes().collect::<Vec<_>>();
        nodes.sort();
        for &from in &nodes {
            for &to in &nodes {
                if from == to {
                    continue;
                }
                let delta = nav.positions[&to] - nav.positions[&from];
                let movement = nav.movement.as_ref().unwrap();
                let max_rise = movement.stats.jump_speed.powi(2) / (2.0 * movement.gravity);
                let flight = (movement.stats.jump_speed
                    + (movement.stats.jump_speed.powi(2) - 2.0 * movement.gravity * delta.y)
                        .max(0.0)
                        .sqrt())
                    / movement.gravity;
                if delta.y > max_rise
                    || delta.x.abs() > movement.stats.air_speed * flight + map.tile_size.x
                {
                    continue;
                }
                let kinds: &[MoveKind] = if delta.y > 0.5 {
                    &[MoveKind::Jump]
                } else {
                    &[MoveKind::Walk, MoveKind::Jump, MoveKind::Drop]
                };
                for &kind in kinds {
                    if kind == MoveKind::Drop
                        && tiles.get(&(from.0 - IVec2::Y).as_uvec2())
                            != Some(&TileCollisionKind::JumpThrough)
                    {
                        continue;
                    }
                    if let Some(frames) = nav.simulate(from, to, kind) {
                        nav.graph.add_edge(from, to, NavEdge { kind, frames });
                        break;
                    }
                }
            }
        }
        nav
    }

    pub fn add_spring(&mut self, position: Vec2, velocity: f32) {
        let Some(from) = self.nearest(position) else {
            return;
        };
        self.springs.insert(from, velocity);
        let nodes = self.graph.nodes().collect::<Vec<_>>();
        for to in nodes {
            if to != from {
                if let Some(frames) = self.simulate(from, to, MoveKind::Spring) {
                    self.graph.add_edge(
                        from,
                        to,
                        NavEdge {
                            kind: MoveKind::Spring,
                            frames,
                        },
                    );
                }
            }
        }
    }

    pub fn steering(&self, motion: Motion, target: Vec2) -> f32 {
        let stats = &self.movement.as_ref().unwrap().stats;
        let desired = ((target.x - motion.pos.x) * 12.0).clamp(-stats.air_speed, stats.air_speed);
        if motion.grounded {
            (desired / stats.walk_speed).clamp(-1.0, 1.0)
        } else {
            ((desired - motion.velocity.x) / stats.accel_air_speed).clamp(-1.0, 1.0)
        }
    }

    pub fn control(
        &self,
        motion: Motion,
        destination: Vec2,
        kind: MoveKind,
        frame: u32,
    ) -> PlayerControl {
        let dropping = kind == MoveKind::Drop && frame < 15;
        PlayerControl {
            move_direction: vec2(
                if dropping && motion.grounded {
                    0.0
                } else {
                    self.steering(motion, destination)
                },
                if dropping { -1.0 } else { 0.0 },
            ),
            jump_pressed: dropping
                || kind == MoveKind::Jump && (frame == 0 || motion.velocity.y > 0.0),
            jump_just_pressed: frame == 0 && matches!(kind, MoveKind::Jump | MoveKind::Drop),
            ..default()
        }
    }

    fn cast(&self, pos: Vec2, delta: Vec2, drop: bool) -> Option<f32> {
        let size = self.movement.as_ref().unwrap().size;
        self.query
            .cast_shape(
                &self.bodies,
                &self.colliders,
                &rapier::Isometry::translation(pos.x, pos.y),
                &delta.into(),
                &rapier::Cuboid::new((size / 2.0).into()),
                1.0,
                true,
                rapier::QueryFilter::new().predicate(&|_, collider| {
                    if collider.user_data == 1 {
                        return true;
                    }
                    // One-way platforms only block a descending fish that starts above their top.
                    let top = collider.compute_aabb().maxs.y;
                    !drop && delta.y < 0.0 && pos.y - size.y / 2.0 >= top - 0.2
                }),
            )
            .map(|(_, hit)| hit.toi)
    }

    pub fn apply_control(&self, motion: &mut Motion, control: PlayerControl) {
        let movement = self.movement.as_ref().unwrap();
        let stats = &movement.stats;
        let x = control.move_direction.x;
        if motion.grounded {
            if control.jump_just_pressed && control.move_direction.y > -0.5 {
                motion.velocity.y = stats.jump_speed;
            }
            motion.velocity.x += stats.accel_walk_speed * x;
            if x > 0.0 {
                motion.velocity.x = motion.velocity.x.min(stats.walk_speed * x);
            } else if x < 0.0 {
                motion.velocity.x = motion.velocity.x.max(stats.walk_speed * x);
            }
        } else {
            if control.jump_pressed {
                motion.velocity.y = motion.velocity.y.max(-stats.slow_fall_speed);
            }
            motion.velocity.x += stats.accel_air_speed * x;
            motion.velocity.x = motion.velocity.x.clamp(-stats.air_speed, stats.air_speed);
        }
        if x == 0.0 {
            motion.velocity.x =
                motion.velocity.x.signum() * (motion.velocity.x.abs() - stats.slowdown).max(0.0);
        }
    }

    pub fn step(&self, motion: &mut Motion, control: PlayerControl) {
        self.apply_control(motion, control);
        let drop = control.move_direction.y < -0.5 && control.jump_pressed;
        for delta in [
            vec2(0.0, motion.velocity.y / FPS),
            vec2(motion.velocity.x / FPS, 0.0),
        ] {
            if delta == Vec2::ZERO {
                continue;
            }
            if let Some(fraction) = self.cast(motion.pos, delta, drop) {
                motion.pos += delta * fraction - delta.normalize() * 0.1;
                if delta.x != 0.0 {
                    motion.velocity.x = 0.0;
                } else {
                    motion.velocity.y = 0.0;
                }
            } else {
                motion.pos += delta;
            }
        }
        motion.grounded = self.cast(motion.pos, vec2(0.0, -0.2), drop).is_some();
        self.finish_step(motion);
    }

    pub fn finish_step(&self, motion: &mut Motion) {
        let movement = self.movement.as_ref().unwrap();
        if motion.grounded && motion.velocity.y <= movement.gravity / FPS {
            motion.velocity.y = 0.0;
        }
        if !motion.grounded {
            motion.velocity.y =
                (motion.velocity.y - movement.gravity / FPS).max(-movement.terminal_velocity);
        }
    }

    fn simulate(&self, from: NavNode, to: NavNode, kind: MoveKind) -> Option<u32> {
        let destination = self.positions[&to];
        let mut motion = Motion {
            pos: self.positions[&from],
            velocity: Vec2::ZERO,
            grounded: true,
        };
        if kind == MoveKind::Spring {
            motion.velocity.y = *self.springs.get(&from)?;
            motion.grounded = false;
        }
        let mut left_ground = false;
        for frame in 0..120 {
            let control = self.control(motion, destination, kind, frame);
            self.step(&mut motion, control);
            left_ground |= !motion.grounded;
            if motion.grounded && motion.pos.distance(destination) < 5.0 {
                return Some(frame + 1);
            }
            if kind != MoveKind::Walk && left_ground && motion.grounded {
                return None;
            }
            if frame > 5 && !left_ground && kind == MoveKind::Jump {
                return None;
            }
        }
        None
    }

    pub fn nearest(&self, position: Vec2) -> Option<NavNode> {
        self.positions
            .iter()
            .min_by(|(a, pa), (b, pb)| {
                pa.distance_squared(position)
                    .total_cmp(&pb.distance_squared(position))
                    .then_with(|| a.cmp(b))
            })
            .map(|(&node, _)| node)
    }

    pub fn route(
        &self,
        from: NavNode,
        target: Vec2,
        blocked: &[(NavNode, NavNode)],
    ) -> Option<Vec<NavNode>> {
        let cost = |a, b, edge: &NavEdge| {
            if blocked.contains(&(a, b)) {
                10000.0
            } else {
                edge.frames as f32
                    + if edge.kind == MoveKind::Walk {
                        0.0
                    } else {
                        4.0
                    }
            }
        };
        let reachable =
            petgraph::algo::dijkstra(&self.graph, from, None, |(a, b, e)| cost(a, b, e));
        let goal = reachable
            .iter()
            .filter(|(_, distance)| **distance < 10000.0)
            .min_by(|(a, ca), (b, cb)| {
                let score = |node: &NavNode, c: f32| {
                    self.positions[node].distance_squared(target) + c * 0.5
                };
                score(a, **ca)
                    .total_cmp(&score(b, **cb))
                    .then_with(|| a.cmp(b))
            })
            .map(|(&node, _)| node)?;
        petgraph::algo::astar(
            &self.graph,
            from,
            |node| node == goal,
            |(a, b, e)| cost(a, b, e),
            |_| 0.0,
        )
        .map(|(_, path)| path)
    }
}

#[derive(Clone, Debug)]
struct ActiveMove {
    from: NavNode,
    to: NavNode,
    edge: NavEdge,
    frame: u32,
    launched: bool,
    airborne: bool,
    elapsed: f32,
}

#[derive(Clone, Debug, Default)]
pub struct NavAgent {
    active: Option<ActiveMove>,
    blocked: Vec<(NavNode, NavNode, f32)>,
    replan_in: f32,
}

impl NavAgent {
    pub fn is_busy(&self) -> bool {
        self.active.is_some()
    }
    pub fn destination(&self, nav: &Navigation) -> Option<Vec2> {
        self.active.as_ref().map(|active| nav.positions[&active.to])
    }

    pub fn control(
        &mut self,
        nav: &Navigation,
        motion: Motion,
        target: Vec2,
        dt: f32,
    ) -> PlayerControl {
        if nav.movement.is_none() {
            return default();
        }
        self.replan_in -= dt;
        for (_, _, remaining) in &mut self.blocked {
            *remaining -= dt;
        }
        self.blocked.retain(|(_, _, remaining)| *remaining > 0.0);
        if let Some(active) = &mut self.active {
            active.elapsed += dt;
            active.airborne |= !motion.grounded;
            let arrived = motion.grounded && motion.pos.distance(nav.positions[&active.to]) < 8.0;
            let failed = active.elapsed > active.edge.frames as f32 / FPS * 2.0 + 0.75
                || active.launched && active.airborne && motion.grounded && !arrived;
            if arrived || failed {
                if failed {
                    self.blocked.push((active.from, active.to, 3.0));
                }
                self.active = None;
                self.replan_in = 0.0;
            }
        }
        if self.active.is_none() && motion.grounded && self.replan_in <= 0.0 {
            self.replan_in = 0.15;
            if let Some(from) = nav.nearest(motion.pos) {
                let blocked = self
                    .blocked
                    .iter()
                    .map(|(from, to, _)| (*from, *to))
                    .collect::<Vec<_>>();
                if let Some(path) = nav.route(from, target, &blocked) {
                    if let Some(&to) = path.get(1) {
                        self.active = Some(ActiveMove {
                            from,
                            to,
                            edge: nav.graph[(from, to)],
                            frame: 0,
                            launched: false,
                            airborne: false,
                            elapsed: 0.0,
                        });
                    }
                }
            }
        }
        let Some(active) = &mut self.active else {
            // Wait on reachable ground instead of walking into an unreachable wall.
            return default();
        };
        if !active.launched {
            let start = nav.positions[&active.from];
            if active.edge.kind == MoveKind::Drop
                && (motion.pos.x - start.x).abs() < 3.0
                && motion.velocity.x.abs() > 0.1
            {
                return default();
            }
            if active.edge.kind == MoveKind::Walk
                || (motion.pos.x - start.x).abs() < 3.0 && motion.velocity.x.abs() < 80.0
            {
                active.launched = true;
            } else {
                return PlayerControl {
                    move_direction: vec2(nav.steering(motion, start), 0.0),
                    ..default()
                };
            }
        }
        let control = nav.control(
            motion,
            nav.positions[&active.to],
            active.edge.kind,
            active.frame,
        );
        active.frame += 1;
        control
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movement() -> Movement {
        let yaml: serde_yaml::Value = serde_yaml::from_str(include_str!(
            "../../assets/player/skins/fishy/fishy.player.yaml"
        ))
        .unwrap();
        Movement {
            size: vec2(32.0, 48.0),
            stats: serde_yaml::from_value(yaml["stats"].clone()).unwrap(),
            gravity: yaml["gravity"].as_f64().unwrap() as f32,
            terminal_velocity: 1800.0,
        }
    }

    fn arena(tiles: &[(u32, u32, TileCollisionKind)]) -> MapMeta {
        let mut map = MapMeta {
            grid_size: uvec2(20, 20),
            tile_size: vec2(32.0, 32.0),
            ..default()
        };
        let mut layer = MapLayerMeta::default();
        for &(x, y, collision) in tiles {
            layer.tiles.push(MapTileMeta {
                pos: uvec2(x, y),
                collision,
                ..default()
            });
        }
        map.layers.push(layer);
        map
    }

    fn physics_world(map: &MapMeta, start: Vec2) -> (World, Entity) {
        let world = World::default();
        let map = map.clone();
        let player = world.run_system(
            move |mut entities: ResMut<Entities>,
                  mut layers: CompMut<TileLayer>,
                  mut kinds: CompMut<TileCollisionKind>,
                  mut metas: CompMut<SpawnedMapLayerMeta>,
                  mut actors: CompMut<Actor>,
                  mut colliders: CompMut<Collider>,
                  mut transforms: CompMut<Transform>| {
                for (idx, layer) in map.layers.iter().enumerate() {
                    let mut tiles = TileLayer::new(map.grid_size, map.tile_size, Handle::default());
                    for tile in &layer.tiles {
                        let ent = entities.create();
                        tiles.set(tile.pos, Some(ent));
                        if tile.collision != TileCollisionKind::Empty {
                            kinds.insert(ent, tile.collision);
                        }
                    }
                    let ent = entities.create();
                    layers.insert(ent, tiles);
                    metas.insert(
                        ent,
                        SpawnedMapLayerMeta {
                            layer_idx: idx as u32,
                        },
                    );
                }
                let player = entities.create();
                actors.insert(player, Actor);
                colliders.insert(
                    player,
                    Collider {
                        shape: ColliderShape::Rectangle {
                            size: vec2(32.0, 48.0),
                        },
                        ..default()
                    },
                );
                transforms.insert(player, Transform::from_translation(start.extend(0.0)));
                player
            },
            (),
        );
        world.run_system(
            move |mut collision: CollisionWorld| {
                collision.update_tiles();
                collision.handle_teleport(player);
            },
            (),
        );
        {
            let mut context = world.resource_mut::<RapierContext>();
            let context = &mut *context;
            context
                .rigid_body_set
                .propagate_modified_body_positions_to_colliders(&mut context.collider_set);
            context
                .query_pipeline
                .update(&context.rigid_body_set, &context.collider_set);
        }
        (world, player)
    }

    fn actual_step(
        world: &World,
        player: Entity,
        nav: &Navigation,
        motion: &mut Motion,
        control: PlayerControl,
    ) {
        nav.apply_control(motion, control);
        let mut next = *motion;
        let size = nav.movement.as_ref().unwrap().size;
        *motion = world.run_system(
            move |mut collision: CollisionWorld, mut transforms: CompMut<Transform>| {
                if control.move_direction.y < -0.5 && control.jump_pressed {
                    collision.descent(player);
                }
                if collision.move_vertical(&mut transforms, player, next.velocity.y / FPS) {
                    next.velocity.y = 0.0;
                }
                if collision.move_horizontal(&mut transforms, player, next.velocity.x / FPS) {
                    next.velocity.x = 0.0;
                }
                next.pos = transforms.get(player).unwrap().translation.truncate();
                let mut below = *transforms.get(player).unwrap();
                below.translation.y -= 0.1;
                let seen_wood = collision.get_collider(player).seen_wood;
                let tile = collision.tile_collision_filtered(
                    below,
                    ColliderShape::Rectangle { size },
                    |ent| {
                        !seen_wood
                            || collision.tile_collision_kinds.get(ent)
                                != Some(&TileCollisionKind::JumpThrough)
                    },
                );
                next.grounded = tile != TileCollisionKind::Empty
                    && !(tile == TileCollisionKind::JumpThrough
                        && control.move_direction.y < -0.5
                        && control.jump_pressed);
                next
            },
            (),
        );
        nav.finish_step(motion);
    }

    fn assert_reaches(map: &MapMeta, nav: &Navigation, from: NavNode, to: NavNode) {
        let target = nav.positions[&to];
        let mut motion = Motion {
            pos: nav.positions[&from],
            velocity: Vec2::ZERO,
            grounded: true,
        };
        let (world, player) = physics_world(map, motion.pos);
        let mut agent = NavAgent::default();
        for _ in 0..900 {
            let control = agent.control(nav, motion, target, 1.0 / FPS);
            actual_step(&world, player, nav, &mut motion, control);
            if motion.grounded && motion.pos.distance(target) < 8.0 {
                return;
            }
        }
        panic!("failed actual physics route {from:?} -> {to:?}; ended at {:?}, target {target:?}, blocked {:?}", motion.pos, agent.blocked);
    }

    #[test]
    fn climbs_short_and_three_tile_ledges_with_a_full_size_fish() {
        for height in 1..=3 {
            let mut tiles = (0..16)
                .map(|x| (x, 0, TileCollisionKind::Solid))
                .collect::<Vec<_>>();
            for y in 1..=height {
                tiles.push((6, y, TileCollisionKind::Solid));
            }
            let map = arena(&tiles);
            let nav = Navigation::build(&map, movement());
            eprintln!(
                "ledge {height}: {} nodes, {} moves",
                nav.graph.node_count(),
                nav.graph.edge_count()
            );
            assert_reaches(
                &map,
                &nav,
                NavNode(ivec2(5, 1)),
                NavNode(ivec2(6, height as i32 + 1)),
            );
        }
    }

    #[test]
    fn drops_from_one_way_platform_and_falls_off_solid_ledges() {
        for kind in [TileCollisionKind::JumpThrough, TileCollisionKind::Solid] {
            let mut tiles = (0..16)
                .map(|x| (x, 0, TileCollisionKind::Solid))
                .collect::<Vec<_>>();
            for x in 3..=6 {
                tiles.push((x, 5, kind));
            }
            let map = arena(&tiles);
            let nav = Navigation::build(&map, movement());
            assert_reaches(&map, &nav, NavNode(ivec2(5, 6)), NavNode(ivec2(9, 1)));
        }
    }

    #[test]
    fn rejects_routes_that_only_fit_a_point_or_exceed_jump_height() {
        let mut tiles = (0..16)
            .map(|x| (x, 0, TileCollisionKind::Solid))
            .collect::<Vec<_>>();
        for y in 1..=5 {
            tiles.push((6, y, TileCollisionKind::Solid));
        }
        // One tile of headroom cannot contain a 48-pixel fish.
        tiles.push((3, 2, TileCollisionKind::Solid));
        let map = arena(&tiles);
        let nav = Navigation::build(&map, movement());
        assert!(!nav.positions.contains_key(&NavNode(ivec2(3, 1))));
        let start = NavNode(ivec2(5, 1));
        let top = NavNode(ivec2(6, 6));
        assert!(nav.graph.edge_weight(start, top).is_none());
        // The short overhead block is a valid stepping stone: two legal jumps can still reach the top.
        assert_reaches(&map, &nav, start, top);
    }

    #[test]
    fn shipped_maps_have_routes_that_work_in_the_game_collision_engine() {
        let levels = (1..=14)
            .map(|level| format!("level_{level}"))
            .chain(["detroit-riverfront", "eastern-market", "belle-isle"].map(String::from));
        for level in levels {
            let file =
                std::fs::read_to_string(format!("assets/map/levels/{level}.map.yaml")).unwrap();
            let yaml: serde_yaml::Value = serde_yaml::from_str(&file).unwrap();
            let mut map = MapMeta {
                grid_size: uvec2(
                    yaml["grid_size"][0].as_u64().unwrap() as u32,
                    yaml["grid_size"][1].as_u64().unwrap() as u32,
                ),
                tile_size: vec2(
                    yaml["tile_size"][0].as_f64().unwrap() as f32,
                    yaml["tile_size"][1].as_f64().unwrap() as f32,
                ),
                ..default()
            };
            for layer in yaml["layers"].as_sequence().unwrap() {
                let mut out = MapLayerMeta::default();
                for tile in layer["tiles"].as_sequence().unwrap() {
                    let collision = match tile["collision"].as_str().unwrap() {
                        "Solid" => TileCollisionKind::Solid,
                        "JumpThrough" => TileCollisionKind::JumpThrough,
                        _ => TileCollisionKind::Empty,
                    };
                    out.tiles.push(MapTileMeta {
                        pos: uvec2(
                            tile["pos"][0].as_u64().unwrap() as u32,
                            tile["pos"][1].as_u64().unwrap() as u32,
                        ),
                        collision,
                        ..default()
                    });
                }
                map.layers.push(out);
            }
            let nav = Navigation::build(&map, movement());
            let mut nodes = nav.graph.nodes().collect::<Vec<_>>();
            nodes.sort();
            let (from, reachable) = nodes
                .into_iter()
                .map(|from| {
                    let reachable =
                        petgraph::algo::dijkstra(&nav.graph, from, None, |(_, _, e)| e.frames);
                    (from, reachable)
                })
                .max_by_key(|(_, reachable)| reachable.len())
                .unwrap();
            let to = *reachable
                .keys()
                .max_by(|a, b| {
                    nav.positions[*a]
                        .distance_squared(nav.positions[&from])
                        .total_cmp(&nav.positions[*b].distance_squared(nav.positions[&from]))
                        .then_with(|| a.cmp(b))
                })
                .unwrap();
            assert_ne!(from, to, "level {level} has no usable routes");
            eprintln!("level {level}: {} standable positions, {} validated moves, checking {from:?} -> {to:?}", nav.graph.node_count(), nav.graph.edge_count());
            assert_reaches(&map, &nav, from, to);
            let mut spawn_nodes = Vec::new();
            for layer in yaml["layers"].as_sequence().unwrap() {
                if let Some(elements) = layer["elements"].as_sequence() {
                    for element in elements {
                        if element["element"]
                            .as_str()
                            .unwrap_or("")
                            .contains("player_spawner")
                        {
                            let position = vec2(
                                element["pos"][0].as_f64().unwrap() as f32,
                                element["pos"][1].as_f64().unwrap() as f32,
                            );
                            if let Some(node) = nav.nearest(position) {
                                spawn_nodes.push(node);
                            }
                        }
                    }
                }
            }
            spawn_nodes.sort();
            spawn_nodes.dedup();
            for from in spawn_nodes {
                let reachable =
                    petgraph::algo::dijkstra(&nav.graph, from, None, |(_, _, e)| e.frames);
                let to = *reachable
                    .keys()
                    .max_by_key(|node| (node.0.y, node.0.x))
                    .unwrap();
                if from != to {
                    assert_reaches(&map, &nav, from, to);
                }
            }
        }
    }
}
