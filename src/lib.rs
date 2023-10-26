#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;
use bevy_tokio_tasks::*;
use veilid_duplex::veilid::P2PApp;

#[derive(Resource, Deref, DerefMut)]
struct VeilidApp(P2PApp);

pub struct VeilidPlugin;
impl Plugin for VeilidPlugin {
    fn build(&self, app: &mut App) {
        // app.insert_resource(veilid_app);
        app.add_systems(Startup, initialize_veilid_app);
        app.add_event::<VeilidInitializedEvent>();
    }
}

fn initialize_veilid_app(runtime: ResMut<TokioTasksRuntime>) {
    runtime.spawn_background_task(|mut ctx| async move {
        let veilid_app = P2PApp::new().await;

        // ew_veilid_initialized.send(VeilidInitializedEvent);

        ctx.run_on_main_thread(move |ctx| {
            let world = ctx.world;

            let veilid_app = VeilidApp(veilid_app.unwrap());
            world.insert_resource(veilid_app);

            world.send_event(VeilidInitializedEvent);
        })
        .await;
    });
}

// ------
// Events
// ------

#[derive(Event)]
pub struct VeilidInitializedEvent;
