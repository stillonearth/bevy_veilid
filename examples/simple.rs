use serde::{Deserialize, Serialize};

use bevy::prelude::*;
use bevy_veilid::*;

use copypasta::*;
use veilid_duplex::utils::crypto_key_from_str;

// ---
// Events
// ---

#[derive(Event)]
struct EventHostGame;

#[derive(Event)]
struct EventJoinGame;

#[derive(Event)]
struct EventChangeCounter {
    pub delta: i32,
}

// ---
// Network message
// ---
#[derive(Serialize, Deserialize, Debug, Clone, Default, Resource)]
struct SampleMessage {
    pub counter: i32,
    pub extra: String,
}
// ---
// UI data
// ---

#[derive(Component, Clone)]
struct UIState {
    titlebar_text: String,
    error_text: String,
    counter: i32,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            titlebar_text: "Initializing Veilid".to_string(),
            error_text: "".to_string(),
            counter: 0,
        }
    }
}

// ---
// Setup UI
// ---

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    // let view_data = UIState::default();
    // let label1 = commands.spawn_empty().insert(view_data.clone()).id();
    // let label2 = commands.spawn_empty().insert(view_data.clone()).id();
    // let label3: Entity = commands.spawn_empty().insert(view_data.clone()).id();
}

fn handle_ui_state(// mut view_data: Query<&mut UIState>,
    // veilid_plugin_status: Res<VeilidPluginStatus>,
    // message: Res<SampleMessage>,
) {
    // let plugin_status = *veilid_plugin_status.into_inner();
    // for mut vd in view_data.iter_mut() {}
}

// ---
// Handle VeilidDuplex Events
// ---

fn on_ev_veilid_initialized(
    mut er_world_initialized: EventReader<EventVeilidInitialized>,
    veilid_app: Res<VeilidApp>,
    mut view_data: Query<&mut UIState>,
) {
    for _ in er_world_initialized.read() {
        let va = veilid_app.app.clone().unwrap();
        let status = format!("Veilid initialized!, dht_key: {}", va.our_dht_key);
        for mut vd in view_data.iter_mut() {
            vd.titlebar_text = status.clone();
        }

        info!("{}", status);
    }
}

fn on_ev_awating_peer(
    mut er_awaiting_peer: EventReader<EventAwaitingPeer>,
    mut view_data: Query<&mut UIState>,
) {
    for _ in er_awaiting_peer.read() {
        // change ui state
        for mut vd in view_data.iter_mut() {
            vd.error_text = "".to_string();
            vd.titlebar_text = "Awaiting other peer".to_string();

            println!("{}", vd.titlebar_text);
        }
    }
}

fn on_ev_connected_peer(
    mut er_awaiting_peer: EventReader<EventConnectedPeer>,
    mut view_data: Query<&mut UIState>,
) {
    for _ in er_awaiting_peer.read() {
        // change ui state
        for mut vd in view_data.iter_mut() {
            vd.titlebar_text = "Peer connected".to_string();
            println!("{}", vd.titlebar_text);
        }
    }
}

fn on_ev_error(mut er_veilid_error: EventReader<EventError>, mut view_data: Query<&mut UIState>) {
    for e in er_veilid_error.read() {
        for mut d in view_data.iter_mut() {
            d.error_text = format!("{}", e.0);
            println!("{}", d.error_text);
        }
    }
}

fn on_ev_veilid_message(
    mut er_veilid_message: EventReader<EventReceiveMessage<SampleMessage>>,
    mut ew_connected_peer: EventWriter<EventConnectedPeer>,
    mut message: ResMut<SampleMessage>,
) {
    for vm in er_veilid_message.read() {
        if vm.message.extra == "START" {
            ew_connected_peer.send(EventConnectedPeer {
                dht_key: vm.dht_key,
            });
        }

        message.counter = vm.message.counter;
    }
}

fn on_ev_change_counter(
    mut er_change_counter: EventReader<EventChangeCounter>,
    mut view_data: Query<&mut UIState>,
    mut message: ResMut<SampleMessage>,
    mut ew_send_message: EventWriter<EventSendMessage<SampleMessage>>,
    veilid_app: Res<VeilidApp>,
) {
    for e in er_change_counter.read() {
        message.counter += e.delta;
        // update viewdata
        for mut vd in view_data.iter_mut() {
            vd.counter = message.counter;
        }

        if veilid_app.other_peer_dht.is_none() {
            debug!("veilid_app.other_peer_dht is unset, unsure where to send message");
            return;
        }

        ew_send_message.send(EventSendMessage::new(
            message.clone(),
            veilid_app.other_peer_dht.unwrap(),
        ));
    }
}

// ---
// Handle UI Events
// ---

fn on_join_game(
    mut er_host_game: EventReader<EventJoinGame>,
    mut ew_send_message: EventWriter<EventSendMessage<SampleMessage>>,
    mut ew_veilid_error: EventWriter<EventError>,
    mut ew_awaiting_peer: EventWriter<EventAwaitingPeer>,
) {
    for _ in er_host_game.read() {
        // paste to clipboard
        let mut ctx = ClipboardContext::new().unwrap();
        let dht_key = ctx.get_contents().unwrap();
        let dht_key = crypto_key_from_str(dht_key);

        // send error event if input isn't ok
        if dht_key.is_err() {
            ew_veilid_error.send(EventError(dht_key.err().unwrap().into()));
            return;
        }
        // send "START" message to other peer to start a game
        ew_send_message.send(EventSendMessage::new(
            SampleMessage {
                counter: 0,
                extra: "START".to_string(),
            },
            dht_key.unwrap(),
        ));
        // send EventAwaitingPeer
        ew_awaiting_peer.send(EventAwaitingPeer);
    }
}

fn on_host_game(
    veilid_app: Res<VeilidApp>,
    mut er_host_game: EventReader<EventHostGame>,
    mut ew_awaiting_peer: EventWriter<EventAwaitingPeer>,
) {
    for _ in er_host_game.read() {
        let va = veilid_app.app.clone().unwrap();
        // copy to clipboard
        let mut ctx = ClipboardContext::new().unwrap();
        let msg = format!("{}", va.our_dht_key);
        ctx.set_contents(msg.to_owned()).unwrap();
        ctx.get_contents().unwrap();
        // send event
        ew_awaiting_peer.send(EventAwaitingPeer);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VeilidPlugin::<SampleMessage>::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                on_ev_veilid_initialized,
                handle_ui_state,
                on_host_game,
                on_join_game,
                on_ev_awating_peer,
                on_ev_error,
                on_ev_veilid_message,
                on_ev_connected_peer,
                on_ev_change_counter,
            ),
        )
        .add_event::<EventHostGame>()
        .add_event::<EventJoinGame>()
        .add_event::<EventChangeCounter>()
        .insert_resource(VeilidPluginStatus::Initializing)
        .insert_resource(SampleMessage::default())
        .run();
}
