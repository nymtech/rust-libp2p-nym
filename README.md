# rust-libp2p-nym

This repo contains an implementation of a libp2p transport using the Nym SDK.

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

See `examples/ping.rs` and `examples/chat.rs` for full usage examples such as setting `Behaviour`.

## Tests

Install `protoc`.

```
# Ubuntu/Debian
apt-get install protobuf-compiler
# Arch/Manjaro
yay protobuf
```

Standard `cargo` command:

```
cargo test
```

## Ping example
TODO once SURBs added

## Chat example
TODO once SURBs added
