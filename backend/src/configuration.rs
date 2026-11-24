use std::{
    fs,
    io::{self, Read},
    time,
};

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::cookie::Key;

use crate::store;

#[derive(Clone, Deserialize, Serialize)]
pub struct Configuration {
    /// Server related configurations.
    server: Server,

    /// Session related configurations.
    session: Session,

    /// Redis connection information.
    redis: Redis,
}

#[derive(Clone, Deserialize, Serialize)]
struct Server {
    /// The bind string.
    bind: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct Session {
    /// The name of the cookie
    name: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct Redis {
    /// The connection information.
    connection_string: String,

    /// The TTL for records, in milliseconds.
    ttl: u64,
}

impl Configuration {
    /// Loads the application configuration from a TOML file.
    ///
    /// # Arguments
    /// *  `path` - The path to the configuration file.
    pub fn load(path: &str) -> io::Result<Self> {
        toml::from_str(&{
            let mut file = fs::File::open(path)?;
            let mut data = String::new();
            file.read_to_string(&mut data)?;
            data
        })
        .map_err(io::Error::other)
    }

    /// The bind string to which to listen.
    pub fn server_bind(&self) -> String {
        self.server.bind.clone()
    }

    /// A cookie session description.
    pub fn session_middleware(
        &self,
        store: RedisSessionStore,
        key: Key,
    ) -> SessionMiddleware<RedisSessionStore> {
        SessionMiddleware::new(store, key)
    }

    /// A store for values.
    pub fn store(&self) -> Result<store::Store, store::Error> {
        store::Store::new(
            self.redis.connection_string.clone(),
            time::Duration::from_millis(self.redis.ttl),
        )
    }

    /// A store for values.
    pub async fn session_store(
        &self,
    ) -> Result<RedisSessionStore, store::Error> {
        RedisSessionStore::new(&self.redis.connection_string)
            .await
            .map_err(|e| {
                log::error!("failed to connect to Redis: {e:?}");
                store::Error::Connection
            })
    }
}
