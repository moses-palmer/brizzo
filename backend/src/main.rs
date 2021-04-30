#[macro_use]
extern crate serde;

use std::env;
use std::io;
use std::sync;

use actix_web::{App, HttpServer};
use env_logger;

mod configuration;
mod messages;
mod store;

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::builder().format_timestamp(None).init();

    let configuration = configuration::Configuration::load(
        &env::var("BRIZZO_CONFIGURATION_FILE")
            .expect("BRIZZO_CONFIGURATION_FILE not set"),
    )?;
    let bind = configuration.server_bind();
    let store = sync::Arc::new(sync::Mutex::new(configuration.store()?));
    HttpServer::new(move || {
        App::new()
            // Grant access to the store
            .data(store.clone())
            // Persist session as a cookie
            .wrap(configuration.session())
            .service(messages::create::handle)
            .service(messages::read::handle)
            .service(messages::update::handle)
    })
    .bind(bind)?
    .run()
    .await
}
