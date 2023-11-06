#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![feature(async_closure)]

use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_tokio_tasks::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;
use veilid_duplex::veilid::*;
use veilid_duplex::veilid_core::*;

pub use veilid_duplex;

// ------
// Resources
// ------

#[derive(Resource, Deref, DerefMut, Default)]
pub struct VeilidApp(pub Option<VeilidDuplex>);

// #[derive(Resource)]
// pub struct VeilidDuplexMessageLog {
//     pub uuids: Vec<Uuid>,
// }

// ------
// Events
// ------

#[derive(Event)]
pub struct VeilidInitializedEvent;

#[derive(Event)]
pub struct EventSendMessage<T> {
    pub message: T,
    pub dht_key: CryptoTyped<CryptoKey>,
    pub uuid: Uuid,
}

impl<T: DeserializeOwned + Serialize + std::marker::Sync + std::marker::Send + Clone + 'static>
    EventSendMessage<T>
{
    pub fn new(message: T, dht_key: CryptoTyped<CryptoKey>) -> EventSendMessage<T> {
        EventSendMessage {
            message,
            dht_key,
            uuid: Uuid::new_v4(),
        }
    }
}

#[derive(Event)]
pub struct EventMessageSent {
    pub uuid: Uuid,
}

#[derive(Event)]
pub struct EventReceiveMessage<T> {
    pub message: T,
}

#[derive(Event)]
pub struct EventError;

fn initialize_veilid_app(runtime: ResMut<TokioTasksRuntime>) {
    runtime.spawn_background_task(|mut ctx| async move {
        let result = VeilidDuplex::new().await;
        if result.is_err() {
            ctx.run_on_main_thread(move |ctx| {
                let world = ctx.world;
                world.send_event(EventError);
            })
            .await;
            return;
        }

        let app = result.unwrap();

        ctx.run_on_main_thread(move |ctx| {
            let world = ctx.world;
            world.insert_resource(VeilidApp(Some(app.clone())));
            world.send_event(VeilidInitializedEvent);
        })
        .await;
    });
}

fn event_on_veilid_initialized<
    T: DeserializeOwned + Serialize + std::marker::Sync + std::marker::Send + Clone + 'static,
>(
    runtime: ResMut<TokioTasksRuntime>,
    mut e_veilid_initialized: EventReader<VeilidInitializedEvent>,
    veilid_app: Res<VeilidApp>,
) {
    for _e in e_veilid_initialized.iter() {
        let mut veilid_app = veilid_app.clone().unwrap();
        runtime.spawn_background_task(|mut ctx| async move {
            let on_app_message = async move |message: AppMessage<T>| {
                let message = message.clone();

                ctx.run_on_main_thread(move |ctx| {
                    let world = ctx.world;

                    world.send_event(EventReceiveMessage {
                        message: message.data,
                    });
                })
                .await;
            };
            let _ = veilid_app.network_loop(on_app_message).await;
        });
    }
}

fn on_ev_send_message<
    T: DeserializeOwned + Serialize + std::marker::Sync + std::marker::Send + Clone + 'static,
>(
    mut er_send_message: EventReader<EventSendMessage<T>>,
    // mut message_log: ResMut<VeilidDuplexMessageLog>,
    veilid_app: Res<VeilidApp>,
    runtime: ResMut<TokioTasksRuntime>,
) {
    if veilid_app.is_none() {
        return;
    }

    let veilid_app = veilid_app.clone().unwrap();
    let origin_dht_key = veilid_app.our_dht_key;

    for e in er_send_message.iter() {
        let veilid_app = veilid_app.clone();
        let destination_dht_key = e.dht_key;

        let app_message: AppMessage<T> = AppMessage {
            data: e.message.clone(),
            dht_record: origin_dht_key,
        };
        let message_uuid = e.uuid;

        runtime.spawn_background_task(move |mut ctx| async move {
            let result = veilid_app
                .send_message(app_message, destination_dht_key)
                .await;

            ctx.run_on_main_thread(move |ctx| {
                let world = ctx.world;
                if result.is_err() {
                    world.send_event(EventError {});
                } else {
                    world.send_event(EventMessageSent { uuid: message_uuid });
                }
            })
            .await;
        });
    }
}

// ---
// Plugin
// ---

#[derive(Default, Clone)]
pub struct VeilidPlugin<
    T: DeserializeOwned + Serialize + std::marker::Sync + std::marker::Send + Clone + 'static,
>(pub PhantomData<T>);

impl<T: DeserializeOwned + Serialize + std::marker::Sync + std::marker::Send + Clone + 'static>
    Plugin for VeilidPlugin<T>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<VeilidApp>();
        app.add_systems(Startup, initialize_veilid_app);
        app.add_systems(
            Update,
            (on_ev_send_message::<T>, event_on_veilid_initialized::<T>),
        );
        app.add_event::<VeilidInitializedEvent>();
        app.add_event::<EventError>();
        app.add_event::<EventReceiveMessage<T>>();
        app.add_event::<EventSendMessage<T>>();
        app.add_event::<EventMessageSent>();
        // .insert_resource(VeilidDuplexMessageLog { uuids: Vec::new() });
    }
}
