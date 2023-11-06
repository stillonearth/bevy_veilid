use serde::{Deserialize, Serialize};

use belly::prelude::*;
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
// ---
// Network message
// ---
#[derive(Component)]
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
    show_connect_buttons: bool,
    counter: i32,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            titlebar_text: "Initializing Veilid".to_string(),
            error_text: "".to_string(),
            show_connect_buttons: false,
            counter: 0,
        }
    }
}

// ---
// Setup UI
// ---

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    let view_data = UIState::default();
    let label1 = commands.spawn_empty().insert(view_data.clone()).id();
    let label2 = commands.spawn_empty().insert(view_data.clone()).id();
    let label3: Entity = commands.spawn_empty().insert(view_data.clone()).id();
    commands.add(StyleSheet::parse(
        r#"
        .box {
            margin: 10px;
            padding: 10px;
        }
        .red {
            background-color: lightcoral;
        }
        .hidden {
            display: none;
        }
        .vbox {
            flex-direction: column;
        }
        .hbox {
            flex-direction: row;
        }
    "#,
    ));
    commands.add(eml! {
        <body s:padding="50px" c:vbox>
            <div c:vbox>
                <label {label1} bind:value=from!(label1, UIState:titlebar_text)/>
            </div>
            <div c:vbox>
                <label class="red" {label2} bind:value=from!(label2, UIState:error_text)/>
            </div>
            <div c:hbox>
                <div class="extra_buttons hidden">
                    <button on:press=|ctx| {ctx.send_event(EventHostGame)}>
                        "Copy DHT key to clipboard and host a game"
                    </button>
                </div>
                <div class="extra_buttons hidden">
                    <button on:press=|ctx| {ctx.send_event(EventJoinGame)}>
                        "Paste DHT key from clipboard and join a game"
                    </button>
                </div>
            </div>
            <div class="game" c:vbox>
                <div c:hbox>
                    <button on:press=|ctx| {ctx.send_event(EventHostGame)}>
                        "-"
                    </button>
                    <label bind:value=from!(label3, UIState:counter|fmt.c("Value: {c}"))/>
                    <button on:press=|ctx| {ctx.send_event(EventHostGame)}>
                        "+"
                    </button>
                </div>
            </div>
        </body>
    });
}

fn handle_ui_state(mut elements: Elements, mut view_data: Query<&mut UIState>) {
    for vd in view_data.iter_mut() {
        if vd.show_connect_buttons {
            elements.select(".extra_buttons").remove_class("hidden");
        } else {
            elements.select(".extra_buttons").add_class("hidden");
        }
    }
}

// ---
// Handle VeilidDuplex Events
// ---

fn on_ev_veilid_initialized(
    mut er_world_initialized: EventReader<VeilidInitializedEvent>,
    veilid_app: Res<VeilidApp>,
    mut view_data: Query<&mut UIState>,
) {
    for _ in er_world_initialized.iter() {
        let va = veilid_app.clone().unwrap();
        let status = format!("Veilid initialized!, dht_key: {}", va.our_dht_key);
        for mut vd in view_data.iter_mut() {
            vd.titlebar_text = status.clone();
            vd.show_connect_buttons = true;
        }
    }
}

fn on_ev_awating_peer(
    mut er_awaiting_peer: EventReader<EventAwaitingPeer>,
    mut view_data: Query<&mut UIState>,
) {
    for _ in er_awaiting_peer.iter() {
        // change ui state
        for mut vd in view_data.iter_mut() {
            vd.show_connect_buttons = false;
            vd.error_text = "".to_string();
            vd.titlebar_text = "Awaiting other peer".to_string();
        }
    }
}

fn on_ev_connected_peer(
    mut er_awaiting_peer: EventReader<EventConnectedPeer>,
    mut view_data: Query<&mut UIState>,
) {
    for _ in er_awaiting_peer.iter() {
        // change ui state
        for mut vd in view_data.iter_mut() {
            vd.show_connect_buttons = false;
            vd.titlebar_text = "Peer connected".to_string();
        }
    }
}

fn on_ev_error(
    mut er_veilid_error: EventReader<EventVeilidError>,
    mut view_data: Query<&mut UIState>,
) {
    for e in er_veilid_error.iter() {
        for mut d in view_data.iter_mut() {
            d.error_text = format!("{}", e.0);
        }
    }
}

fn on_ev_veilid_message(
    mut er_veilid_message: EventReader<EventReceiveMessage<SampleMessage>>,
    mut ew_connected_peer: EventWriter<EventConnectedPeer>,
) {
    for vm in er_veilid_message.iter() {
        if vm.message.extra == "START" {
            ew_connected_peer.send(EventConnectedPeer);
        }
    }
}

// ---
// Handle UI Events
// ---

fn on_join_game(
    mut er_host_game: EventReader<EventJoinGame>,
    mut ew_send_message: EventWriter<EventSendMessage<SampleMessage>>,
    mut ew_veilid_error: EventWriter<EventVeilidError>,
    mut ew_awaiting_peer: EventWriter<EventAwaitingPeer>,
) {
    for _ in er_host_game.iter() {
        // paste to clipboard
        let mut ctx = ClipboardContext::new().unwrap();
        let dht_key = ctx.get_contents().unwrap();
        let dht_key = crypto_key_from_str(dht_key);

        // send error event if input isn't ok
        if dht_key.is_err() {
            ew_veilid_error.send(EventVeilidError(dht_key.err().unwrap().into()));
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
    for _ in er_host_game.iter() {
        let va = veilid_app.clone().unwrap();
        // copy to clipboard
        let mut ctx = ClipboardContext::new().unwrap();
        let msg = format!("{}", va.our_dht_key);
        ctx.set_contents(msg.to_owned()).unwrap();
        // send event
        ew_awaiting_peer.send(EventAwaitingPeer);
    }
}

fn main() {
    // let default_env_filter = EnvFilter::try_from_default_env();
    // let fallback_filter =
    //     EnvFilter::new("veilid_core=error").add_directive("veilid_duplex=error".parse().unwrap());
    // let env_filter = default_env_filter.unwrap_or(fallback_filter);

    // tracing_subscriber::fmt()
    //     .with_writer(std::io::stderr)
    //     .with_env_filter(env_filter)
    //     .init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default())
        .add_plugins(VeilidPlugin::<SampleMessage>::default())
        .add_plugins(BellyPlugin)
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
            ),
        )
        .add_event::<EventHostGame>()
        .add_event::<EventJoinGame>()
        .add_event::<EventAwaitingPeer>()
        .add_event::<EventConnectedPeer>()
        .add_event::<EventVeilidError>()
        .insert_resource(VeilidPluginStatus::Initializing)
        .run();
}
