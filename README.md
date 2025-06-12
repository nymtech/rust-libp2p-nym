# rust-libp2p-nym

This repo contains an active work in progress implementation of a libp2p transport using the Nym SDK. It a fork of the work done by [Chainsafe](https://github.com/ChainSafe/rust-libp2p-nym) several years ago, updated with some extra Nym-specific anonymity and unlinkability properties.

## Usage

To instantiate a libp2p swarm using the transport:

```rust
use futures::prelude::*;
use libp2p::SwarmBuilder;
use libp2p::{ping, swarm::SwarmEvent, Multiaddr};
use libp2p_identity::{Keypair, PeerId};
use rust_libp2p_nym::transport::NymTransport;
use std::error::Error;

let local_key = Keypair::generate_ed25519();
let local_peer_id = PeerId::from(local_key.public());
println!("Local peer id: {local_peer_id:?}");

let mut swarm = {
    println!("Running `ping` example using NymTransport");
    let client = nym_sdk::mixnet::MixnetClient::connect_new().await?;
    let transport = NymTransport::new(client, local_key.clone()).await?;

    SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_other_transport(|_| transport)?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(20)))
        .build()
};
```

See `examples/ping.rs` and `examples/chat.rs` for fuller usage examples (instructions below).

## Tests

Install `protoc`.

```
# Ubuntu/Debian
apt-get install protobuf-compiler
# Arch/Manjaro
yay protobuf
```

Test with the standard `cargo` command:

```
cargo test
```


## Chat example
You can either grab multiaddr from someone else sharing it out of band, or run the chat in two terminal windows, and loop traffic through the mixnet between two local clients. If you are using an address from someone else, just run the second step of the below instructions.

### ProtocolBerg Workshop Chat
To join, run the following to connect to the workshop peer:
```
cargo run --example chat -- /nym/3NRXaVgffaa7dEfn4BpNhBRZ7rf4bjdtJxtS3XPeQF1s.HXxt1bJ2LAKMSFBCh9RNzKi8F46rSF2mhbPLorRt3wMt@9PG6vqoVniK7bWD7esueje9pD3P3iU3Md1T8FAuNQipW
```

### Local ChaT
```
# Terminal window 1
cargo run --example chat
# this will log the listen address of the listener peer:
2025-06-12T07:14:09.499Z INFO  chat > 📋 To connect to this node, use: /nym/B9SXzkbeTbg22bC1NEiHE9dBA7BUDeXzEdH5BVYzGjdF.FwMChmPoowr18k1tTcgY4HTrKJLp8vofVtQKUKsjZGAN@mD6g3NYiWxkQSpVSJx7KbRjFgbSXkRn7zL6Mtq1bcvC

# Terminal window 2
cargo run --example chat -- /nym/B9SXzkbeTbg22bC1NEiHE9dBA7BUDeXzEdH5BVYzGjdF.FwMChmPoowr18k1tTcgY4HTrKJLp8vofVtQKUKsjZGAN@mD6g3NYiWxkQSpVSJx7KbRjFgbSXkRn7zL6Mtq1bcvC
```

You will have to wait until the connection upgrade is complete. If running with logging in `DEBUG` mode you will see a lot of back and forth messages between both cliente before the protocol negotiation is complete and you  are able tO send messages between them.

Once the protocol negotiation iS complete and the connection substreams are established, the example captures `STDIN` and pipes it to the other clients subscribed to the gossipsub topic.

If you connect multiple clients you will see status / mesh logging like:

```
2025-06-12T07:23:11.908Z INFO  chat                                 > 📊 Status: 5 connected, 5 subscribed
2025-06-12T07:23:11.908Z INFO  chat                                 > 🕸️ Gossipsub internal state:
2025-06-12T07:23:11.908Z INFO  chat                                 >   All peers known to gossipsub: 5
2025-06-12T07:23:11.908Z INFO  chat                                 >   Peers in mesh for topic: 5
2025-06-12T07:23:11.908Z INFO  chat                                 >   Topics we know about: 1
2025-06-12T07:23:11.908Z INFO  chat                                 >   Gossipsub peer 1: 12D3KooWGtvK (subscribed to 1 topics)
2025-06-12T07:23:11.908Z INFO  chat                                 >   Gossipsub peer 2: 12D3KooWPDBY (subscribed to 1 topics)
2025-06-12T07:23:11.908Z INFO  chat                                 >   Gossipsub peer 3: 12D3KooWHnoH (subscribed to 1 topics)
2025-06-12T07:23:11.908Z INFO  chat                                 >   Gossipsub peer 4: 12D3KooWHUKF (subscribed to 1 topics)
2025-06-12T07:23:11.908Z INFO  chat                                 >   Gossipsub peer 5: 12D3KooWKVjr (subscribed to 1 topics)
2025-06-12T07:23:11.908Z INFO  chat                                 >   Mesh peer 1: 12D3KooWGtvK
2025-06-12T07:23:11.908Z INFO  chat                                 >   Mesh peer 2: 12D3KooWHUKF
2025-06-12T07:23:11.908Z INFO  chat                                 >   Mesh peer 3: 12D3KooWHnoH
2025-06-12T07:23:11.908Z INFO  chat                                 >   Mesh peer 4: 12D3KooWKVjr
2025-06-12T07:23:11.908Z INFO  chat                                 >   Mesh peer 5: 12D3KooWPDBY
2025-06-12T07:23:11.909Z INFO  chat                                 >   👤 12D3KooWHnoH: connected=✅ subscribed=✅
2025-06-12T07:23:11.909Z INFO  chat                                 >   👤 12D3KooWHUKF: connected=✅ subscribed=✅
2025-06-12T07:23:11.909Z INFO  chat                                 >   👤 12D3KooWPDBY: connected=✅ subscribed=✅
2025-06-12T07:23:11.909Z INFO  chat                                 >   👤 12D3KooWKVjr: connected=✅ subscribed=✅
2025-06-12T07:23:11.909Z INFO  chat                                 >   👤 12D3KooWGtvK: connected=✅ subscribed=✅
```

Currently this information is done manaully as standard peer discovery mechanisms such as the Kademlia DHT are not yet adapted for Nym (but are on the development roadmap).

If you wish to modify the verbosity of the logging, change `DEBUG` to `INFO` in the `pretty_env_logger::formatted_timed_builder()` starting on LN29.

## Ping example
```
# Terminal window 1
cargo run --example ping
# this will log something like Listening on /nym/4Gf3CkYhc8tYzLsyWwboGVDgcVX9WHUrtbYtdb1Y5YiA.9n5XxwvyUuL9GVfFS9mwawSnG3hvaitDKq7HT8bMHTJb@C7J8SwZQqjWqhBryyjJxLt7FacVuPTwAmR2otGy53ayi - copy that /nym/ multiaddr

# Terminal window 2
cargo run --example ping -- $multiaddr_from_clipboard
```

You will have to wait until the connection upgrade is complete, so you will see some back and forth messages between both clients before seeing any `ping` logging in the console.
