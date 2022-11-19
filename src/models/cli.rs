use clap::Parser;
use libp2p::Multiaddr;

#[derive(Parser, Debug)]
#[clap(name = "p2p chat")]
pub struct Opt {
    #[clap(long)]
    pub peer: Option<Multiaddr>,

    #[clap(long)]
    pub listen_address: Option<Multiaddr>,
    // #[clap(subcommand)]
    // argument: CliArgument,
}
