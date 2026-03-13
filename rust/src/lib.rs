use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use log::{Level, info};
use web_sys::{
    BinaryType, Document, Element, ErrorEvent, HtmlElement, MessageEvent, WebSocket, window,
};

use wasm_bindgen::prelude::*;

mod error;
mod protocol;

pub use error::{Result, WebError};
use protocol::{ClientMessage, ServerMessage, SnakeData, encode};

const STAGE_SELECTOR: &str = ".stage";
const TILES_MARGIN: usize = 1;
const TILES_SIZE: usize = 10;
const COLS: usize = 64;
const ROWS: usize = 32;
const SERVER_URL: &str = "wss://yamanote.proxy.rlwy.net:59780";

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

fn snake_hue(id: u8) -> f32 {
    const START_HUES: [f32; 3] = [0.0, 120.0, 240.0];
    if id < 3 {
        START_HUES[id as usize]
    } else {
        (START_HUES[2] + (id as f32 - 2.0) * 137.50776) % 360.0
    }
}

pub fn snake_color(id: u8) -> String {
    let (r, g, b) = hsv_to_rgb(snake_hue(id), 0.85, 0.95);
    format!("#{r:02X}{g:02X}{b:02X}")
}

// ---------------------------------------------------------------------------
// WASM setup
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn setup_logs() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Debug).expect("Could not setup log level.");
    info!("WASM initialized!");
}

// ---------------------------------------------------------------------------
// Tile grid
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum TileKind {
    #[default]
    Empty,
    /// Pre-computed hex color for this segment.
    Snake(String),
    Food,
}

#[derive(Clone, Debug)]
struct Tile {
    element: HtmlElement,
    kind: TileKind,
}

#[derive(Clone, Debug)]
struct Tiles([[Tile; ROWS]; COLS]);

impl Tiles {
    fn build(document: &Document, stage: &Element) -> Self {
        let tiles = std::array::from_fn(|cols_i| {
            std::array::from_fn(|rows_i| {
                let element = document
                    .create_element("div")
                    .unwrap()
                    .dyn_into::<HtmlElement>()
                    .unwrap();

                element.set_class_name("tile");
                let style = element.style();
                style.set_property("position", "absolute").unwrap();

                let left = format!("{}px", cols_i * (TILES_SIZE + TILES_MARGIN));
                let top = format!("{}px", rows_i * (TILES_SIZE + TILES_MARGIN));
                style.set_property("left", &left).unwrap();
                style.set_property("top", &top).unwrap();

                let size = format!("{TILES_SIZE}px");
                style.set_property("width", &size).unwrap();
                style.set_property("height", &size).unwrap();

                stage.append_child(&element).unwrap();

                Tile {
                    element,
                    kind: TileKind::Empty,
                }
            })
        });
        Self(tiles)
    }

    fn set_tile_kind(&mut self, x: usize, y: usize, tile_kind: TileKind) -> bool {
        if let Some(row) = self.0.get_mut(x)
            && let Some(slot) = row.get_mut(y)
        {
            slot.kind = tile_kind;
            return true;
        }
        false
    }

    /// Reset all tiles to Empty.
    fn clear(&mut self) {
        for x in 0..COLS {
            for y in 0..ROWS {
                self.0[x][y].kind = TileKind::Empty;
            }
        }
    }

