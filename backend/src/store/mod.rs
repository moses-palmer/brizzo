use std::time;

use r2d2_redis;
use r2d2_redis::r2d2;
use r2d2_redis::redis;

mod error;
pub use self::error::Error;

/// A distributed store.
#[derive(Clone)]
pub struct Store {
    /// The connection pool.
    pool: r2d2::Pool<r2d2_redis::RedisConnectionManager>,

    /// The TTL for records.
    ttl: time::Duration,
}

impl Store {
    /// Creates a new store.
    ///
    /// # Arguments
    /// *  `connection_info` - A connection string.
    /// *  `ttl` - The time-to-live for records.
    pub fn new<T>(
        connection_info: T,
        ttl: time::Duration,
    ) -> Result<Self, Error>
    where
        T: redis::IntoConnectionInfo,
    {
        Ok(Self {
            pool: r2d2::Pool::builder().build(
                r2d2_redis::RedisConnectionManager::new(connection_info)?,
            )?,
            ttl,
        })
    }
}
