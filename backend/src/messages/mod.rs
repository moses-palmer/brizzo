use std::ops;

use actix_session::Session;

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

/// Clears the session cookie.
///
/// # Arguments
/// *  `session` - The session.
pub fn clear_id(session: &Session) {
    session.remove(XID_COOKIE);
}

/// Loads the identifier cookie from the session.
///
/// # Arguments
/// *  `session` - The session.
pub fn load_id(session: &Session) -> Result<xid::Identifier, xid::Error> {
    session
        .get::<String>(XID_COOKIE)
        .map_err(|_| xid::Error::Format)
        .and_then(|c| c.ok_or(xid::Error::Missing))
        .and_then(|s| s.parse::<xid::IdentifierCookie>())
        .map(xid::Identifier::from)
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