    fn refresh(&self, prev_tiles: &Tiles) {
        for x in 0..COLS {
            for y in 0..ROWS {
                if self.0[x][y].kind != prev_tiles.0[x][y].kind {
                    let style = self.0[x][y].element.style();
                    match &self.0[x][y].kind {
                        TileKind::Empty => {
                            style.remove_property("background-color").unwrap();
                        }
                        TileKind::Snake(color) => {
                            style.set_property("background-color", color).unwrap();
                        }
                        TileKind::Food => {
                            style.set_property("background-color", "#FFE000").unwrap();
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared game state (updated by WS, read by draw)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct GameState {
    food: Option<[u16; 2]>,
    snakes: Vec<SnakeData>,
    crowns: Vec<(String, u32)>,
    leaderboard: Vec<protocol::LeaderboardEntry>,
    pending_join: Option<Vec<u8>>,
    /// Stable name → color-index map; first-seen order never changes.
    player_color_idx: HashMap<String, u8>,
}

impl GameState {
    fn apply_state(&mut self, food: [u16; 2], snakes: Vec<SnakeData>) {
        self.food = Some(food);
        // Assign a stable color index the first time we see each player.
        let mut next_idx = self.player_color_idx.len() as u8;
        for snake in &snakes {
            self.player_color_idx
                .entry(snake.name.clone())
                .or_insert_with(|| {
                    let idx = next_idx % 255;
                    next_idx = next_idx.wrapping_add(1);
                    idx
                });
        }
        self.snakes = snakes;
    }

    /// Upsert a single player's crown count and keep the list sorted.
    fn apply_crown(&mut self, name: String, crowns: u32) {
        if let Some(entry) = self.crowns.iter_mut().find(|(n, _)| n == &name) {
            entry.1 = crowns;
        } else {
            self.crowns.push((name, crowns));
        }
        self.crowns.sort_by(|a, b| b.1.cmp(&a.1));
    }

    /// Replace the full crown list and leaderboard from a leaderboard broadcast.
    fn apply_leaderboard(&mut self, players: Vec<protocol::LeaderboardEntry>) {
        self.crowns = players.iter().map(|p| (p.name.clone(), p.crowns)).collect();
        self.crowns.sort_by(|a, b| b.1.cmp(&a.1));
        self.leaderboard = players;
    }
}

// ---------------------------------------------------------------------------
// Direction enum (exported to JS)
// ---------------------------------------------------------------------------

/// Direction values match the server protocol: 0=Up, 1=Right, 2=Down, 3=Left.
#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up = 0,
    Right = 1,
    Down = 2,
    Left = 3,
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct Board {
    tiles: Tiles,
    stage: HtmlElement,
    username: Option<String>,
    ws: WebSocket,
    state: Rc<RefCell<GameState>>,
}

/// Opens a WebSocket to SERVER_URL and registers all event callbacks.
/// `state` is shared with the caller so the onmessage handler can write into it.
fn ws_connect(state: Rc<RefCell<GameState>>) -> WebSocket {
    let ws = WebSocket::new(SERVER_URL).expect("failed to open WebSocket");
    ws.set_binary_type(BinaryType::Arraybuffer);

    // Clone the Rc shares we need for each closure before any move happens
    let state_msg = Rc::clone(&state);
    let state_open = Rc::clone(&state);
    drop(state); // all consumers have their own clone now

    // --- onmessage ---
    let onmessage = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let bytes = js_sys::Uint8Array::new(&abuf).to_vec();
            match protocol::decode(&bytes) {
                Ok(ServerMessage::State { tick, food, snakes }) => {
                    log::debug!("state tick={tick} snakes={}", snakes.len());
                    state_msg.borrow_mut().apply_state(food, snakes);
                }
                Ok(ServerMessage::Crown { name, crowns }) => {
                    log::info!("crown: {name} now has {crowns} crowns");
                    state_msg.borrow_mut().apply_crown(name, crowns);
                }
                Ok(ServerMessage::Leaderboard { players }) => {
                    log::debug!("leaderboard: {} players", players.len());
                    state_msg.borrow_mut().apply_leaderboard(players);
                }
                Ok(ServerMessage::Error { msg }) => {
                    log::warn!("server error: {msg}");
                }
                Err(e) => {
                    log::warn!("failed to decode message: {e}");
                }
            }
        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            log::info!("received text: {:?}", txt);
        } else {
            log::warn!("received unknown message type");
        }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // --- onerror ---
    let onerror = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        log::error!("WebSocket error: {:?}", e.message());
    });
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    // --- onopen — flush any queued join message ---
    let ws_open = ws.clone();
    let onopen = Closure::<dyn FnMut()>::new(move || {
        log::info!("WebSocket opened");
        if let Some(bytes) = state_open.borrow_mut().pending_join.take() {
            match ws_open.send_with_u8_array(&bytes) {
                Ok(_) => log::info!("join sent (flushed from onopen)"),
                Err(e) => log::error!("failed to send queued join: {:?}", e),
            }
        }
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    ws
}

#[wasm_bindgen]
impl Board {
    /// Initialise the board: build the DOM tile grid and open the WebSocket connection.
    pub fn create() -> Self {
        let document = window()
            .and_then(|w| w.document())
            .expect("failed to get document");

        let stage = document
            .query_selector(STAGE_SELECTOR)
            .expect("query_selector failed")
            .expect("no element found")
            .dyn_into::<HtmlElement>()
            .unwrap();

        let width = format!("{}px", COLS * (TILES_SIZE + TILES_MARGIN));
        let height = format!("{}px", ROWS * (TILES_SIZE + TILES_MARGIN));

        let style = stage.style();

        style.set_property("width", &width).unwrap();
        style.set_property("height", &height).unwrap();

        let state = Rc::new(RefCell::new(GameState::default()));
        let ws = ws_connect(Rc::clone(&state));
        log::info!("WebSocket connecting to {SERVER_URL}");

        Self {
            tiles: Tiles::build(&document, &stage),
            username: None,
            stage,
            ws,
            state,
        }
    }

    /// Register as a player.
    /// Sends the join message immediately if the socket is open,
    /// or queues it to be sent by the onopen handler if still connecting.
    pub fn join(&mut self, username: String) -> Result<()> {
        self.username = Some(username.clone());

        let bytes = encode(&ClientMessage::Join {
            username: username.clone(),
        })
        .map_err(|e| WebError::WebSocket(format!("{e}")))?;

        if self.ws.ready_state() == WebSocket::OPEN {
            self.ws
                .send_with_u8_array(&bytes)
                .map_err(|e| WebError::WebSocket(format!("{e:?}")))?;
            log::info!("join sent immediately for '{username}'");
        } else {
            // Still CONNECTING — onopen will flush it
            self.state.borrow_mut().pending_join = Some(bytes);
            log::info!("join queued for '{username}' (ws connecting)");
        }

        Ok(())
    }

    /// Send a direction change to the server.
    pub fn turn(&mut self, dir: Direction) -> Result<()> {
        let bytes = encode(&ClientMessage::Turn { dir: dir as u8 })
            .map_err(|e| WebError::WebSocket(format!("{e}")))?;
        self.ws
            .send_with_u8_array(&bytes)
            .map_err(|e| WebError::WebSocket(format!("{e:?}")))?;

        log::debug!("turn sent: {:?} ({})", dir, dir as u8);
        Ok(())
    }

    /// Render the current game state to the DOM tile grid.
    /// Called every animation frame from JS.
    pub fn draw(&mut self) -> Result<()> {
        let prev = self.tiles.clone();

        // Clear the board then paint the current state
        self.tiles.clear();

        let state = self.state.borrow();

        // Paint snakes — stable color per player
        for snake in state.snakes.iter() {
            let idx = state
                .player_color_idx
                .get(&snake.name)
                .copied()
                .unwrap_or(0);
            let color = snake_color(idx);
            for &[x, y] in &snake.body {
                self.tiles
                    .set_tile_kind(x as usize, y as usize, TileKind::Snake(color.clone()));
            }
        }

        // Paint food
        if let Some([fx, fy]) = state.food {
            self.tiles
                .set_tile_kind(fx as usize, fy as usize, TileKind::Food);
        }

        drop(state); // release borrow before calling refresh
        self.tiles.refresh(&prev);

        Ok(())
    }

    /// Returns crown counts as JSON — sorted by crowns descending.
    pub fn crowns(&self) -> String {
        serde_json::to_string(&self.state.borrow().crowns).unwrap_or_else(|_| "[]".to_string())
    }

    /// Returns the full leaderboard as JSON with per-player worm color, updated every 25 ticks.
    /// Shape: `[{"name":"alice","crowns":5,"length":4,"alive":true,"color":"#F24"}, ...]`
    pub fn leaderboard(&self) -> String {
        #[derive(serde::Serialize)]
        struct Entry<'a> {
            name: &'a str,
            crowns: u32,
            length: u16,
            alive: bool,
            color: String,
        }
        let state = self.state.borrow();
        let entries: Vec<Entry> = state
            .leaderboard
            .iter()
            .map(|p| {
                let idx = state.player_color_idx.get(&p.name).copied().unwrap_or(0);
                Entry {
                    name: &p.name,
                    crowns: p.crowns,
                    length: p.length,
                    alive: p.alive,
                    color: snake_color(idx),
                }
            })
            .collect();
        serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
    }
}
