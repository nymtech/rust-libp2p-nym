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

Test with the standard `cargo` command:

```
cargo test
```

## Ping example
```
# Terminal window 1 
cargo run --example ping
# this will log something like Listening on /nym/4Gf3CkYhc8tYzLsyWwboGVDgcVX9WHUrtbYtdb1Y5YiA.9n5XxwvyUuL9GVfFS9mwawSnG3hvaitDKq7HT8bMHTJb@C7J8SwZQqjWqhBryyjJxLt7FacVuPTwAmR2otGy53ayi - copy that /nym/ multiaddr 

# Terminal window 2 
cargo run --example ping -- $multiaddr_from_clipboard 
```

You will have to wait until the connection upgrade is complete, so you will see some back and forth messages between both clients before seeing any `ping` logging in the console. 

## Chat example 
```
# Terminal window 1 
cargo run --example chat  
# this will log something like Listening on /nym/4Gf3CkYhc8tYzLsyWwboGVDgcVX9WHUrtbYtdb1Y5YiA.9n5XxwvyUuL9GVfFS9mwawSnG3hvaitDKq7HT8bMHTJb@C7J8SwZQqjWqhBryyjJxLt7FacVuPTwAmR2otGy53ayi - copy that /nym/ multiaddr 

# Terminal window 2 
cargo run --example chat -- $multiaddr_from_clipboard 
```

You will have to wait until the connection upgrade is complete, so you will see some back and forth messages between both clients before being able to send messages between them. Once the upgrade is complete and the connection is established, the example captures `STDIN` and pipes it to the other client. 
