use bevy::{
    prelude::*,
    render::view::RenderLayers,
    utils::{HashMap, HashSet},
};
use bevy_mod_picking::{
    events::{Click, Pointer},
    prelude::On,
    PickableBundle,
};
use doryen_fov::MapData;

use crate::game::{
    ai::PendingActions,
    character::{Character, CharacterStat},
    feel::TweenSize,
    fov::Sight,
    grid::{WorldEntityBundle, WorldEntityKind},
    health::{Health, RecoveryCounter},
    inventory::{CarriedItems, EquippedItems, ItemBuilder, ItemType},
    magic::{Focus, Magic},
    mobs::{make_acolyte, make_bat, make_goblin, make_healer, make_orc, make_thaumaturge},
    sprite::{ChangePassability, ChangeSprite},
    sprites::*,
    turns::{Energy, TurnOrderEntity, TurnTaker},
    ui::ShowEntityDetails,
};

use super::{
    feel::Random, fov::{on_new_fov_added, recalculate_fov, RecalculateFOVEvent}, grid::{Grid, Passability, WorldData, WorldEntity}, history::HistoryLog, turns::{TurnCounter, TurnOrder, TurnOrderProgressEvent}, DebugFlag, GameStates
};

#[derive(Event, PartialEq, Eq)]
pub enum ProcGenEvent {
    RestartWorld,
    NextLevel,
}

#[derive(Component)]
pub struct PlayerMarker;

#[derive(Resource)]
pub struct MapRadius(pub i32);

#[derive(Resource)]
pub struct LevelDepth(pub u32, pub i32);

#[derive(Component)]
pub struct ClearLevel;

