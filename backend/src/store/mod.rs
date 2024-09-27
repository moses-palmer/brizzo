use std::time;

use r2d2_redis;
use r2d2_redis::r2d2;
use r2d2_redis::redis;
use r2d2_redis::redis::Commands;

use crate::messages;
use crate::messages::xid;

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

    /// Reads a room description from the store.
    ///
    /// # Arguments
    /// *  `message_name` - The name of the message.
    /// *  `id` - The room ID.
    pub fn get(
        &mut self,
        message_name: &str,
        id: Option<xid::Identifier>,
    ) -> Result<Option<messages::Room>, Error> {
        let mut conn = self.pool.get()?;

        Ok(conn.get(
            id.map(|id| self.key(message_name, id))
                .unwrap_or_else(|| message_name.into()),
        )?)
    }

    /// Checks whether a message exists.
    ///
    /// # Arguments
    /// *  `message_name` - The name of the message.
    pub fn exists(&mut self, message_name: &str) -> Result<bool, Error> {
        let mut conn = self.pool.get()?;

        Ok(conn.exists(message_name)?)
    }

    /// Stores an entire message in the store.
    ///
    /// This method will fail if a message with the given name already exists.
    ///
    /// # Arguments
    /// *  `message` - The message to store.
    pub fn put_message(
        &mut self,
        message: &messages::Message,
    ) -> Result<(), Error> {
        let mut conn = self.pool.get()?;

        if conn.exists(message.name())? {
            Err(Error::Exists)
        } else {
            // First store the entrance room...
            let entrance = message
                .describe((0isize, 0isize).into())
                .ok_or(Error::InternalError)?;
            conn.set_ex::<_, _, ()>(
                message.name(),
                entrance,
                self.ttl.as_secs() as usize,
            )
            .map_err(|_| Error::WriteError)?;

            // ...then all the others
            for room in message.rooms() {
                conn.set_ex::<_, _, ()>(
                    self.key(message.name(), room.xid),
                    room,
                    self.ttl.as_secs() as usize,
                )
                .map_err(|_| Error::WriteError)?;
            }

            Ok(())
        }
    }

    /// Generates the key for a room in a message.
    ///
    /// # Arguments
    /// *  `message_name` - The name of the message.
    /// *  `id` - The ID of the room.
    fn key(&self, message_name: &str, id: xid::Identifier) -> String {
        format!("{}.{}", message_name, id)
    }
}

impl redis::FromRedisValue for messages::Room {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        match v {
            redis::Value::Data(v) => rmp_serde::from_slice(v).map_err(|_| {
                (redis::ErrorKind::TypeError, "invalid room data").into()
            }),
            _ => Err((redis::ErrorKind::TypeError, "invalid room data").into()),
        }
    }
}

impl redis::ToRedisArgs for messages::Room {
    fn write_redis_args<W: ?Sized>(&self, out: &mut W)
    where
        W: redis::RedisWrite,
    {
        match rmp_serde::to_vec(self) {
            Ok(v) => out.write_arg(&v),
            Err(_) => log::warn!("Failed to write {:?} to redis", self),
        }
    }
}
