use std::fs;
use std::io;
use std::io::Read;
use std::time;

use actix_session::CookieSession;
use toml;

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
    /// The secret used to protect cookies.
    secret: String,

    /// Whether the cookie should be secure.
    secure: bool,

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
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    /// The bind string to which to listen.
    pub fn server_bind(&self) -> String {
        self.server.bind.clone()
    }

    /// A cookie session description.
    pub fn session(&self) -> CookieSession {
        CookieSession::signed(self.session.secret.as_bytes())
            .secure(self.session.secure)
            .name(&self.session.name)
    }

    /// A store for values.
    pub fn store(&self) -> Result<store::Store, store::Error> {
        store::Store::new(
            self.redis.connection_string.clone(),
            time::Duration::from_millis(self.redis.ttl),
        )
    }
}
