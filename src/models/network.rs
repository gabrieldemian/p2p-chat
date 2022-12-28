use async_std::io;
use async_std::io::prelude::BufReadExt;
use clap::Parser;
use libp2p::{
    core::upgrade,
    futures::StreamExt,
    gossipsub::{Gossipsub, GossipsubConfig, GossipsubEvent, IdentTopic, MessageAuthenticity},
    identity::Keypair,
    kad::{store::MemoryStore, Kademlia, KademliaEvent},
    mplex,
    multiaddr::Protocol,
    noise::NoiseAuthenticated,
    swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp::{self, Config},
    Multiaddr, PeerId, Swarm, Transport,
};
use log::{error, info};
use tokio::{
    select,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

use super::{cli::Opt, utils::utils::draw_cowsay};

// defines the behaviour of the current peer
// on the network
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event")]
pub struct AppBehaviour {
    pub gossipsub: Gossipsub,
    pub kademlia: Kademlia<MemoryStore>,
}

#[derive(Debug)]
pub enum Event {
    Dial(Multiaddr),
    Kademlia(KademliaEvent),
    Gossipsub(GossipsubEvent),
}

impl From<KademliaEvent> for Event {
    fn from(event: KademliaEvent) -> Self {
        Event::Kademlia(event)
    }
}

impl From<GossipsubEvent> for Event {
    fn from(event: GossipsubEvent) -> Self {
        Event::Gossipsub(event)
    }
}

pub struct Network {
    pub peer_id: PeerId,
    pub topic: IdentTopic,
    pub swarm: Swarm<AppBehaviour>,
    pub event_sender: UnboundedSender<Event>,
    pub event_receiver: UnboundedReceiver<Event>,
}

impl Network {
    pub async fn new() -> Self {
        // get the object representing the CLI flags
        let opt = Opt::parse();
        // generate the peer public key (peerId)
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();

        // instantiate the TCP protocol, with noise cryptography
        // and multiplexed
        let transport_config = Config::new().port_reuse(true);
        let transport = tcp::tokio::Transport::new(transport_config)
            .upgrade(upgrade::Version::V1)
            .authenticate(
                NoiseAuthenticated::xx(&keypair)
                    .expect("Signing libp2p-noise static DH keypair failed."),
            )
            .multiplex(mplex::MplexConfig::new())
            .boxed();

        // the message authenticity - How we expect to publish messages
        // the publisher will sign the message with his key
        let message_authenticity = MessageAuthenticity::Signed(keypair.clone());

        // protocol - kademlia
        // this is not being used at the moment.
        let kademlia = Kademlia::new(peer_id, MemoryStore::new(peer_id));

        // protocol - gossipsub
        let gossipsub_config = GossipsubConfig::default();
        let mut gossipsub = Gossipsub::new(message_authenticity, gossipsub_config)
            .expect("could not create gossipsub");

        let topic = IdentTopic::new("secret-room");

        gossipsub
            .subscribe(&topic)
            .expect("could not subscribe to topic");

        // swarm manages all events, events, and protocols
        let mut swarm = {
            let behaviour = AppBehaviour {
                gossipsub,
                kademlia,
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

        let (event_sender, event_receiver) = mpsc::unbounded_channel::<Event>();

        Self {
            swarm,
            topic,
            peer_id,
            event_sender,
            event_receiver,
        }
    }

    pub async fn daemon(&mut self) {
        // Read full lines from stdin
        let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();
        let msg = concat!(
            "To start sending messages, you first need to know your friend multiaddr. ",
            "Look for a log that starts with \"/ip4/192...\" and send to your friend.\n",
            "1. Alice - listen for events: RUST_LOG=info cargo run\n",
            "2. Bob - dial Bob multiaddr: RUST_LOG=info cargo run -- --peer /ip4/x.x.x.x/tcp/xxxxx\n",
            "Now they are connected and can start sending messages on the terminal."
        );
        draw_cowsay(msg.to_string());

        loop {
            select! {
                line = stdin.select_next_some() => {
                    if let Err(e) = self.swarm
                        .behaviour_mut().gossipsub
                        .publish(
                            self.topic.clone(),
                            line.expect("Stdin not to close").as_bytes()
                        )
                        {
                            println!("Publish error: {e:?}");
                        }
                },
                event = self.event_receiver.recv() => {
                    match event.unwrap() {
                        Event::Gossipsub(event) => {
                            match event {
                                GossipsubEvent::Subscribed{peer_id, topic} => {
                                    info!(
                                        "{peer_id} subscribed to {topic}"
                                    )
                                },
                                GossipsubEvent::Message{propagation_source: peer_id, message, ..} => {
                                    let peer_id = peer_id.to_string();
                                    let peer_id = peer_id[peer_id.len() - 7..].to_string();
                                    println!(
                                        "{peer_id}: {}",
                                        String::from_utf8_lossy(&message.data),
                                    )
                                },
                                GossipsubEvent::Unsubscribed{..} => {},
                                GossipsubEvent::GossipsubNotSupported{..} => {
                                    error!("Gossipsub is not supported.")
                                },
                            }
                        },
                        Event::Kademlia(_event) => {},
                        Event::Dial(addr) => {
                            let peer_id = match addr.iter().last() {
                                Some(Protocol::P2p(hash)) => {
                                    info!("Dialing: {addr}");
                                    PeerId::from_multihash(hash).expect("Valid hash.")
                                }
                                _ => return ()
                            };
                            self.swarm.dial(addr.clone()).expect("to call addr");
                            self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    };
                },
                swarm_event = self.swarm.select_next_some() => match swarm_event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(
                            "Local node is listening on {:?}",
                            address.with(Protocol::P2p(self.peer_id.into()))
                        );
                        let opt = Opt::parse();

                        self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&self.peer_id);

                        if let Some(addr) = &opt.peer {
                            info!("dialing {addr}");
                            self.event_sender
                                .send(Event::Dial(addr.clone()))
                                .expect("to send dial event on mpsc");
                        };
                    },
                    SwarmEvent::Behaviour(Event::Kademlia(e)) => {
                        info!("Received kademlia event {:#?}", e);
                    },
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        if endpoint.is_dialer() {
                            info!("Connection established - peerId: {peer_id}");
                        }
                    }
                    _ => {}
                },
            };
        }
    }
}
