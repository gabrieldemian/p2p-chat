use libp2p::{
    core::upgrade,
    futures::StreamExt,
    gossipsub::{Gossipsub, GossipsubConfig, IdentTopic, MessageAuthenticity},
    identity::Keypair,
    kad::{store::MemoryStore, Kademlia},
    mplex,
    multiaddr::Protocol,
    noise::NoiseAuthenticated,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp::{GenTcpConfig, TokioTcpTransport},
    Multiaddr, NetworkBehaviour, PeerId, Swarm, Transport,
};
use log::info;
use tokio::{select, sync::mpsc};

use super::cli::Opt;

// defines the behaviour of the current peer
// on the network
#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    pub gossipsub: Gossipsub,
    pub kademlia: Kademlia<MemoryStore>,
}

pub enum Event {
    Liebe(Vec<u8>),
}

pub struct Network {
    pub swarm: Swarm<AppBehaviour>,
    pub peer_id: PeerId,
    pub event_sender: mpsc::UnboundedSender<Event>,
    pub event_receiver: mpsc::UnboundedReceiver<Event>,
}

impl Network {
    pub async fn new(opt: &Opt) -> Self {
        // generate the peer public key (peerId)
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();

        // instantiate the TCP protocol, with noise cryptography
        // and multiplexed
        let transport_config = GenTcpConfig::new().port_reuse(true);
        let transport = TokioTcpTransport::new(transport_config)
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
            SwarmBuilder::new(transport, behaviour, peer_id)
                .executor(Box::new(|fut| {
                    tokio::spawn(fut);
                }))
                .build()
        };

        let multiaddr: Multiaddr = match &opt.listen_address {
            Some(addr) => addr.clone(),
            None => "/ip4/0.0.0.0/tcp/0".parse().expect("address to be valid"),
        };

        // this peer will listen to events on the network
        swarm
            .listen_on(multiaddr)
            .expect("could not listen on swarm");

        info!("Your PeerID is {peer_id}");

        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Self {
            swarm,
            peer_id,
            event_sender,
            event_receiver,
        }
    }

    pub async fn daemon(&mut self) {
        loop {
            select! {
                event = self.event_receiver.recv() => {
                    match event.unwrap() {
                        Event::Liebe(data) => { info!("liebe: {:#?}", data) }
                    };
                },
                swarm_event = self.swarm.select_next_some() => match swarm_event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(
                            "Local node is listening on {:?}",
                            address.with(Protocol::P2p(self.peer_id.into()))
                        );
                    },
                    SwarmEvent::Dialing(peer_id) => info!("Dialing {peer_id}"),
                    _ => {}
                },
            };
        }
    }
}