#[allow(clippy::identity_op)]
#[allow(clippy::too_many_arguments)]
pub fn generate_level(
    mut procgen: EventReader<ProcGenEvent>,
    player: Query<Entity, With<PlayerMarker>>,
    clear: Query<Entity, With<ClearLevel>>,
    mut world_entities: Query<(&mut WorldEntity, &mut Transform)>,
    mut commands: Commands,
    mut map: ResMut<WorldData>,
    mut rng: ResMut<Random>,
    mut turn_order: ResMut<TurnOrder>,
    mut sprites: Query<(&mut TextureAtlasSprite, &mut Passability)>,
    mut visibility: Query<&mut Visibility>,
    mut turn_order_progress: EventWriter<TurnOrderProgressEvent>,
    mut log: ResMut<HistoryLog>,
    mut magic: ResMut<Magic>,
    grid: Res<Grid>,
    mut radius: ResMut<MapRadius>,
    mut depth: ResMut<LevelDepth>,
) {
    let mut interiors: HashSet<IVec2> = HashSet::new();

    for proc in procgen.read() {
        let restart = proc == &ProcGenEvent::RestartWorld;

        if !restart {
            let mut r = radius.0;
            r -= 50;

            if r <= 50 {
                r = 50;
            }

            radius.0 = r;
        }

        if restart {
            magic.reset(&mut rng);
            log.clear();
            depth.0 = 1;
            for c in &clear {
                commands.entity(c).despawn_recursive();
            }
        } else {
            for c in &clear {
                if world_entities
                    .get(c)
                    .map(|c| !c.0.is_player)
                    .unwrap_or_default()
                {
                    commands.entity(c).despawn_recursive();
                }
            }
        }

        fn clear_grid(
            grid: &Res<Grid>,
            rng: &mut ResMut<Random>,
            map: &mut ResMut<WorldData>,
            radius: &ResMut<MapRadius>,
            visibility: &mut Query<&mut Visibility>,
            sprites: &mut Query<(&mut TextureAtlasSprite, &mut Passability)>,
        ) -> HashSet<IVec2> {
            grid.entities.iter().for_each(|(pos, _)| {
                map.blocking.remove(pos);
                map.solid.remove(pos);
            });

            let symbols = Tiles::default()
                .add_more(EMPTY_FLOOR, 4)
                .add_bunch(&[
                    EXTERIOR_FLOOR1,
                    EXTERIOR_FLOOR2,
                    EXTERIOR_FLOOR3,
                    EXTERIOR_FLOOR4,
                ])
                .done();

            let mut okay_for_player = HashSet::new();

            map.solid.clear();
            grid.entities.iter().for_each(|(pos, e)| {
                if let Ok(mut vis) = visibility.get_mut(*e) {
                    *vis = Visibility::Hidden;
                }

                if let Ok((mut sprite, mut passable)) = sprites.get_mut(*e) {
                    let dist = pos.distance_squared(IVec2::ZERO);
                    let r = radius.0;
                    if dist < r {
                        sprite.index = symbols[rng.gen(0..symbols.len() as i32) as usize];
                        sprite.color = Color::WHITE;
                        *passable = Passability::Passable;
                        okay_for_player.insert(*pos);
                        map.data.set_transparent(
                            (pos.x + grid.size.x / 2 + 1) as usize,
                            (pos.y + grid.size.y / 2 + 1) as usize,
                            true,
                        );
                    } else {
                        sprite.index = VOID.into();
                        sprite.color = Color::WHITE;
                        *passable = Passability::Blocking;
                        map.data.set_transparent(
                            (pos.x + grid.size.x / 2 + 1) as usize,
                            (pos.y + grid.size.y / 2 + 1) as usize,
                            false,
                        );
                        map.solid.insert(*pos);
                    }
                }
            });

            okay_for_player
        }

        #[allow(clippy::identity_op)]
        fn make_obstructions(
            commands: &mut Commands,
            count: usize,
            size: IVec2,
            rng: &mut ResMut<Random>,
            grid: &Res<Grid>,
            depth: &ResMut<LevelDepth>,
            map: &mut ResMut<WorldData>,
            okay: &mut HashSet<IVec2>,
            interiors: &mut HashSet<IVec2>,
        ) {
            let mut forests = vec![EMPTY_FLOOR, FOREST1, FOREST2, FOREST3];
            if depth.0 == 3 {
                forests.extend(vec![ FOREST4, FOREST4, FOREST4, FOREST5 ]);
            } else if depth.0 == 4 { 
                forests.extend(vec![ FOREST4, FOREST4, FOREST5, FOREST6, FOREST7, FOREST7 ]);
            } else if depth.0 == 5 {
                forests.extend(vec![ FOREST4, FOREST7, FOREST8, FOREST7, FOREST8, FOREST7, FOREST8 ]);
            }
            let forest_tiles = Tiles::default()
                .add_bunch(forests.as_slice())
                .done();

            let ruin_tiles = Tiles::default()
                .add_more(WALL1, 4)
                .add_bunch(&[WALL2, WALL3, WALL4, WALL5, WALL6])
                .done();

            for _attempt in 0..count {
                let half = size / 2;
                let middle = IVec2::new(rng.gen(-half.x..half.x), rng.gen(-half.y..half.y));

                let (tiles, passability) = if rng.percent(45 - (depth.0 * 5).clamp(0, 25)) {
                    (forest_tiles.as_slice(), Passability::SightBlocking)
                } else {
                    (ruin_tiles.as_slice(), Passability::Blocking)
                };

                let IVec2 { x, y } = rng.gen2d(3..6, 4..7);
                for i in -x..=x {
                    for j in -y..=y {
                        let pos = middle + IVec2::new(i, j);
                        let dist = middle.distance_squared(pos);

                        let index = rng.from(tiles);

                        if okay.contains(&pos) && rng.percent(3 * dist as u32) {
                            commands.add(ChangeSprite {
                                position: pos,
                                index,
                            });

                            commands.add(ChangePassability {
                                position: pos,
                                passable: passability,
                            });

                            if passability == Passability::Blocking {
                                map.solid.insert(pos);
                                interiors.remove(&pos);
                            } else {
                                interiors.insert(pos);
                            }

                            okay.remove(&pos);

                            map.data.set_transparent(
                                (pos.x + grid.size.x / 2 + 1) as usize,
                                (pos.y + grid.size.y / 2 + 1) as usize,
                                passability == Passability::Passable,
                            );
                        }
                    }
                }
            }
        }

        #[allow(clippy::identity_op)]
        fn make_houses(
            commands: &mut Commands,
            count: usize,
            size: IVec2,
            rng: &mut ResMut<Random>,
            grid: &Res<Grid>,
            map: &mut ResMut<WorldData>,
            okay: &mut HashSet<IVec2>,
            interiors: &mut HashSet<IVec2>,
        ) {
            let wall_tiles: Vec<usize> = Tiles::default().add_one(WALL1).done();
            let floor_tiles: Vec<usize> = Tiles::default()
                .add_more(INTERIOR_FLOOR2, 9)
                .add_bunch(&[
                    EXTERIOR_FLOOR1,
                    EXTERIOR_FLOOR2,
                    EXTERIOR_FLOOR3,
                    EXTERIOR_FLOOR4,
                    INTERIOR_FLOOR1,
                ])
                .done();

            let mut walls = HashMap::new();
            for _attempt in 0..count {
                let half = size / 2;
                let dx = -half.x..half.x;
                let dy = -half.y..half.y;
                let middle = rng.gen2d(dx, dy);
                let room_size = rng.gen2d(3..7, 3..7);
                for i in -room_size.x..=room_size.x {
                    for j in -room_size.y..=room_size.y {
                        if rng.gen(0..100) > 70 {
                            continue;
                        }

                        let ij = IVec2::new(i, j);
                        let pos = middle + ij;

                        if okay.contains(&pos) {
                            let index = rng.from(&wall_tiles);
                            commands.add(ChangeSprite {
                                position: pos,
                                index,
                            });

                            walls.insert(pos, index);
                            map.data.set_transparent(
                                (pos.x + grid.size.x / 2 + 1) as usize,
                                (pos.y + grid.size.y / 2 + 1) as usize,
                                false,
                            );
                        }
                    }
                }

                for i in -room_size.x + 1..room_size.x {
                    for j in -room_size.y + 1..room_size.y {
                        let ij = IVec2::new(i, j);
                        let pos = middle + ij;

                        if okay.contains(&pos) {
                            let index = rng.from(&floor_tiles);
                            commands.add(ChangeSprite {
                                position: pos,
                                index,
                            });

                            walls.remove(&pos);
                            map.data.set_transparent(
                                (pos.x + grid.size.x / 2 + 1) as usize,
                                (pos.y + grid.size.y / 2 + 1) as usize,
                                true,
                            );

                            if map.solid.contains(&pos) {
                                map.solid.remove(&pos);
                            }

                            interiors.insert(pos);
                        }
                    }
                }

                for (pos, wall) in &walls {
                    if okay.contains(pos) {
                        commands.add(ChangePassability {
                            position: *pos,
                            passable: if *wall != wall_tiles[0] {
                                Passability::Passable
                            } else {
                                Passability::Blocking
                            },
                        });
                        okay.remove(pos);
                        interiors.remove(pos);
                        map.solid.insert(*pos);
                    }
                }
            }
        }

        let size = grid.size;

        map.data = MapData::new(122, 64);
        map.memory.clear();

        turn_order.clear();

        let mut okay = clear_grid(
            &grid,
            &mut rng,
            &mut map,
            &radius,
            &mut visibility,
            &mut sprites,
        );

        make_obstructions(
            &mut commands,
            20 + 3 * depth.0 as usize,
            size,
            &mut rng,
            &grid,
            &depth,
            &mut map,
            &mut okay,
            &mut interiors,
        );

        make_houses(
            &mut commands,
            40 - 2 * depth.0 as usize,
            size,
            &mut rng,
            &grid,
            &mut map,
            &mut okay,
            &mut interiors,
        );

        let stats = [
            CharacterStat::STR,
            CharacterStat::ARC,
            CharacterStat::INT,
            CharacterStat::WIS,
            CharacterStat::WIL,
            CharacterStat::AGI,
        ];

        let mut places_for_interior = rng.shuffle(interiors.clone().into_iter().collect());

        let mut places_for_spawning = rng.shuffle(
            okay.into_iter()
                .filter(|i| !interiors.contains(i))
                .collect::<Vec<_>>(),
        );

        if !restart {
            // TODO: move player to safe place
            if let Ok((mut world, mut transform)) = world_entities.get_mut(player.single()) {
                if let Some(place) = places_for_spawning.pop() {
                    world.position = place;
                    let z = transform.translation.z;
                    *transform = grid.get_tile_position(place);
                    transform.translation.z = z;
                }
            }
        }
        
        // add scrolls
        for _ in 1..4 {
            let mut builder = ItemBuilder::default()
                .with_name("Arcane Writ")
                .with_image(rng.from(&[SCROLL1, SCROLL2]))
                .with_type(ItemType::Scroll);
            
            builder.create_at(
                places_for_interior.pop().unwrap_or_default(),
                &mut commands,
                &grid,
                &magic,
            )
        }

        // add staffs
        for _ in 1..(5 + depth.0) {
            let mut builder = ItemBuilder::default()
                .with_name("Staff")
                .with_image(rng.from(&[STAFF1, STAFF2, STAFF3, STAFF4, STAFF5]))
                .with_type(ItemType::Weapon);

            builder = builder.with_stat(CharacterStat::ARC, 1);
            builder = builder.with_stat(CharacterStat::WIS, 1);
            for _ in 0..rng.gen(0..(2 + depth.0 as i32).clamp(2, 4)) {
                let mut stat = rng.from(&stats);
                let mut power = 0;
                let mut attempt = 0;
                while power == 0 || (stat == CharacterStat::ARC && stat == CharacterStat::WIS) {
                    power = rng.gen((-1 - depth.0 as i32)..(5 + depth.0 as i32));
                    stat = rng.from(&stats);
                    attempt += 1;
                    if attempt > 10 {
                        break;
                    }
                }

                builder = builder.with_stat(stat, power);
            }

            builder.create_at(
                places_for_interior.pop().unwrap_or_default(),
                &mut commands,
                &grid,
                &magic,
            )
        }

        // add swords
        for _ in 1..(depth.0 as i32 + rng.gen(0..(2 + depth.0 as i32).clamp(1, 4))) {
            let mut builder = ItemBuilder::default()
                .with_name("Sword")
                .with_image(rng.from(&[SWORD1, SWORD2, SWORD3, SWORD4, SWORD5]))
                .with_type(ItemType::Weapon);

            builder = builder.with_stat(CharacterStat::STR, 2 + (depth.0 / 3) as i32);
            for _ in 0..rng.gen(0..2) {
                let mut stat = rng.from(&stats);
                let mut power = 0;
                let mut attempt = 0;
                while power == 0 || stat == CharacterStat::STR {
                    power = rng.gen((-1 - depth.0 as i32)..(5 + depth.0 as i32));
                    stat = rng.from(&stats);
                    attempt += 1;
                    if attempt > 10 {
                        break;
                    }
                }

                builder = builder.with_stat(stat, power);
            }

            builder.create_at(
                places_for_interior.pop().unwrap_or_default(),
                &mut commands,
                &grid,
                &magic,
            )
        }

        for _ in 1..(depth.0 as i32 + rng.gen(0..(2 + depth.0 as i32).clamp(2, 4))) {
            let mut builder = ItemBuilder::default()
                .with_name("Dagger")
                .with_image(rng.from(&[DAGGER1, DAGGER2, DAGGER3, DAGGER4, DAGGER5]))
                .with_type(ItemType::Weapon);

            builder = builder.with_stat(CharacterStat::AGI, 2 + (depth.0 / 3) as i32);
            for _ in 0..rng.gen(0..2) {
                let mut stat = rng.from(&stats);
                let mut power = 0;
                let mut attempt = 0;
                while power == 0 || stat == CharacterStat::AGI {
                    power = rng.gen((-2 - depth.0 as i32)..(3 + depth.0 as i32));
                    stat = rng.from(&stats);
                    attempt += 1;
                    if attempt > 10 {
                        break;
                    }
                }

                builder = builder.with_stat(stat, power);
            }

            builder.create_at(
                places_for_interior.pop().unwrap_or_default(),
                &mut commands,
                &grid,
                &magic,
            )
        }

        // add player
        if restart {
            let mut player = commands.spawn(WorldEntityBundle::new(
                &grid,
                "You",
                places_for_spawning.pop().unwrap_or_default(),
                EMO_MAGE.into(),
                true,
                WorldEntityKind::Player,
                None,
            ));
            player
                .with_children(|f| {
                    f.spawn((
                        SpriteSheetBundle {
                            sprite: TextureAtlasSprite::new(0),
                            texture_atlas: grid.atlas.clone_weak(),
                            transform: Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
                            ..Default::default()
                        },
                        RenderLayers::layer(1),
                    ));
                    f.spawn(((
                        SpriteSheetBundle {
                            sprite: TextureAtlasSprite::new(SELECTION.into()),
                            texture_atlas: grid.atlas.clone_weak(),
                            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0))
                                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
                            ..Default::default()
                        },
                        RenderLayers::layer(1),
                        TweenSize {
                            baseline: 1.5,
                            max: 0.25,
                        },
                    ),));
                })
                .insert((
                    Character {
                        agility: 5,
                        ..Default::default()
                    },
                    RecoveryCounter::default(),
                    CarriedItems::default(),
                    EquippedItems::default(),
                    PlayerMarker,
                    PendingActions::default(),
                    Health::new(18),
                    Focus(0),
                    TurnTaker,
                    PickableBundle::default(),
                    On::<Pointer<Click>>::send_event::<ShowEntityDetails>(),
                    Sight(6),
                ));
        } else {
            turn_order.order.push(
                TurnOrderEntity {
                    entity: player.single(),
                },
                Energy(0),
            );
        }

        // add mobs

        match depth.0 {
            1 => {
                for _ in 2..rng.gen(3..5) {
                    let aggro = rng.percent(20u32);
                    make_orc(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                        aggro,
                    );
                }

                for _ in 3..rng.gen(6..10) {
                    make_goblin(
                        &mut commands,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 0..rng.gen(0..5) {
                    make_bat(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }
            }

            2 => {
                for _ in 2..rng.gen(2..5) {
                    let aggro = rng.percent(20u32);
                    make_orc(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                        aggro,
                    );
                }

                for _ in 2..rng.gen(3..10) {
                    make_goblin(
                        &mut commands,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 0..rng.gen(2..6) {
                    make_bat(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }
            }

            3 => {
                for _ in 3..rng.gen(3..10) {
                    make_acolyte(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 2..rng.gen(2..6) {
                    make_goblin(
                        &mut commands,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 0..rng.gen(0..3) {
                    make_bat(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }
            }

            4 => {
                for _ in 2..rng.gen(4..6) {
                    make_acolyte(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 1..rng.gen(1..4) {
                    make_thaumaturge(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 1..rng.gen(1..2) {
                    let aggro = rng.percent(70u32);
                    make_orc(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                        aggro
                    );
                }

                for _ in 1..rng.gen(1..6) {
                    make_goblin(
                        &mut commands,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }
            }

            5 => {
                for _ in 5..rng.gen(5..9) {
                    make_bat(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 5..rng.gen(5..9) {
                    make_goblin(
                        &mut commands,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 1..rng.gen(3..5) {
                    make_acolyte(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                for _ in 1..rng.gen(3..4) {
                    make_thaumaturge(
                        &mut commands,
                        &mut rng,
                        &grid,
                        places_for_interior.pop().unwrap_or_default(),
                    );
                }

                make_healer(
                    &mut commands,
                    &mut rng,
                    &grid,
                    depth.1 as i32,
                    places_for_interior.pop().unwrap_or_default(),
                );
            }

            _ => {}
        }

        turn_order_progress.send(TurnOrderProgressEvent);
    }
}

pub fn debug_radius(mut map_radius: ResMut<MapRadius>, keys: Res<Input<KeyCode>>) {
    let mut radius = map_radius.0;

    if keys.just_pressed(KeyCode::F6) {
        radius -= 50;
    } else if keys.just_pressed(KeyCode::F7) {
        radius += 50;
    }

    if radius <= 50 {
        radius = 50;
    }

    map_radius.0 = radius;
}

pub fn debug_procgen(
    mut procgen_events: EventWriter<ProcGenEvent>,
    keys: Res<Input<KeyCode>>,
    mut turn_counter: ResMut<TurnCounter>,
    mut debug: ResMut<DebugFlag>,
) {
    if keys.just_pressed(KeyCode::F5) {
        procgen_events.send(ProcGenEvent::RestartWorld);
        turn_counter.0 = 0;
    }

    if keys.just_pressed(KeyCode::F12) {
        debug.0 = !debug.0;
    }
}

pub struct SvarogProcgenPlugin;

impl Plugin for SvarogProcgenPlugin {
    fn build(&self, bevy: &mut App) {
        bevy.add_event::<ProcGenEvent>()
            .add_event::<RecalculateFOVEvent>()
            .insert_resource(MapRadius(800))
            .insert_resource(LevelDepth(1, 0))
            .insert_resource(ClearColor(Color::BLACK))
            .insert_resource(Msaa::Off)
            .add_systems(Update, on_new_fov_added.run_if(in_state(GameStates::Game)))
            .add_systems(
                Update,
                recalculate_fov
                    .run_if(on_event::<RecalculateFOVEvent>())
                    .run_if(in_state(GameStates::Game)),
            )
            .add_systems(Update, (debug_radius, debug_procgen))
            .add_systems(Last, generate_level.run_if(on_event::<ProcGenEvent>()));
    }
}
