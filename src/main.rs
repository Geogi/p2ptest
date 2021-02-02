use anyhow::{anyhow, Error, Result};
use async_std::task;
use identity::Keypair;
use libp2p::{
    gossipsub::{Gossipsub, GossipsubConfig, IdentTopic, MessageAuthenticity},
    identity, PeerId, Swarm,
};
use futures::future;

#[cfg(feature = "gui")]
mod gui;
mod key;

fn main() -> Result<()> {
    env_logger::init();

    let mut swarm = setup_p2p()?;
    p2p_addrs(&mut swarm)?;

    task::block_on(future::poll_fn(|cx| run(cx, &mut swarm)))?;

    Ok(())
}

fn setup_p2p() -> Result<Swarm<Gossipsub>> {
    let id_keys = Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    let transport = libp2p::build_tcp_ws_pnet_noise_mplex_yamux(id_keys.clone(), key::key())?;
    let topic = IdentTopic::new("p2ptest");
    let message_authenticity = MessageAuthenticity::Signed(id_keys);
    let gossipsub_config = GossipsubConfig::default();
    let mut gossipsub: Gossipsub =
        Gossipsub::new(message_authenticity, gossipsub_config).map_err(Error::msg)?;
    gossipsub
        .subscribe(&topic)
        .map_err(|e| anyhow!("{:?}", e))?;
    let swarm = Swarm::new(transport, gossipsub, peer_id);

    Ok(swarm)
}

fn p2p_addrs(swarm: &mut Swarm<Gossipsub>) -> Result<()> {
    if cfg!(feature = "gui") {
        Swarm::listen_on(swarm, "/ip4/0.0.0.0/tcp/0".parse()?)?;
        Swarm::dial_addr(swarm, "/dns/yuyuwai.net/tcp/8000".parse()?)?;
    } else {
        Swarm::listen_on(swarm, "/ip4/0.0.0.0/tcp/8000".parse()?)?;
    }

    Ok(())
}

#[cfg(not(feature = "gui"))]
pub async fn run<T>(swarm: &mut T) -> Result<()>
where
    T: Stream + Unpin,
    <T as Stream>::Item: Debug,
{
    loop {
        while let Some(event) = swarm.next().await {
            println!("{:?}", event);
        }
    }
}

#[cfg(feature = "gui")]
use gui::run;
