use std::sync::mpsc;
use std::thread;
use crate::event;
use std::sync::Mutex;
use actix_web::{middleware, rt::System, web::{self, Data}, App, HttpServer, HttpRequest, HttpResponse, post};
use tracing_actix_web::TracingLogger;
use libp2p::{
    core::{upgrade, muxing::StreamMuxerBox, transport},
    floodsub::{self, Floodsub, FloodsubEvent},
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex,
    noise,
    swarm::{NetworkBehaviourEventProcess, SwarmBuilder, SwarmEvent, Swarm},
    // `TokioTcpConfig` is available through the `tcp-tokio` feature.
    tcp::TokioTcpConfig,
    Multiaddr,
    NetworkBehaviour,
    PeerId,
    Transport,
    futures::StreamExt,
};
use tracing::{info, error};
use std::collections::HashMap;
use crate::protos::generated::spacechat_agent::{PostRoomMessageRequest, PostRoomJoinRequest};

type Swarms = HashMap<String, (Swarm<Behaviour>, floodsub::Topic)>;

#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct Behaviour {
    floodsub: Floodsub,
    mdns: Mdns,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {
    // Called when `floodsub` produces an event.
    fn inject_event(&mut self, event: FloodsubEvent) {
        info!("event: {:?}", &event);
        event::handle(event)
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    info!("discovered: {}", &peer);
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub async fn run(host: &str, port: &str, listen_multiaddr: &str) -> std::io::Result<()> {
    // Create a random PeerId
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    info!("Local peer id: {:?}", peer_id);

    // Create a keypair for authenticated encryption of the transport.
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&id_keys)
        .expect("Signing libp2p-noise static DH keypair failed.");

    // Create a tokio-based TCP transport use noise for authenticated
    // encryption and Mplex for multiplexing of substreams on a TCP stream.
    let transport = TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Create a Floodsub topic
    let floodsub_topic = floodsub::Topic::new("chat");

    // Create a Swarm to manage peers and events.
    let mut swarm = {
        let mdns = Mdns::new(Default::default()).await.unwrap();
        let mut behaviour = Behaviour {
            floodsub: Floodsub::new(peer_id.clone()),
            mdns,
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());

        SwarmBuilder::new(transport, behaviour, peer_id)
            // We want the connection background tasks to be spawned
            // onto the tokio runtime.
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build()
    };

    // Reach out to another node if specified
    // if let Some(to_dial) = std::env::args().nth(1) {
    //     let addr: Multiaddr = "/ip4/192.168.1.7/tcp/64619".parse().unwrap();
    //     swarm.dial(addr).unwrap();
    //     info!("Dialed {:?}", to_dial)
    // }

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

    let mut m = HashMap::new();
    m.insert(String::from("chat"), (swarm, floodsub_topic));
    let swarms = web::Data::new(Mutex::new(m));

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(TracingLogger::default())
            .app_data(swarms.clone())
            .app_data(peer_id.clone())
            // .service(handler::post_register_client)
            .service(post_room_messages)
            .service(post_room_join)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

#[post("/1/rooms/{room_id}/messages")]
pub async fn post_room_messages(
    swarms: web::Data<Mutex<Swarms>>,
    path: web::Path<String>,
    web::Json(body): web::Json<PostRoomMessageRequest>,
    _req: HttpRequest,
) -> HttpResponse {
    let room_id = path.into_inner();
    let mut swarms = swarms.lock().unwrap();

    match swarms.get_mut(&room_id) {
        Some((swarm, floodsub_topic)) => {
            info!("room_id: {}", &room_id);
            swarm.behaviour_mut().floodsub.publish(floodsub_topic.clone(), body.text.as_bytes());
            HttpResponse::Ok().body("")
        }
        None => {
            HttpResponse::NotFound().body("")
        }
    }
}

#[post("/1/rooms/{room_id}/join")]
pub async fn post_room_join(
    swarms: web::Data<Mutex<Swarms>>,
    peer_id: web::Data<PeerId>,
    transport: web::Data<transport::Boxed<(PeerId, StreamMuxerBox)>>,
    path: web::Path<String>,
    web::Json(body): web::Json<PostRoomJoinRequest>,
    _req: HttpRequest,
) -> HttpResponse {
    let room_id = path.into_inner();
    let peer_id = peer_id.as_ref();
    let transport = transport.as_ref();

    let floodsub_topic = floodsub::Topic::new(&body.topic);

    // Create a Swarm to manage peers and events.
    let mut swarm = {
        let mdns = Mdns::new(Default::default()).await.unwrap();
        let mut behaviour = Behaviour {
            floodsub: Floodsub::new(peer_id.clone()),
            mdns,
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());

        SwarmBuilder::new(transport.clone(), behaviour, *peer_id)
            // We want the connection background tasks to be spawned
            // onto the tokio runtime.
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build()
    };

    swarm.listen_on(format!("/ip4/0.0.0.0/tcp/0").parse().unwrap()).unwrap();

    let mut swarms = swarms.lock().unwrap();
    swarms.insert(room_id, (swarm, floodsub_topic));

    HttpResponse::Ok().body("")
}
