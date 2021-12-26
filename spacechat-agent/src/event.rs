use libp2p::floodsub::FloodsubEvent;
use tracing::info;

pub fn handle(message: FloodsubEvent) {
    match message {
        FloodsubEvent::Message(message) => {
            info!(
                "Received: '{:?}' from {:?}",
                String::from_utf8_lossy(&message.data),
                message.source
            );
        }
        _ => (),
    }
}
