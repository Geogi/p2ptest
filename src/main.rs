use std::time::Duration;

use anyhow::{anyhow, Error, Result};
use identity::Keypair as IdKeypair;
use libp2p::{
    core::{
        muxing::StreamMuxerBox,
        transport::Boxed,
        upgrade::{SelectUpgrade, Version},
    },
    dns::DnsConfig,
    gossipsub::{Gossipsub, GossipsubConfig, IdentTopic, MessageAuthenticity},
    identity,
    mplex::MplexConfig,
    noise::{Keypair as NoiseKeypair, NoiseConfig, X25519Spec},
    pnet::{PnetConfig, PreSharedKey},
    tcp::TcpConfig,
    websocket::WsConfig,
    yamux::YamuxConfig,
    PeerId, Swarm, Transport,
};

#[cfg(feature = "gui")]
mod gui;
mod key;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut swarm = setup_p2p().await?;
    p2p_addrs(&mut swarm)?;

    run(&mut swarm).await?;

    Ok(())
}

async fn transport_default_plus_pnet(
    keypair: IdKeypair,
    psk: PreSharedKey,
) -> Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let transport = {
        let tcp = TcpConfig::new().nodelay(true);
        let dns_tcp = DnsConfig::system(tcp).await?;
        let ws_dns_tcp = WsConfig::new(dns_tcp.clone());
        dns_tcp.or_transport(ws_dns_tcp)
    };

    let noise_keys = NoiseKeypair::<X25519Spec>::new()
        .into_authentic(&keypair)
        .expect("Signing libp2p-noise static DH keypair failed.");

    Ok(transport
        .and_then(move |socket, _| PnetConfig::new(psk).handshake(socket))
        .upgrade(Version::V1)
        .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(SelectUpgrade::new(
            YamuxConfig::default(),
            MplexConfig::default(),
        ))
        .timeout(Duration::from_secs(20))
        .boxed())
}

async fn setup_p2p() -> Result<Swarm<Gossipsub>> {
    let id_keys = IdKeypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    let transport = transport_default_plus_pnet(id_keys.clone(), key::key()).await?;
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
