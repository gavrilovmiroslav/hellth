use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{
        camera::{CameraUpdateSystem, ScalingMode},
        view::RenderLayers,
    },
    transform::TransformSystem,
};
use bevy_asset_loader::prelude::*;

use self::{
    camera::{focus_camera, FollowCameraMarker, MovingCameraMarker, SvarogCameraPlugin},
    feel::SvarogFeelPlugin,
    fov::RecalculateFOVEvent,
    grid::{GameEntity, Grid, SvarogGridPlugin, WorldData},
    loading::SvarogLoadingPlugin,
    player::{character_controls, SvarogPlayerPlugin},
    procgen::{PlayerMarker, ProcGenEvent, SvarogProcgenPlugin},
    turns::SvarogTurnPlugin,
    window::SvarogWindowPlugins,
};

pub mod camera;
pub mod feel;
pub mod fov;
pub mod grid;
pub mod loading;
pub mod player;
pub mod procgen;
pub mod sprite;
pub mod turns;
pub mod window;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameStates {
    #[default]
    AssetLoading,
    Setup,
    Game,
}

#[derive(AssetCollection, Resource)]
pub struct GameAssets {
    #[asset(key = "atlas")]
    pub atlas: Handle<TextureAtlas>,
}

#[derive(Event)]
pub enum CharacterIntent {
    Move(Entity, IVec2),
}

#[derive(Event)]
pub struct StartGameEvent;

fn start_game(mut commands: Commands, mut procgen_events: EventWriter<ProcGenEvent>) {
    commands.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
            },
            projection: OrthographicProjection {
                far: 1000.,
                near: -1000.,
                scale: 1.0,
                scaling_mode: ScalingMode::WindowSize(2.0),
                ..Default::default()
            },
            camera: Camera {
                order: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        MovingCameraMarker,
        RenderLayers::layer(0),
    ));

    commands.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::None,
            },
            projection: OrthographicProjection {
                far: 1000.,
                near: -1000.,
                scale: 1.0,
                scaling_mode: ScalingMode::WindowSize(2.0),
                ..Default::default()
            },
            camera: Camera {
                order: 1,
                ..Default::default()
            },

            ..Default::default()
        },
        FollowCameraMarker,
        RenderLayers::layer(1),
    ));

    procgen_events.send(ProcGenEvent);
}

pub struct SvarogGamePlugin;

impl Plugin for SvarogGamePlugin {
    fn build(&self, bevy: &mut App) {
        bevy.add_plugins(SvarogWindowPlugins)
            .add_plugins(SvarogLoadingPlugin)
            .add_plugins(SvarogGridPlugin)
            .add_plugins(SvarogFeelPlugin)
            .add_plugins(SvarogProcgenPlugin)
            .add_plugins(SvarogCameraPlugin)
            .add_plugins(SvarogTurnPlugin)
            .add_plugins(SvarogPlayerPlugin)
            .add_systems(OnEnter(GameStates::Game), start_game);
    }
}
