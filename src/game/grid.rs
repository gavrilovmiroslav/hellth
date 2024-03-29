use bevy::{
    app::{Plugin, Update},
    asset::Handle,
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        query::Added,
        schedule::{common_conditions::in_state, IntoSystemConfigs, NextState, OnEnter, OnExit},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    math::{IVec2, Vec3},
    render::{
        color::Color,
        view::{RenderLayers, Visibility},
    },
    sprite::{SpriteSheetBundle, TextureAtlas, TextureAtlasSprite},
    transform::components::Transform,
    utils::{hashbrown::HashMap, HashSet},
};
use doryen_fov::MapData;

use crate::game::{GameAssets, GameStates};

#[cfg(feature = "debug_mode")]
use bevy::{
    app::Update,
    ecs::schedule::{common_conditions::in_state, IntoSystemConfigs},
    system::Query,
};

#[cfg(feature = "debug_mode")]
use super::feel::Random;
use super::procgen::ClearLevel;

#[derive(Component)]
pub struct WorldEntityMarker;

#[derive(Component, Clone)]
pub struct WorldEntity {
    pub name: String,
    pub position: IVec2,
    pub sprite_index: usize,
    pub blocking: bool,
    pub is_player: bool,
}

#[derive(Component)]
pub struct FOV;

#[derive(Bundle)]
pub struct WorldEntityBundle {
    pub entity: WorldEntity,
    pub sprite: SpriteSheetBundle,
    pub marker: WorldEntityMarker,
    pub layer: RenderLayers,
    pub fov: FOV,
    pub color: WorldEntityColor,
    pub clear: ClearLevel,
}

#[derive(Component)]
pub struct WorldEntityColor {
    pub color: Color,
}

#[derive(PartialEq, Eq)]
pub enum WorldEntityKind {
    Player,
    NPC,
    Item,
}

impl WorldEntityBundle {
    #[allow(clippy::too_many_arguments)]
    pub fn new_raw(
        mut transform: Transform,
        atlas: Handle<TextureAtlas>,
        name: &str,
        pos: IVec2,
        sprite_index: usize,
        blocking: bool,
        kind: WorldEntityKind,
        color: Option<Color>,
    ) -> Self {
        let mut sprite = TextureAtlasSprite::new(sprite_index);
        if let Some(color) = color {
            sprite.color = color;
        }

        transform.translation.z += match kind {
            WorldEntityKind::Player => 10.0,
            WorldEntityKind::NPC => 5.0,
            WorldEntityKind::Item => 1.0,
        };

        WorldEntityBundle {
            entity: WorldEntity {
                name: name.to_string(),
                position: pos,
                sprite_index,
                blocking,
                is_player: kind == WorldEntityKind::Player,
            },
            sprite: SpriteSheetBundle {
                sprite,
                texture_atlas: atlas.clone_weak(),
                transform,
                ..Default::default()
            },
            marker: WorldEntityMarker,
            layer: RenderLayers::layer(1),
            fov: FOV,
            color: WorldEntityColor {
                color: color.unwrap_or(Color::WHITE),
            },
            clear: ClearLevel,
        }
    }

    pub fn new(
        grid: &Res<Grid>,
        name: &str,
        pos: IVec2,
        sprite_index: usize,
        blocking: bool,
        kind: WorldEntityKind,
        color: Option<Color>,
    ) -> Self {
        let mut transform = grid.get_tile_position(pos);
        transform.translation.z += match kind {
            WorldEntityKind::Player => 10.0,
            WorldEntityKind::NPC => 5.0,
            WorldEntityKind::Item => 1.0,
        };

        let mut sprite = TextureAtlasSprite::new(sprite_index);
        if let Some(color) = color {
            sprite.color = color;
        }

        WorldEntityBundle {
            entity: WorldEntity {
                name: name.to_string(),
                position: pos,
                sprite_index,
                blocking,
                is_player: kind == WorldEntityKind::Player,
            },
            sprite: SpriteSheetBundle {
                sprite,
                texture_atlas: grid.atlas.clone_weak(),
                transform,
                ..Default::default()
            },
            marker: WorldEntityMarker,
            layer: RenderLayers::layer(1),
            fov: FOV,
            color: WorldEntityColor {
                color: color.unwrap_or(Color::WHITE),
            },
            clear: ClearLevel,
        }
    }
}

