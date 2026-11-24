#[macro_use]
extern crate serde;

use std::{env, io, sync};

use actix_web::{web::Data, App, HttpServer};

mod configuration;
mod messages;
mod store;

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::builder().format_timestamp(None).init();

    let configuration = configuration::Configuration::load(
        &env::var("BRIZZO_CONFIGURATION_FILE")
            .expect("BRIZZO_CONFIGURATION_FILE not set"),
    )
    .unwrap();
    let bind = configuration.server_bind();
    let store = Data::new(sync::Mutex::new(configuration.store().unwrap()));
    let redis_session_store = configuration.session_store().await.unwrap();
    let key = actix_web::cookie::Key::generate();
    HttpServer::new(move || {
        App::new()
            // Grant access to the store
            .app_data(store.clone())
            // Persist session as a cookie
            .wrap(configuration.session_middleware(redis_session_store.clone(), key.clone()))
            .service(messages::create::handle)
            .service(messages::read::handle)
            .service(messages::update::handle)
    })
    .bind(bind)?
    .run()
    .await
}
