use bevy::{prelude::*, render::camera::ScalingMode};

use super::{
    grid::{GameEntity, Grid},
    procgen::PlayerMarker,
    GameStates,
};

#[derive(Component)]
pub struct MovingCameraMarker;

#[derive(Component)]
pub struct FollowCameraMarker;

#[derive(Default)]
pub enum CameraMovingMode {
    #[default]
    Calm,
    Tracking,
}

#[derive(Resource)]
pub struct CameraSettings {
    pub tracking_speed: f32,
    pub tracking_distance: f32,
    pub stop_tracking_under: f32,
}

pub fn focus_camera(
    mut main_camera: Query<&mut Transform, With<MovingCameraMarker>>,
    mut follow_cameras: Query<
        &mut Transform,
        (Without<MovingCameraMarker>, With<FollowCameraMarker>),
    >,
    player: Query<&GameEntity, With<PlayerMarker>>,
    camera_settings: Res<CameraSettings>,
    grid: Option<Res<Grid>>,
    time: Res<Time>,
    mut mode: Local<CameraMovingMode>,
) {
    let Some(grid) = grid else {
        return;
    };

    let Ok(player_game_entity) = player.get_single() else {
        return;
    };

    for mut camera_transform in &mut main_camera {
        let target = grid
            .get_tile_position(player_game_entity.position)
            .translation;

        match *mode {
            CameraMovingMode::Calm => {
                let dist = camera_transform.translation.distance(target);
                if dist > camera_settings.tracking_distance {
                    *mode = CameraMovingMode::Tracking;
                }
            }
            CameraMovingMode::Tracking => {
                let direction = (target - camera_transform.translation).normalize_or_zero();
                camera_transform.translation +=
                    direction * camera_settings.tracking_speed * time.delta_seconds();

                let dist = camera_transform.translation.distance(target);
                if dist < camera_settings.stop_tracking_under {
                    *mode = CameraMovingMode::Calm;
                }
            }
        }

        for mut follow_camera in &mut follow_cameras {
            follow_camera.translation = camera_transform.translation;
        }
    }
}

fn debug_camera(mut camera_query: Query<&mut OrthographicProjection>, keys: Res<Input<KeyCode>>) {
    for mut projection in &mut camera_query {
        if let ScalingMode::WindowSize(size) = projection.scaling_mode {
            let mut new_size = size;
            if keys.just_pressed(KeyCode::F1) {
                new_size = 1.0;
            } else if keys.just_pressed(KeyCode::F2) {
                new_size = 2.0;
            } else if keys.just_pressed(KeyCode::F3) {
                new_size = 3.0;
            } else if keys.just_pressed(KeyCode::F4) {
                new_size = 4.0;
            }

            projection.scaling_mode = ScalingMode::WindowSize(new_size);
        }
    }
}

pub struct SvarogCameraPlugin;
impl Plugin for SvarogCameraPlugin {
    fn build(&self, bevy: &mut bevy::prelude::App) {
        bevy.insert_resource(CameraSettings {
            tracking_speed: 128.0,
            tracking_distance: 160.0,
            stop_tracking_under: 16.0,
        })
        .add_systems(PostUpdate, focus_camera.run_if(in_state(GameStates::Game)))
        .add_systems(PostUpdate, debug_camera);
    }
}
