use std::collections::vec_deque;
use std::ops;
use std::sync;

use actix_session::Session;
use r2d2_redis::redis;
use rmp_serde;

use maze::initialize;
use maze::matrix;
use maze::physical;
use maze_tools::alphabet;
use maze_tools::cell::*;
use maze_tools::image::Color;

pub mod create;
pub mod read;
pub mod update;
pub mod xid;

/// The maximum number of cached messages.
const MAX_MESSAGES: usize = 64;

/// The name of the room identifier cookie.
const XID_COOKIE: &'static str = "xid";

/// The colour of the text.
const TEXT_COLOR: Color = Color {
    red: 64,
    green: 64,
    blue: 64,
    alpha: 255,
};

/// Information for a single room.
#[derive(Clone, Copy, Default)]
pub struct Cell {
    /// The room colour.
    color: Color,

    /// The room ID.
    id: xid::Identifier,
}

/// The maze type.
pub type Maze = maze::Maze<Cell>;

/// A cached message.
pub struct Message {
    /// The name of this message.
    name: String,

    /// The actual maze.
    maze: Maze,
}

impl Message {
    /// Constructs a new message.
    ///
    /// # Arguments
    /// *  `name` - The name of the message.
    /// *  `text` - The actual text.
    /// *  `shape` - The type of maze to generate.
    /// *  `seed` - The random seed.
    pub fn new(
        name: &str,
        text: &str,
        shape: maze::Shape,
        mut seed: initialize::LFSR,
    ) -> Self {
        // Let the matrix be square-ish
        let columns = (text.len() as f32).sqrt().ceil() as usize;
        let rows = (text.len() as f32 / columns as f32).ceil() as usize;

        let name = name.to_owned();
        let (width, height) =
            shape.minimal_dimensions(columns as f32 * 16.0, rows as f32 * 16.0);
        let viewbox = shape.viewbox(width, height);
        let data = alphabet::default::ALPHABET
            .render(text, columns, width * 16)
            .map(|(pos, v)| {
                (
                    physical::Pos {
                        x: viewbox.width * pos.x / columns as f32,
                        y: viewbox.height * pos.y / rows as f32,
                    },
                    Intermediate::from((pos, v)),
                )
            })
            .split_by(&shape, width, height)
            .map(|&color| Cell {
                color,
                id: seed.advance().into(),
            });
        let maze = shape
            .create_with_data(data.width, data.height, |pos| data[pos])
            .initialize(initialize::Method::Branching, &mut seed);

        Self { name, maze }
    }

    /// The name of this message.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Ierates over all room descriptions.
    pub fn rooms<'a>(&'a self) -> impl Iterator<Item = Room> + 'a {
        self.maze
            .positions()
            .flat_map(move |pos| self.describe(pos))
    }

    /// Returns the identifier of the entry room.
    pub fn entry(&self) -> xid::Identifier {
        self.maze.data((0isize, 0isize).into()).unwrap().id
    }

    /// Finds a room position by identifier.
    ///
    /// # Arguments
    /// *  `id` - The room identifier.
    pub fn lookup(&self, id: xid::Identifier) -> Option<matrix::Pos> {
        self.maze
            .positions()
            .filter(|&pos| self.maze.data(pos).unwrap().id == id)
            .next()
    }

    /// Generates a description of a room.
    ///
    /// # Arguments
    /// *  `pos` - The room position.
    pub fn describe(&self, pos: matrix::Pos) -> Option<Room> {
        self.maze.data(pos).map(|&data| Room {
            xid: data.id,
            pos: self.maze.center(pos),
            col: data.color.to_string(),
            see: self
                .maze
                .neighbors(pos)
                .filter_map(|pos| self.maze.data(pos))
                .map(|data| data.id)
                .collect(),
        })
    }

    /// Attempts to move from a room to a reachable neighbour with the
    /// identifier specified.
    ///
    /// # Arguments
    /// *  `from` - The current position.
    /// *  `to` - The identifier of the neighbour.
    pub fn transition(
        &self,
        from: matrix::Pos,
        to: xid::Identifier,
    ) -> Option<matrix::Pos> {
        self.maze
            .neighbors(from)
            .filter(|&pos| {
                self.maze
                    .data(pos)
                    .map(|&data| data.id == to)
                    .unwrap_or(false)
            })
            .next()
    }
}

/// An intermediate value use to accumulate data for a room.
#[derive(Clone, Copy, Default)]
struct Intermediate(physical::Pos, f32);

