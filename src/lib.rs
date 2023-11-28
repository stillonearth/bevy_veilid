#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![feature(async_closure)]

use std::marker::PhantomData;

use anyhow::Error;
use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use copypasta::*;

#[cfg(not(target_arch = "wasm32"))]
use bevy_tokio_tasks::*;
#[cfg(target_arch = "wasm32")]
use bevy_wasm_tasks::*;

#[cfg(target_arch = "wasm32")]
use futures::executor::block_on;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;
use veilid_duplex::veilid::*;
use veilid_duplex::veilid_core::*;

pub use veilid_duplex;

#[cfg(target_arch = "wasm32")]
type TasksPlugin = WASMTasksPlugin;

#[cfg(target_arch = "wasm32")]
pub type TasksRutime = WASMTasksRuntime;

#[cfg(not(target_arch = "wasm32"))]
type TasksPlugin = TokioTasksPlugin;

#[cfg(not(target_arch = "wasm32"))]
pub type TasksRutime = TokioTasksRuntime;

// ------
// Resources
// ------

#[derive(Resource, Default)]
pub struct VeilidApp {
    pub app: Option<VeilidDuplex>,
    pub other_peer_dht: Option<CryptoTyped<CryptoKey>>,
}

#[derive(Resource, PartialEq, Eq, Clone, Copy)]
pub enum VeilidPluginStatus {
    Initializing,
    Initialized,
    ConnectedPeer,
    AwaitingPeer,
    Error,
}

// ------
// Events
// ------

#[derive(Event)]
pub struct EventVeilidInitialized;

#[derive(Event)]
pub struct EventReadFromClipboardDone(pub String);

#[derive(Event)]
pub struct EventReadFromClipboard;

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
    pub dht_key: CryptoTyped<CryptoKey>,
}

#[derive(Event)]
pub struct EventReceiveMessage<T> {
    pub message: T,
    pub dht_key: CryptoTyped<CryptoKey>,
}

#[derive(Event)]
pub struct EventAwaitingPeer;

#[derive(Event)]
pub struct EventConnectedPeer {
    pub dht_key: CryptoTyped<CryptoKey>,
}

#[derive(Event)]
pub struct EventError(pub Error);

// ---
// Systems
// ---

fn on_ev_connected_peer(
    mut reader: EventReader<EventConnectedPeer>,
    mut veilid_plugin_status: ResMut<VeilidPluginStatus>,
    mut veilid_app: ResMut<VeilidApp>,
) {
    for e in reader.read() {
        *veilid_plugin_status = VeilidPluginStatus::ConnectedPeer;
        veilid_app.other_peer_dht = Some(e.dht_key);
    }
}

fn on_ev_awaiting_peer(
    mut reader: EventReader<EventAwaitingPeer>,
    mut veilid_plugin_status: ResMut<VeilidPluginStatus>,
) {
    for _ in reader.read() {
        *veilid_plugin_status = VeilidPluginStatus::AwaitingPeer;
    }
}

fn on_ev_error(
    mut er_veilid_error: EventReader<EventError>,
    mut veilid_plugin_status: ResMut<VeilidPluginStatus>,
) {
    for _ in er_veilid_error.read() {
        *veilid_plugin_status = VeilidPluginStatus::Error;
    }
}

fn on_ev_veilid_message_sent(
    mut er_veilid_message: EventReader<EventMessageSent>,
    mut ew_connected_peer: EventWriter<EventConnectedPeer>,
    veilid_plugin_status: Res<VeilidPluginStatus>,
) {
    if *veilid_plugin_status.into_inner() == VeilidPluginStatus::AwaitingPeer {
        for m in er_veilid_message.read() {
            ew_connected_peer.send(EventConnectedPeer { dht_key: m.dht_key });
        }
    }
}

// --

fn initialize_veilid_app(runtime: ResMut<TasksRutime>) {
    runtime.spawn_background_task(|mut ctx| async move {
        let result = VeilidDuplex::new().await;
        if result.is_err() {
            ctx.run_on_main_thread(move |ctx| {
                let world = ctx.world;
                world.send_event(EventError(result.err().unwrap()));
            })
            .await;
            return;
        }

        let app = result.unwrap();

        ctx.run_on_main_thread(move |ctx| {
            let world = ctx.world;
            world.insert_resource(VeilidApp {
                app: Some(app.clone()),
                other_peer_dht: None,
            });
            world.send_event(EventVeilidInitialized);
        })
        .await;
    });
}

fn event_on_veilid_initialized<
    T: DeserializeOwned + Serialize + std::marker::Sync + std::marker::Send + Clone + 'static,
