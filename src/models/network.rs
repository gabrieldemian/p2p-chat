use libp2p_noise as noise;
use async_std::io;
use async_std::io::prelude::BufReadExt;
use clap::Parser;
use libp2p::{
    core::upgrade,
    futures::StreamExt,
    gossipsub::{self, IdentTopic},
    identity::Keypair,
    kad::{store::MemoryStore, Kademlia, KademliaEvent},
    yamux,
    multiaddr::Protocol,
    swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp::{self, Config},
    Multiaddr, PeerId, Swarm, Transport,
};
use log::info;
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
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
}

#[derive(Debug)]
pub enum Event {
    Dial(Multiaddr),
    Kademlia(KademliaEvent),
    Gossipsub(gossipsub::Event),
}

impl From<KademliaEvent> for Event {
    fn from(event: KademliaEvent) -> Self {
        Event::Kademlia(event)
    }
}

impl From<gossipsub::Event> for Event {
    fn from(event: gossipsub::Event) -> Self {
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

        // protocol - gossipsub
        let gossipsub_config = gossipsub::Config::default();
        let mut gossipsub = gossipsub::Behaviour::new(message_authenticity, gossipsub_config)
            .expect("could not create gossipsub interface");

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
                        Event::Dial(addr) => {
                            let peer_id = match addr.iter().last() {
                                Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
                                _ => return ()
                            };
                            self.swarm.dial(addr.clone()).expect("to call addr");
                            self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                        },
                        Event::Kademlia(e) => {info!("unhandled {:#?}", e)},
                        _ => {info!("not handled event")}
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
                    SwarmEvent::Dialing(peer_id) => info!("Dialing {peer_id}"),
                    SwarmEvent::Behaviour(Event::Gossipsub(gossipsub::Event::Subscribed {
                        peer_id,
                        topic,
                    })) => {
                            info!(
                                "{peer_id} subscribed to {topic}"
                            )
                        }
                    SwarmEvent::Behaviour(Event::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message,
                        ..
                    })) => {
                            let peer_id = peer_id.to_string();
                            let peer_id = peer_id[peer_id.len() - 7..].to_string();
                            println!(
                                "{peer_id}: {}",
                                String::from_utf8_lossy(&message.data),
                            )
                        },
                    _ => {}
                },
            };
        }
    }
}