impl From<(physical::Pos, f32)> for Intermediate {
    fn from((pos, v): (physical::Pos, f32)) -> Self {
        Intermediate(pos, v)
    }
}

impl ops::Add<Intermediate> for Intermediate {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Intermediate(other.0, self.1 + other.1)
    }
}

impl ops::Div<usize> for Intermediate {
    type Output = Color;

    fn div(self, divisor: usize) -> Self::Output {
        let f = self.1 / divisor as f32;
        let red = 120.0 + (10.0 * (3.0 * self.0.x).cos());
        let green = 120.0 + (10.0 * (3.0 * (self.0.x + self.0.y)).cos());
        let blue = 120.0 + (10.0 * (3.0 * (self.0.x * self.0.y)).cos());
        TEXT_COLOR.fade(
            Color {
                red: red as u8,
                green: green as u8,
                blue: blue as u8,
                alpha: 255,
            },
            f,
        )
    }
}

/// A room description.
#[derive(Debug, Deserialize, Serialize)]
pub struct Room {
    /// The room identifier.
    pub xid: xid::Identifier,

    /// The position of the centre of the room.
    pub pos: physical::Pos,

    /// The colour of the room.
    pub col: String,

    /// The identifiers of the room neighbours.
    pub see: Vec<xid::Identifier>,
}

impl redis::FromRedisValue for Room {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        match v {
            redis::Value::Data(v) => {
                rmp_serde::from_read_ref(v).map_err(|_| {
                    (redis::ErrorKind::TypeError, "invalid room data").into()
                })
            }
            _ => Err((redis::ErrorKind::TypeError, "invalid room data").into()),
        }
    }
}

impl redis::ToRedisArgs for Room {
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

/// The room cache type.
#[derive(Clone)]
pub struct Cache(sync::Arc<sync::RwLock<vec_deque::VecDeque<Message>>>);

impl Cache {
    /// Creates a new cache.
    pub fn new() -> Self {
        Self(sync::Arc::new(
            sync::RwLock::new(vec_deque::VecDeque::new()),
        ))
    }

    /// Grants access to cached messages.
    ///
    /// # Panics
    /// This function will panic if the cache lock cannot be acquired.
    pub fn read(&self) -> sync::RwLockReadGuard<vec_deque::VecDeque<Message>> {
        self.0.read().unwrap()
    }

    /// Attempts to cache a message.
    ///
    /// If a message with the same name already exists, this function will
    /// return the message wrapped in an error.
    ///
    /// # Arguments
    /// *  `message` - The message to cache.
    ///
    /// # Panics
    /// This function will panic if the cache lock cannot be acquired.
    pub fn store(&self, message: Message) -> Result<String, Message> {
        let mut cache = self.0.write().unwrap();
        if cache.iter().any(|m| m.name == message.name) {
            Err(message)
        } else {
            if cache.len() >= MAX_MESSAGES {
                (*cache).pop_front();
            }
            let result = message.name.clone();
            (*cache).push_back(message);

            Ok(result)
        }
    }
}

/// Loads the identifier cookie from the session.
///
/// # Arguments
/// *  `session` - The session.
pub fn load_id(
    session: &Session,
) -> Option<Result<xid::Identifier, xid::Error>> {
    let string = session
        .get::<String>(XID_COOKIE)
        .map_err(|_| xid::Error::Format)
        .transpose()?;
    Some(
        string
            .and_then(|s| s.parse::<xid::IdentifierCookie>())
            .map(xid::Identifier::from),
    )
}

/// Stores an identifier cookie to the session.
///
/// # Arguments
/// *  `session` - The session.
/// *  `id` - The identifier to store.
pub fn store_id(
    session: &Session,
    id: xid::Identifier,
) -> Result<xid::Identifier, xid::Error> {
    session
        .set(XID_COOKIE, xid::IdentifierCookie::from(id).to_string())
        .map_err(|_| xid::Error::Format)
        .map(|_| id)
}

/// Asserts that the session contains an identifier cookie.
///
/// If none exists, a default value if generated by `default` and stored to the
/// session.
///
/// # Arguments
/// *  `session` - The session.
/// *  `default` - A generator of default values.
pub fn assert_id<F>(
    session: &Session,
    default: F,
) -> Result<xid::Identifier, xid::Error>
where
    F: FnOnce() -> xid::Identifier,
{
    load_id(session).unwrap_or_else(|| store_id(session, default()))
}