>(
    runtime: ResMut<TasksRutime>,
    mut veilid_plugin_status: ResMut<VeilidPluginStatus>,
    mut e_veilid_initialized: EventReader<EventVeilidInitialized>,
    veilid_app: Res<VeilidApp>,
) {
    for _e in e_veilid_initialized.read() {
        *veilid_plugin_status = VeilidPluginStatus::Initialized;
        let mut veilid_app = veilid_app.app.clone().unwrap();
        runtime.spawn_background_task(|mut ctx| async move {
            let on_app_message = async move |message: AppMessage<T>| {
                let message = message.clone();

                ctx.run_on_main_thread(move |ctx| {
                    let world = ctx.world;

                    world.send_event(EventReceiveMessage {
                        message: message.data,
                        dht_key: message.dht_record,
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
    mut ew_awaiting_peer: EventWriter<EventAwaitingPeer>,
    veilid_app: Res<VeilidApp>,
    runtime: ResMut<TasksRutime>,
) {
    if veilid_app.app.is_none() {
        return;
    }

    let veilid_app = veilid_app.app.clone().unwrap();
    let origin_dht_key = veilid_app.our_dht_key;

    for e in er_send_message.read() {
        let veilid_app = veilid_app.clone();
        let destination_dht_key = e.dht_key;

        let app_message: AppMessage<T> = AppMessage {
            data: e.message.clone(),
            dht_record: origin_dht_key,
            uuid: "".to_string(),
        };
        let uuid = e.uuid;
        let dht_key = e.dht_key;

        ew_awaiting_peer.send(EventAwaitingPeer);

        runtime.spawn_background_task(move |mut ctx| async move {
            let result = veilid_app
                .send_message(app_message, destination_dht_key)
                .await;

            ctx.run_on_main_thread(move |ctx| {
                let world = ctx.world;
                if result.is_err() {
                    world.send_event(EventError(result.err().unwrap()));
                } else {
                    world.send_event(EventMessageSent { uuid, dht_key });
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
        app.add_plugins(TasksPlugin::default());

        app.init_resource::<VeilidApp>();
        app.add_systems(Startup, initialize_veilid_app);
        app.add_systems(
            Update,
            (on_ev_send_message::<T>, event_on_veilid_initialized::<T>),
        );
        app.add_systems(
            Update,
            (
                on_ev_connected_peer,
                on_ev_awaiting_peer,
                on_ev_error,
                on_ev_veilid_message_sent,
            ),
        );
        // Clipboard QoL
        app.add_systems(Update, on_read_from_clipboard);
        app.add_event::<EventConnectedPeer>();
        app.add_event::<EventError>();
        app.add_event::<EventAwaitingPeer>();
        app.add_event::<EventVeilidInitialized>();
        app.add_event::<EventReceiveMessage<T>>();
        app.add_event::<EventSendMessage<T>>();
        app.add_event::<EventMessageSent>();
        app.add_event::<EventReadFromClipboardDone>();
        app.add_event::<EventReadFromClipboard>();
        app.insert_resource(VeilidPluginStatus::Initializing);
    }
}

// -----
// Utils
// -----

#[cfg(target_arch = "wasm32")]
pub fn copy_to_clipboard(value: String, runtime: ResMut<TasksRutime>) {
    runtime.spawn_background_task(|mut ctx| async move {
        let window = web_sys::window().unwrap();
        let promise = window
            .navigator()
            .clipboard()
            .unwrap()
            .write_text(value.as_str());

        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_to_clipboard(value: String, runtime: ResMut<TasksRutime>) {
    // copy to clipboard
    let mut ctx = ClipboardContext::new().unwrap();
    let msg = format!("{}", value);
    ctx.set_contents(msg.to_owned()).unwrap();
    ctx.get_contents().unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
fn on_read_from_clipboard(
    mut ew: EventWriter<EventReadFromClipboardDone>,
    mut er: EventReader<EventReadFromClipboard>,
) {
    for _ in er.read() {
        let mut ctx = ClipboardContext::new().unwrap();
        let dht_key = ctx.get_contents().unwrap();

        ew.send(EventReadFromClipboardDone(dht_key.clone()));
    }
}

#[cfg(target_arch = "wasm32")]
fn on_read_from_clipboard(
    runtime: ResMut<TasksRutime>,
    mut ew: EventWriter<EventReadFromClipboardDone>,
    mut er: EventReader<EventReadFromClipboard>,
) {
    for _ in er.read() {
        runtime.spawn_background_task(|mut ctx| async move {
            let window = web_sys::window().unwrap();
            let promise = window.navigator().clipboard().unwrap().read_text();

            let result = wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
            let result = result.as_string().unwrap();

            ctx.run_on_main_thread(move |ctx| {
                let world = ctx.world;
                world.send_event(EventReadFromClipboardDone(result));
            })
            .await;
        });
    }
}
