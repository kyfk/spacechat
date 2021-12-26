use crate::{server::handler, PrivateKey, PublicKey};
use actix_web::{middleware, web::Data, App, HttpServer};
use std::io;
use tracing_actix_web::TracingLogger;

pub async fn run_server(
    host: &str,
    public_key: PublicKey,
    private_key: PrivateKey,
    passphrase: Vec<u8>,
) -> io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(TracingLogger::default())
            .app_data(Data::new((public_key.clone(), private_key.clone())))
            .app_data(Data::new(passphrase.clone()))
            .service(handler::get_health)
            .service(handler::post_id_allocate)
            .service(handler::get_id_verify)
    })
    .bind(host)?
    .run()
    .await
}
