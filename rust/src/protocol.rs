use serde::{Deserialize, Serialize};

/// Messages sent from the client to the server.
/// Serialized as MessagePack with an internal "type" tag.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join { username: String },
    Turn { dir: u8 },
}

/// Messages received from the server.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    State {
        tick: u64,
        food: [u16; 2],
        snakes: Vec<SnakeData>,
    },
    Crown {
        name: String,
        crowns: u32,
    },
    Leaderboard {
        players: Vec<LeaderboardEntry>,
    },
    Error {
        msg: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnakeData {
    pub name: String,
    pub body: Vec<[u16; 2]>,
    pub dir: u8,
    pub crowns: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub name: String,
    pub crowns: u32,
    pub length: u16,
    pub alive: bool,
}

/// Encode a `ClientMessage` to a MessagePack byte vector.
pub fn encode(msg: &ClientMessage) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec_named(msg)
}

/// Decode a `ServerMessage` from a MessagePack byte slice.
pub fn decode(bytes: &[u8]) -> Result<ServerMessage, rmp_serde::decode::Error> {
    rmp_serde::from_slice(bytes)
}
