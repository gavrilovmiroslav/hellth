use bevy::{ecs::system::SystemState, prelude::*};

use crate::game::{
    fov::RecalculateFOVEvent,
    grid::{Grid, WorldData, WorldEntity},
    procgen::PlayerMarker,
};

use super::*;

#[derive(Debug)]
pub struct MoveAction {
    pub entity: Entity,
    pub direction: IVec2,
}

pub fn a_move(who: Entity, wher: IVec2) -> AbstractAction {
    Box::new(MoveAction {
        entity: who,
        direction: wher,
    })
}

enum MoveResult {
    MoveSucceed {
        next_position: IVec2,
        new_transform: Transform,
    },
    #[allow(dead_code)]
    CancelMove,
}

impl Action for MoveAction {
    fn get_affiliated_stat(&self) -> CharacterStat {
        CharacterStat::AGI
    }

    fn do_action(&self, world: &mut World) -> ActionResult {
        if self.direction == IVec2::ZERO {
            return vec![];
        }

        // this is the read-only part
        let move_result = {
            let mut read_system_state =
                SystemState::<(Res<Grid>, Res<WorldData>, Query<(&WorldEntity, &Transform)>)>::new(
                    world,
                );

            let (grid, world_data, world_entities) = read_system_state.get(world);

            let Ok((WorldEntity { position, .. }, transform)) = world_entities.get(self.entity)
            else {
                return vec![];
            };

            let next_position = *position + self.direction;
            let mut new_transform = grid.get_tile_position(next_position);
            new_transform.translation.z = transform.translation.z;

            if !world_data.solid.contains(&next_position) {
                if !world_data.blocking.contains_key(&next_position) {
                    MoveResult::MoveSucceed {
                        next_position,
                        new_transform,
                    }
                } else {
                    return vec![a_melee(self.entity, self.direction)];
                }
            } else {
                // todo: push non-solid here too
                MoveResult::CancelMove
            }
        };

        // by the end of this, we have free'd the world, so we can now do mut stuff
        match move_result {
            MoveResult::MoveSucceed {
                next_position,
                new_transform,
            } => {
                let mut write_system_state = SystemState::<(
                    Query<&mut WorldEntity>,
                    Query<(&PlayerMarker, &mut Transform)>,
                    ResMut<WorldData>,
                    EventWriter<RecalculateFOVEvent>,
                )>::new(world);

                let (
                    mut world_entity_query,
                    mut player_transform_query,
                    mut world_data,
                    mut fov_events,
                ) = write_system_state.get_mut(world);

                if let Ok(mut world_entity) = world_entity_query.get_mut(self.entity) {
                    world_data.blocking.remove(&world_entity.position);
                    world_entity.position = next_position;
                    if world_entity.blocking {
                        world_data.blocking.insert(next_position, self.entity);
                    }
                }

                if let Ok((_, mut transform)) = player_transform_query.get_mut(self.entity) {
                    transform.translation = new_transform.translation;
                    fov_events.send(RecalculateFOVEvent);
                }

                play_sfx("gameplay_step", world);
                vec![]
            }

            _ => {
                vec![]
            }
        }
    }
}