fn on_world_entity_placed(
    world_data: Option<ResMut<WorldData>>,
    world_entities: Query<(Entity, &WorldEntity), Added<WorldEntity>>,
) {
    let Some(mut world_data) = world_data else {
        return;
    };

    for (new_entity, world_entity) in &world_entities {
        if world_entity.blocking {
            world_data
                .blocking
                .insert(world_entity.position, new_entity);
        }
    }
}

#[derive(Resource)]
pub struct Grid {
    pub size: IVec2,
    pub tile: IVec2,
    pub atlas: Handle<TextureAtlas>,
    pub entities: HashMap<IVec2, Entity>,
}

impl Grid {
    pub fn norm(&self, tile: IVec2) -> (usize, usize) {
        let x = (tile.x + self.size.x / 2 + 1) as usize;
        let y = (tile.y + self.size.y / 2 + 1) as usize;

        (x.min(self.size.x as usize), y.min(self.size.y as usize))
    }
}

#[derive(Resource)]
pub struct WorldData {
    pub data: MapData,
    pub solid: HashSet<IVec2>,
    pub memory: HashSet<IVec2>,
    pub blocking: HashMap<IVec2, Entity>,
}

#[derive(Component, Default, Clone, Copy, PartialEq)]
pub enum Passability {
    #[default]
    Passable,
    Blocking,
    SightBlocking,
}

impl Grid {
    pub fn get_tile_position(&self, position: IVec2) -> Transform {
        Transform::from_translation(Vec3::new(
            (self.tile.x * position.x) as f32,
            (self.tile.y * position.y) as f32,
            0.0,
        ))
    }

    pub fn spawn(&self, commands: &mut Commands, index: usize, position: IVec2) -> Entity {
        commands
            .spawn((
                SpriteSheetBundle {
                    transform: self.get_tile_position(position),
                    sprite: TextureAtlasSprite::new(index),
                    texture_atlas: self.atlas.clone_weak(),
                    visibility: Visibility::Hidden,
                    ..Default::default()
                },
                Passability::Passable,
            ))
            .id()
    }

    pub fn get(&self, position: IVec2) -> Option<&Entity> {
        self.entities.get(&position)
    }
}

fn create_grid_resource(mut commands: Commands, assets: Res<GameAssets>) {
    commands.insert_resource(Grid {
        size: IVec2::new(120, 62),
        tile: IVec2::new(16, 16),
        atlas: assets.atlas.clone_weak(),
        entities: Default::default(),
    });

    commands.insert_resource(WorldData {
        data: MapData::new(122, 64),
        solid: Default::default(),
        memory: Default::default(),
        blocking: Default::default(),
    });
}

fn initialize_grid(
    mut commands: Commands,
    mut grid: ResMut<Grid>,
    mut next_state: ResMut<NextState<GameStates>>,
) {
    let size = grid.size;
    for i in (0..=size.y as usize).rev() {
        for j in 0..=size.x as usize {
            let position = IVec2::new(j as i32 - grid.size.x / 2, i as i32 - grid.size.y / 2);
            let spawned = grid.spawn(&mut commands, 0, position);
            grid.entities.insert(position, spawned);
        }
    }

    next_state.set(GameStates::Game);
}

pub struct SvarogGridPlugin;

impl Plugin for SvarogGridPlugin {
    fn build(&self, bevy: &mut bevy::prelude::App) {
        bevy.add_systems(OnExit(GameStates::AssetLoading), create_grid_resource)
            .add_systems(
                Update,
                on_world_entity_placed.run_if(in_state(GameStates::Game)),
            )
            .add_systems(OnEnter(GameStates::Setup), initialize_grid);
    }
}
