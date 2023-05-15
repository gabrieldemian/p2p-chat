use clap::Parser;
use libp2p::{
    core::upgrade,
    futures::StreamExt,
    gossipsub::{self, IdentTopic},
    identity::Keypair,
    kad::{store::MemoryStore, Kademlia, KademliaEvent},
    mdns,
    multiaddr::Protocol,
    swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp::{self, Config},
    yamux, Multiaddr, PeerId, Swarm, Transport,
};
use libp2p_noise as noise;
use log::info;
use std::time::Duration;
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};

use crate::app::AppMessage;

use super::cli::Opt;

// defines the behaviour of the current peer
// on the network
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NetworkMessage")]
pub struct AppBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
pub enum NetworkMessage {
    Dial(Multiaddr),
    Kademlia(KademliaEvent),
    Gossipsub(gossipsub::Event),
    Mdns(mdns::Event),

    Quit,
    MessageReceived(IdentTopic, String),
    Subscribed(IdentTopic),
}

impl From<KademliaEvent> for NetworkMessage {
    fn from(event: KademliaEvent) -> Self {
        NetworkMessage::Kademlia(event)
    }
}

impl From<gossipsub::Event> for NetworkMessage {
    fn from(event: gossipsub::Event) -> Self {
        NetworkMessage::Gossipsub(event)
    }
}

impl From<mdns::Event> for NetworkMessage {
    fn from(event: mdns::Event) -> Self {
        NetworkMessage::Mdns(event)
    }
}

pub struct Network {
    pub peer_id: PeerId,
    pub swarm: Swarm<AppBehaviour>,
    pub event_receiver: Receiver<NetworkMessage>,
    pub event_sender: Sender<NetworkMessage>,
}

impl Network {
    pub fn new(tx: Sender<NetworkMessage>, rx: Receiver<NetworkMessage>) -> Self {
        // get the object representing the CLI flags
        let opt = Opt::parse();
        // generate the peer public key (peerId)
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();

        // instantiate the TCP protocol, with noise cryptography
        // and multiplexed
        let transport_config = Config::new().port_reuse(true);
        let transport = tcp::tokio::Transport::new(transport_config)
            .upgrade(upgrade::Version::V1Lazy)
            .authenticate(
                noise::Config::new(&keypair)
                    .expect("Signing libp2p-noise static DH keypair failed."),
            )
            .multiplex(yamux::Config::default())
            .boxed();

        // the message authenticity - How we expect to publish messages
        // the publisher will sign the message with his key
        let message_authenticity = gossipsub::MessageAuthenticity::Signed(keypair.clone());

        // protocol - kademlia
        // this is not being used at the moment.
        let kademlia = Kademlia::new(peer_id, MemoryStore::new(peer_id));

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id).unwrap();

        // protocol - gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
            .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
            .build()
            .expect("Valid config");

        let gossipsub = gossipsub::Behaviour::new(message_authenticity, gossipsub_config)
            .expect("could not create gossipsub interface");

        // swarm manages all events, events, and protocols
        let mut swarm = {
            let behaviour = AppBehaviour {
                gossipsub,
                kademlia,
                mdns,
            };
            SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build()
        };

        let multiaddr: Multiaddr = match &opt.listen_address {
            Some(addr) => addr.clone(),
            None => "/ip4/0.0.0.0/tcp/0".parse().expect("address to be valid"),
        };

        // this peer will listen to events on the network
        swarm
            .listen_on(multiaddr)
            .expect("could not listen on swarm");

        Self {
            swarm,
            peer_id,
            event_receiver: rx,
            event_sender: tx,
        }
    }

    pub async fn daemon<'a>(&mut self, tx_app: Sender<AppMessage<'a>>) -> () {
        loop {
            select! {
                event = self.event_receiver.recv() => {
                    match event.unwrap() {
                        NetworkMessage::Dial(addr) => {
                            let peer_id = match addr.iter().last() {
                                Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
                                _ => return ()
                            };
                            self.swarm.dial(addr.clone()).expect("to call addr");
                            self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                        },
                        NetworkMessage::Kademlia(e) => {info!("unhandled {:#?}", e)},
                        NetworkMessage::MessageReceived(topic, message) => {
                            if let Err(e) =
                                self.swarm.behaviour_mut()
                                .gossipsub.publish(topic, message.as_bytes())
                            {
                                info!("could not send msg from daemon {:?}", e);
                            };
                        },
                        NetworkMessage::Quit => return (),
                        NetworkMessage::Subscribed(topic) => {
                            info!("subscribed ??");
                            self.swarm.behaviour_mut().gossipsub
                                .subscribe(&topic)
                                .expect("could not subscribe to topic");
                        },
                        _ => {info!("not handled kademlia event received")}
                    };
                },
                swarm_event = self.swarm.select_next_some() => match swarm_event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(
                            "Local node is listening on {:?}",
                            address.with(Protocol::P2p(self.peer_id.into()))
                        );
                        let opt = Opt::parse();
                        if let Some(addr) = &opt.peer {
                            self.event_sender
                                .send(NetworkMessage::Dial(addr.clone())).await.unwrap();
                        };
                    },
                    SwarmEvent::Behaviour(NetworkMessage::Kademlia(_e)) => {
                        // info!("Received kademlia event {:#?}", e);
                    },
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        if endpoint.is_dialer() {
                            info!("Connection established - peerId: {peer_id}");
                        }
                    }
                    SwarmEvent::Dialing(peer_id) => info!("Dialing {peer_id}"),
                    SwarmEvent::Behaviour(NetworkMessage::Gossipsub(gossipsub::Event::Subscribed {
                        peer_id,
                        topic,
                    })) => {
                        info!(
                            "{peer_id} subscribed to {topic}"
                        );
                    }
                    SwarmEvent::Behaviour(NetworkMessage::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message,
                        ..
                    })) => {
                        let peer_id = peer_id.to_string();
                        let peer_id = peer_id[peer_id.len() - 7..].to_string();

                        let message = format!(
                            "{peer_id}: {}",
                            String::from_utf8_lossy(&message.data),
                        );

                        info!("got msg {message}");

                        tx_app.send(AppMessage::MessageReceived{ message }).await.unwrap();
                    },
                    SwarmEvent::Behaviour(NetworkMessage::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, _multiaddr) in list {
                            // info!("discovered new peer {peer_id}");
                            self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    },
                    SwarmEvent::Behaviour(NetworkMessage::Mdns(mdns::Event::Expired(list))) => {
                        for (peer_id, _multiaddr) in list {
                            self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        }
                    },
                    _ => {}
                },
            };
        }
    }
}
