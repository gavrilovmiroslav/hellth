use bevy::prelude::*;
use bevy_mod_imgui::ImguiContext;
use bevy_mod_picking::{
    events::{Click, Over, Pointer},
    prelude::ListenerInput,
};

use super::{
    character::Character,
    grid::{Grid, WorldData, WorldEntity},
    health::Health,
    procgen::PlayerMarker,
    GameStates,
};

#[derive(Event, Debug)]
pub struct ShowEntityDetails(Entity, f32);

impl From<ListenerInput<Pointer<Click>>> for ShowEntityDetails {
    fn from(event: ListenerInput<Pointer<Click>>) -> Self {
        ShowEntityDetails(event.target, event.hit.depth)
    }
}

pub fn on_show_details(
    mut show_details: EventReader<ShowEntityDetails>,
    world_entities: Query<&WorldEntity>,
) {
    for detail in show_details.read() {
        if let Ok(world_entity) = world_entities.get(detail.0) {
            println!(
                "Show Detail for {:?} at {:?}: {}",
                detail, world_entity.position, world_entity.name
            );
        }
    }
}

fn show_status_for_world_entities(
    player_entity: Query<(&WorldEntity, &Character, &Health), With<PlayerMarker>>,
    world_entities: Query<(&WorldEntity, &Character, &Health), Without<PlayerMarker>>,
    grid: Option<Res<Grid>>,
    world: Res<WorldData>,
    mut context: NonSendMut<ImguiContext>,
) {
    let Some(grid) = grid else {
        return;
    };

    let ui = context.ui();

    let [width, _height] = ui.io().display_size;

    let Ok((player, player_char, player_health)) = player_entity.get_single() else {
        return;
    };

    ui.window(&player.name)
        .position_pivot([1.0, 0.0])
        .position([width - 10.0, 10.0], imgui::Condition::Always)
        .size([400.0, 75.0], imgui::Condition::Always)
        .resizable(false)
        .collapsible(false)
        .focused(false)
        .build(|| {
            ui.text(format!("{:?}", player_char));
            ui.text(format!("{:?}", player_health));
        });

    let mut window_y = 85.0f32;
    for (other_entity, other_char, other_health) in &world_entities {
        let (x, y) = grid.norm(other_entity.position);
        if world.data.is_in_fov(x, y) {
            ui.window(&other_entity.name)
                .position_pivot([1.0, 0.0])
                .position([width - 10.0, window_y], imgui::Condition::Always)
                .size([400.0, 75.0], imgui::Condition::Always)
                .resizable(false)
                .collapsible(false)
                .focused(false)
                .build(|| {
                    ui.text(format!("{:?}", other_char));
                    ui.text(format!("{:?}", other_health));
                });

            window_y += 75.0;
        }
    }
}

pub struct SvarogUIPlugin;

impl Plugin for SvarogUIPlugin {
    fn build(&self, bevy: &mut App) {
        bevy.add_event::<ShowEntityDetails>();
        bevy.add_systems(
            Update,
            show_status_for_world_entities.run_if(in_state(GameStates::Game)),
        );

        bevy.add_systems(
            Update,
            on_show_details.run_if(on_event::<ShowEntityDetails>()),
        );
    }
}
