use bevy::prelude::*;
use doryen_fov::{FovAlgorithm, FovRecursiveShadowCasting};

use super::{
    grid::{Grid, WorldData, WorldEntity, FOV},
    procgen::PlayerMarker,
};

#[derive(Event)]
pub struct RecalculateFOVEvent;

#[derive(Component)]
pub struct Sight(pub u32);

#[derive(Component, Default)]
pub struct LastSeen(pub Option<IVec2>);

pub fn on_new_fov_added(
    query: Query<Added<FOV>>,
    mut recalc_event: EventWriter<RecalculateFOVEvent>,
) {
    for _ in &query {
        recalc_event.send(RecalculateFOVEvent);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn recalculate_fov(
    mut recalc_event: EventReader<RecalculateFOVEvent>,
    player_entity: Query<(&WorldEntity, &Sight), With<PlayerMarker>>,
    grid: Option<Res<Grid>>,
    map: Option<ResMut<WorldData>>,
    mut non_players: Query<(Entity, &WorldEntity, &mut Transform), Without<PlayerMarker>>,
    mut last_seen: Query<&mut LastSeen>,
    mut sprites: Query<&mut TextureAtlasSprite>,
    mut visibility: Query<&mut Visibility>,
) {
    if !recalc_event.is_empty() {
        recalc_event.clear();
    } else {
        return;
    }

    let Some(grid) = grid else {
        return;
    };

    let Some(mut map) = map else {
        return;
    };

    let Ok((world_entity, sight)) = &player_entity.get_single() else {
        return;
    };

    let mut fov = FovRecursiveShadowCasting::new();

    map.data.clear_fov();

    {
        let (x, y) = grid.norm(world_entity.position);
        fov.compute_fov(&mut map.data, x, y, sight.0 as usize, true);
    }

    grid.entities.iter().for_each(|(pos, e)| {
        let Ok(mut vis) = visibility.get_mut(*e) else {
            return;
        };

        let Ok(mut sprite) = sprites.get_mut(*e) else {
            return;
        };

        let (x, y) = grid.norm(*pos);
        if map.data.is_in_fov(x, y) {
            map.memory.insert(*pos);
            sprite.color = Color::WHITE;
            *vis = Visibility::Visible;
        } else if map.memory.contains(pos) {
            sprite.color = Color::GRAY;
            *vis = Visibility::Visible;
        } else {
            sprite.color = Color::BLACK;
            *vis = Visibility::Hidden;
        }
    });

    for (non_player_entity, world_entity, mut transform) in &mut non_players {
        let Ok(mut vis) = visibility.get_mut(non_player_entity) else {
            continue;
        };

        let Ok(mut last_seen_at) = last_seen.get_mut(non_player_entity) else {
            continue;
        };

        let Ok(mut sprite) = sprites.get_mut(non_player_entity) else {
            return;
        };

        if last_seen_at.0.is_none() {
            let (x, y) = grid.norm(world_entity.position);
            if map.data.is_in_fov(x, y) {
                *vis = Visibility::Visible;
                sprite.color = Color::WHITE;
                transform.translation = grid.get_tile_position(world_entity.position).translation;
                *last_seen_at = LastSeen(Some(world_entity.position));
            } else {
                *vis = Visibility::Hidden;
            }
        } else {
            let (x, y) = grid.norm(last_seen_at.0.unwrap());
            if map.data.is_in_fov(x, y) {
                *vis = Visibility::Visible;
                sprite.color = Color::WHITE;
                transform.translation = grid.get_tile_position(world_entity.position).translation;
                *last_seen_at = LastSeen(Some(world_entity.position));
            } else {
                *vis = Visibility::Visible;
                sprite.color = Color::GRAY;
                *last_seen_at = LastSeen(Some(world_entity.position));
            }
        }
    }
}
