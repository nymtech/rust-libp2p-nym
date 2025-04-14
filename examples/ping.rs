// Copyright TODO based on the rust libp2p examples check how to smush 2 together / if this is necessary

use futures::prelude::*;
use libp2p::SwarmBuilder;
use libp2p::{ping, swarm::SwarmEvent, Multiaddr};
use libp2p_identity::{Keypair, PeerId};
use log::LevelFilter;
use nym_sdk::mixnet::{MixnetClientBuilder, StoragePaths};
use rust_libp2p_nym::transport::NymTransport;
use std::path::PathBuf;
use std::{error::Error, time::Duration};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Info)
        .filter(Some("libp2p_ping"), LevelFilter::Debug)
        .init();

    let local_key = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id:?}");

    let mut swarm = {
        println!("Running `ping` example using NymTransport");
        let config_dir = PathBuf::from(TempDir::new().unwrap().path().to_str().unwrap());
        let storage_paths = StoragePaths::new_from_dir(&config_dir).unwrap();

        // Create the client with a storage backend, and enable it by giving it some paths. If keys
        // exists at these paths, they will be loaded, otherwise they will be generated.
        let client = MixnetClientBuilder::new_with_default_storage(storage_paths)
            .await
            .unwrap()
            .build()
            .unwrap();

        let client = client.connect_to_mixnet().await.unwrap();

        let transport = NymTransport::new(client, local_key.clone()).await?;

        SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_other_transport(|_| transport)?
            .with_behaviour(|_| ping::Behaviour::default())?
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(90))) // TODO this sets the config timeout for the ping example - change for keepalive behaviour if possible
            .build()
    };

    // Dial the peer identified by the multi-address given as the second
    // command-line argument, if any.
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Dialed {addr}")
    }

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
            SwarmEvent::Behaviour(event) => println!("{event:?}"),
            _ => {}
        }
    }
}
