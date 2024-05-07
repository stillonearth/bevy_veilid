# bevy_veilid

[![Crates.io](https://img.shields.io/crates/v/bevy_veilid.svg)](https://crates.io/crates/bevy_veilid)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/d/bevy_veilid.svg)](https://crates.io/crates/bevy_veilid)
[![Rust](https://github.com/stillonearth/bevy_veilid/workflows/CI/badge.svg)](https://github.com/stillonearth/bevy_veilid/actions)

## Build 2-Player turn-based p2p games with Bevy and Veilid

Build p2p turn-based games with anonimity for both clients with help of Veilid.

https://github.com/stillonearth/bevy_veilid/assets/97428129/4c505eef-1dee-4ab4-b0e7-51262a3b3337



## Compatibility

| bevy version | veilid version | bevy_veilid version |
| ------------ | :-------------:| :-----------------: |
| 0.11         |   0.2.4        | 0.1.0               |
| 0.11         |   0.2.5        | 0.1.2               |
| 0.12         |   0.3          | 0.2                 |
| 0.13         |   0.3          | 0.3                 |

## 📝Features

- Event-Based: read and send event to communicate with other peer
- Turn-Based: no tick synchronization
- Anonymous: each run creates a new persona 

## 👩‍💻 Usage

Refer to [examples/pingpong](examples/pingpong.rs) for basic example.

### 1. Define a message to send over network

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default, Resource)]
struct SampleMessage {
    pub counter: i32,
    pub extra: String,
}
```

### 2. Attach plugin to bevy

```rust
fn main() {

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VeilidPlugin::<SampleMessage>::default())
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
        .run();
}

```

### 3. Connect to systems

#### Events

* EventConnectedPeer
* EventError
* EventAwaitingPeer
* EventVeilidInitialized
* EventReceiveMessage<SampleMessage>
* EventSendMessage<SampleMessage>
* EventMessageSent

#### Resources

`bevy_veilid` will inject this into bevy

```rust
pub enum VeilidPluginStatus {
    Initializing,
    Initialized,
    ConnectedPeer,
    AwaitingPeer,
    Error,
}
```

## 💻 Under the hood

A full veilid instance will run in background with settings defined in [veilid_duplex](https://gitlab.com/cwiz/veilid_duplex). 
`veilid_duplex` manages veilid internals and provides an API to send a message to another peer by refering each other with dht_keys unique for each run.

## Examples

1. [passing message with increment / decriment](https://github.com/stillonearth/bevy_veilid/blob/main/examples/pingpong.rs)
2. [checkers on bevy](https://github.com/stillonearth/CheckersOnBevy](https://github.com/stillonearth/CheckersOnBevy/)https://github.com/stillonearth/CheckersOnBevy)
