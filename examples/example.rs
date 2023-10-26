use bevy::prelude::*;
use bevy_tokio_tasks::*;
use bevy_veilid::*;

fn setup(mut commands: Commands) {}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TokioTasksPlugin::default())
        .add_plugins(VeilidPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, on_world_initialized)
        .run();
}

fn on_world_initialized(mut er_world_initialized: EventReader<VeilidInitializedEvent>) {
    for _ in er_world_initialized.iter() {
        println!("Veilid initialized!");
    }
}
