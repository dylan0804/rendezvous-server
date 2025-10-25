use libp2p::{
    futures::StreamExt,
    identify,
    identity::{self, Keypair},
    multiaddr::Protocol,
    noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr,
};
use std::{error::Error, fs, net::Ipv4Addr, path::Path};

const PORT: u16 = 8123;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // let _ = tracing_subscriber::fmt()
    //     .with_env_filter(EnvFilter::from_default_env())
    //     .try_init();

    // let opt = Opt::parse();

    // Create a static known PeerId based on given secret
    // let local_key: identity::Keypair = generate_ed25519(123);
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| ".".into())
        .join("meshdrop/relay.key");
    let key = load_or_generate_key(&config_path)?;
    println!("Peer id {}", key.public().to_peer_id());

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| Behaviour {
            relay: relay::Behaviour::new(key.public().to_peer_id(), Default::default()),
            ping: ping::Behaviour::new(ping::Config::new()),
            identify: identify::Behaviour::new(identify::Config::new(
                "/TODO/0.0.1".to_string(),
                key.public(),
            )),
        })?
        .build();

    // Listen on all interfaces
    let listen_addr_tcp = Multiaddr::empty()
        // .with(match opt.use_ipv6 {
        //     Some(true) => Protocol::from(Ipv6Addr::UNSPECIFIED),
        //     _ => Protocol::from(Ipv4Addr::UNSPECIFIED),
        // })
        .with(Protocol::from(Ipv4Addr::UNSPECIFIED))
        .with(Protocol::Tcp(PORT));
    swarm.listen_on(listen_addr_tcp)?;

    let listen_addr_quic = Multiaddr::empty()
        // .with(match opt.use_ipv6 {
        //     Some(true) => Protocol::from(Ipv6Addr::UNSPECIFIED),
        //     _ => Protocol::from(Ipv4Addr::UNSPECIFIED),
        // })
        .with(Protocol::from(Ipv4Addr::UNSPECIFIED))
        .with(Protocol::Udp(PORT))
        .with(Protocol::QuicV1);
    swarm.listen_on(listen_addr_quic)?;

    loop {
        match swarm.next().await.expect("Infinite Stream.") {
            SwarmEvent::Behaviour(event) => {
                if let BehaviourEvent::Identify(identify::Event::Received {
                    info: identify::Info { observed_addr, .. },
                    ..
                }) = &event
                {
                    swarm.add_external_address(observed_addr.clone());
                }

                println!("{event:?}")
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {address:?}");
            }
            _ => {}
        }
    }
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    relay: relay::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

fn load_or_generate_key(config_path: &Path) -> Result<Keypair, Box<dyn Error>> {
    if config_path.exists() {
        let bytes = fs::read(config_path)?;
        let key = identity::Keypair::from_protobuf_encoding(&bytes)?;
        Ok(key)
    } else {
        let key = identity::Keypair::generate_ed25519();
        let bytes = key.to_protobuf_encoding()?;

        if let Some(prefix) = config_path.parent() {
            std::fs::create_dir_all(prefix)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(config_path)?;

            use std::io::Write;
            f.write_all(&bytes)?
        }

        Ok(key)
    }
}

// fn generate_ed25519(secret_key_seed: u8) -> identity::Keypair {
//     let mut bytes = [0u8; 32];
//     bytes[0] = secret_key_seed;
//
//     identity::Keypair::ed25519_from_bytes(bytes).expect("only errors on wrong length")
// }

// #[derive(Debug, Parser)]
// #[command(name = "libp2p relay")]
// struct Opt {
//     /// Determine if the relay listen on ipv6 or ipv4 loopback address. the default is ipv4
//     #[arg(long)]
//     use_ipv6: Option<bool>,
//
//     /// Fixed value to generate deterministic peer id
//     #[arg(long)]
//     secret_key_seed: u8,
//
//     /// The port used to listen on all interfaces
// }
