#[macro_use]
extern crate serde;

use std::env;
use std::io;

use actix_session::CookieSession;
use actix_web::{App, HttpServer};
use env_logger;

mod messages;

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::builder().format_timestamp(None).init();
    let cookie_secret =
        env::var("BRIZZO_COOKIE_SECRET").expect("BRIZZO_COOKIE_SECRET not set");
    let cache = messages::Cache::new();
    HttpServer::new(move || {
        App::new()
            // Grant access to the cache
            .data(cache.clone())
            // Persist session as a cookie
            .wrap(
                CookieSession::signed(cookie_secret.as_bytes())
                    .secure(true)
                    .name("brizzo"),
            )
            .service(messages::create::handle)
            .service(messages::read::handle)
            .service(messages::update::handle)
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
