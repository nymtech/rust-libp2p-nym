// Copyright TODO based on the rust libp2p examples check how to smush 2 together / if this is necessary

use futures::stream::StreamExt;
use libp2p::{
    gossipsub,
    swarm::{NetworkBehaviour, SwarmEvent},
    PeerId,
};
use libp2p::{Multiaddr, SwarmBuilder};
use libp2p_identity::Keypair;
use log::{debug, info, warn, LevelFilter};
use rust_libp2p_nym::transport::NymTransport;
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    error::Error,
    hash::{Hash, Hasher},
    time::Duration,
};
use tokio::{io, io::AsyncBufReadExt, select, time::sleep};

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Modify these values for different verbosity depending on how granularly you want to follow traffic in & out of your local client
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Info)
        .filter_module("rust_libp2p_nym", LevelFilter::Debug)
        .filter_module("libp2p_gossipsub", LevelFilter::Info)
        .filter_module("libp2p_swarm", LevelFilter::Debug)
        .init();

    let local_key = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    info!("Starting chat example with PeerId: {}", local_peer_id);
    info!("Running `chat` example using NymTransport");

    info!("Connecting to Nym mixnet...");
    let client = match nym_sdk::mixnet::MixnetClient::connect_new().await {
        Ok(client) => {
            info!("Successfully connected to Nym mixnet");
            client
        }
        Err(e) => {
            warn!("Failed to connect to Nym mixnet: {}", e);
            return Err(e.into());
        }
    };

    let transport = NymTransport::new_with_timeout(
        client,
        local_key.clone(),
        Duration::from_secs(90), // Increased timeout for protocol negotiation over mixnet
    )
    .await?;

    info!("Building swarm...");
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_other_transport(|_| transport)?
        .with_behaviour(|key| {
            // To content-address message, we can take the hash of message and use it as an ID.
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(40))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .max_transmit_size(65536)
                .duplicate_cache_time(Duration::from_secs(60))
                .mesh_n(1)
                .mesh_n_low(1)
                .mesh_n_high(14)
                .mesh_outbound_min(0)
                .gossip_lazy(6)
                .fanout_ttl(Duration::from_secs(60))
                .support_floodsub()
                .flood_publish(true)
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            Ok(MyBehaviour { gossipsub })
        })?
        .with_swarm_config(|c| {
            c.with_idle_connection_timeout(Duration::from_secs(120)) // Timeout increases across the board
                .with_max_negotiating_inbound_streams(50)
        })
        .build();

    info!("Swarm built successfully");

    // Create Gossipsub topic
    let topic = gossipsub::IdentTopic::new("nym-transport-test");
    info!("Created topic: {}", topic);
    let mut subscription_attempts = 0;
    let max_attempts = 3;

    loop {
        match swarm.behaviour_mut().gossipsub.subscribe(&topic) {
            Ok(true) => {
                info!(
                    "✅ Successfully subscribed to topic on attempt {}",
                    subscription_attempts + 1
                );
                break;
            }
            Ok(false) => {
                warn!("⚠️ Already subscribed to topic");
                break;
            }
            Err(e) => {
                subscription_attempts += 1;
                if subscription_attempts >= max_attempts {
                    warn!(
                        "❌ Failed to subscribe after {} attempts: {}",
                        max_attempts, e
                    );
                    break;
                }
                warn!(
                    "⚠️ Subscription attempt {} failed: {}, retrying...",
                    subscription_attempts, e
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }

    // Subscribe to our topic
    match swarm.behaviour_mut().gossipsub.subscribe(&topic) {
        Ok(subscribed) => {
            info!(
                "Successfully subscribed to topic: {}, subscribed: {}",
                topic, subscribed
            );
        }
        Err(e) => {
            warn!("Failed to subscribe to topic: {}", e);
            return Err(e.into());
        }
    }

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Track peers and their subscriptions - compare against Heartbeat debug logging, something not quite synced yet..
    let mut connected_peers: HashSet<PeerId> = HashSet::new();
    let mut subscribed_peers = HashSet::new();
    let mut ready_to_chat = false;

    info!("Enter messages via STDIN and they will be sent to connected peers using Gossipsub");
    info!("Note: Wait for 'Ready to chat!' message before sending messages");

    // Handle command line argument for dialing
    if let Some(addr) = std::env::args().nth(1) {
        info!("Attempting to dial: {}", addr);
        let remote: Multiaddr = match addr.parse() {
            Ok(addr) => addr,
            Err(e) => {
                warn!("Failed to parse multiaddr '{}': {}", addr, e);
                return Err(e.into());
            }
        };

        match swarm.dial(remote.clone()) {
            Ok(_) => info!("Initiated dial to {}", remote),
            Err(e) => {
                warn!("Failed to initiate dial to {}: {}", remote, e);
                return Err(e.into());
            }
        }
    } else {
        info!("No remote address provided, waiting for incoming connections");
        info!("To connect to this node, run:");
        info!("cargo run --example chat -- /nym/YOUR_ADDRESS_HERE");
    }

    let mut status_interval = tokio::time::interval(Duration::from_secs(30));

    // Kick it off
    loop {
        select! {
            // _ = status_interval.tick() => {
            //     let connected_count = connected_peers.len();
            //     let subscribed_count = subscribed_peers.len();
            //     info!("Status: {} connected peers, {} subscribed to topic", connected_count, subscribed_count);

            //     if connected_count > 0 && subscribed_count > 0 && !ready_to_chat {
            //         ready_to_chat = true;
            //         info!("🎉 Ready to chat! You can now send messages.");
            //     }

            //     if connected_count > 0 {
            //         for peer in &connected_peers {
            //             let is_subscribed = subscribed_peers.contains(peer);
            //             info!("Peer {}: subscribed={}", peer, is_subscribed);
            //         }
            //     }
            // }
            //

            _ = status_interval.tick() => {
                let connected_count = connected_peers.len();
                let subscribed_count = subscribed_peers.len();
                info!("📊 Status: {} connected, {} subscribed", connected_count, subscribed_count);

                let gossipsub_all_peers: Vec<_> = swarm.behaviour().gossipsub.all_peers().collect();
                let topic_hash = topic.hash();
                let gossipsub_mesh_peers: Vec<_> = swarm.behaviour().gossipsub.mesh_peers(&topic_hash).collect();
                let gossipsub_topics: Vec<_> = swarm.behaviour().gossipsub.topics().collect();

                info!("🕸️ Gossipsub internal state:");
                info!("  All peers known to gossipsub: {}", gossipsub_all_peers.len());
                info!("  Peers in mesh for topic: {}", gossipsub_mesh_peers.len());
                info!("  Topics we know about: {}", gossipsub_topics.len());

                // Show what gossipsub knows about each peer
                for (i, (peer_id, topic_hashes)) in gossipsub_all_peers.iter().enumerate() {
                    let peer_short = peer_id.to_string().chars().take(12).collect::<String>();
                    info!("  Gossipsub peer {}: {} (subscribed to {} topics)",
                          i+1, peer_short, topic_hashes.len());
                }

                for (i, peer_id) in gossipsub_mesh_peers.iter().enumerate() {
                    let peer_short = peer_id.to_string().chars().take(12).collect::<String>();
                    info!("  Mesh peer {}: {}", i+1, peer_short);
                }

                for peer in &connected_peers {
                    let is_subscribed = subscribed_peers.contains(peer);
                    let peer_short = peer.to_string().chars().take(12).collect::<String>();
                    info!("  👤 {}: connected=✅ subscribed={}",
                          peer_short, if is_subscribed { "✅" } else { "❌" });
                }

                if gossipsub_mesh_peers.is_empty() && !gossipsub_all_peers.is_empty() {
                    warn!("🚨 MESH PROBLEM: Gossipsub knows about {} peers but mesh is empty!",
                          gossipsub_all_peers.len());
                }

                if connected_peers.len() != gossipsub_all_peers.len() {
                    warn!("🚨 SYNC PROBLEM: libp2p sees {} connected peers but gossipsub sees {}",
                           connected_peers.len(), gossipsub_all_peers.len());
                }

                if connected_count > 0 && subscribed_count > 0 && !ready_to_chat {
                    ready_to_chat = true;
                    info!("🎉 Ready to chat! You can now send messages.");
                }
            }


            Ok(Some(line)) = stdin.next_line() => {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                if !ready_to_chat {
                    warn!("Not ready to chat yet. Wait for peer subscriptions...");
                    continue;
                }

                info!("📤 Publishing message: '{}'", line);
                match swarm.behaviour_mut().gossipsub.publish(topic.clone(), line.as_bytes()) {
                    Ok(message_id) => {
                        info!("✅ Published message with ID: {}", message_id);
                    }
                    Err(gossipsub::PublishError::InsufficientPeers) => {
                        warn!("❌ Not enough peers subscribed to the topic yet. Wait a moment...");
                        ready_to_chat = false; // Reset the flag to wait for proper subscription
                    }
                    Err(e) => {
                        warn!("❌ Publish error: {:?}", e);
                    }
                }
            }

            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, established_in, .. } => {
                        info!("🔗 Connection established with peer: {} (endpoint: {:?}, established in: {:?})",
                              peer_id, endpoint, established_in);
                        connected_peers.insert(peer_id);

                        // Give some time for gossipsub to exchange subscription info
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(4)).await;
                            debug!("Connection stabilization period completed for {}", peer_id);
                        });
                    }

                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        info!("❌ Connection closed with peer: {} (cause: {:?})", peer_id, cause);
                        connected_peers.remove(&peer_id);
                        subscribed_peers.remove(&peer_id);
                        if connected_peers.is_empty() {
                            ready_to_chat = false;
                        }
                    }

                    SwarmEvent::IncomingConnection { local_addr, send_back_addr, .. } => {
                        info!("📞 Incoming connection from {} to {}", send_back_addr, local_addr);
                    }

                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        warn!("💥 Outgoing connection error to {:?}: {}", peer_id, error);
                    }

                    SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
                        warn!("💥 Incoming connection error from {} to {}: {}", send_back_addr, local_addr, error);
                    }

                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!("🎧 Local node is listening on {}", address);
                        info!("📋 To connect to this node, use: {}", address);
                    }

                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic })) => {
                        info!("📝 Peer {} subscribed to topic: {}", peer_id, topic);
                        subscribed_peers.insert(peer_id);

                        // Check if we're ready to chat
                        if connected_peers.contains(&peer_id) && !ready_to_chat {
                            ready_to_chat = true;
                            info!("🎉 Ready to chat! You can now send messages.");
                        }
                    }

                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed { peer_id, topic })) => {
                        info!("❌ Peer {} unsubscribed from topic: {}", peer_id, topic);
                        subscribed_peers.remove(&peer_id);
                        if subscribed_peers.is_empty() {
                            ready_to_chat = false;
                        }
                    }

                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: _id,
                        message,
                    })) => {
                        let msg_str = String::from_utf8_lossy(&message.data);
                        info!("📨 Message from {}: '{}'", peer_id, msg_str);
                        println!("\n💬 [{}]: {}\n", peer_id.to_string().chars().take(12).collect::<String>(), msg_str);
                    }

                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::GossipsubNotSupported { peer_id })) => {
                        warn!("⚠️  Peer {} does not support gossipsub", peer_id);
                    }

                    _ => {
                        debug!("Other swarm event: {:?}", event);
                    }
                }
            }
        }
    }
}
