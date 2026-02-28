# Oxidar Snake — Web Client Spec

## Project Overview

**Project name**: snake-web-client

**Description**: A browser-based client for the [oxidar-snake](./SERVER_SPEC.md) multiplayer snake game server. The client is implemented as a Rust WebAssembly module, with JavaScript acting only as a thin host layer — calling into WASM for all game logic, rendering, and networking.

**Server reference**: See [SERVER_SPEC.md](./SERVER_SPEC.md) for the full server spec: protocol, game rules, board dimensions, message formats, and deployment details.

**Architecture principle**: All logic lives in the Rust WASM crate (`rust/`). JavaScript only initializes the WASM module, passes user input, and calls the exposed `#[wasm_bindgen]` API. No game or network logic is implemented in JS.

---

## Architecture

```
Browser
├── JS layer (React / Vite)          ← thin host: init WASM, call API, handle DOM events
│   ├── main.jsx                     ← mounts React app
│   ├── App.jsx                      ← login flow, calls Board.create / board.connect / board.draw
│   └── components/Login.jsx         ← username input form
│
└── WASM layer (Rust → wasm-pack)    ← all logic lives here
    └── rust/src/lib.rs
        ├── Board                    ← top-level WASM-exported struct
        │   ├── create(token)        ← init board, build tile grid in DOM
        │   ├── connect()            ← open WebSocket, handle messages
        │   ├── send_turn(dir)       ← send direction change (msgpack)
        │   └── draw()              ← called each animation frame, renders state
        ├── Tiles / Tile             ← tile grid backed by DOM <div> elements
        ├── Snake / SnakeBlock       ← local snake state parsed from server messages
        ├── protocol (decode/encode) ← MessagePack parsing of server messages
        └── snake_color(id)          ← HSV color assignment per player
```

### Data flow

```
Server (msgpack/WS) → Board::on_message (Rust)
                   → parse TickState
                   → update self.snakes, self.food
                   → next draw() call → Tiles::refresh → DOM updates

User keypress → JS keydown listener → board.send_turn(dir: u8) → WS binary frame → Server
```

---

## Tech Stack

| Layer               | Technology                                                  |
| ------------------- | ----------------------------------------------------------- |
| Language (logic)    | Rust (2024 edition)                                         |
| WASM toolchain      | `wasm-pack`, `wasm-bindgen`                                 |
| JS framework        | React 19 (JSX via Vite)                                     |
| HTTP client (login) | `axios`                                                     |
| WebSocket           | `web-sys::WebSocket` (in Rust)                              |
| Serialization       | `rmp-serde` MessagePack (in Rust)                           |
| Rendering           | DOM `<div>` tiles (absolute positioned, managed by Rust)    |
| Build               | `wasm-pack build --target web` → output into `js/src/wasm/` |

---

## Functional Requirements

### Login

- User enters a username on the login screen
- JS posts `POST /login` with `{ name }` to the server
- Server returns a session `token`
- JS passes the token to `Board.create(token)` and calls `board.connect()`

### WebSocket connection (Rust)

- On `connect()`: open `ws://<server>/game/<token>` as binary (ArrayBuffer)
- On open: send `join` message (msgpack) with the username
- On message: decode msgpack → update internal game state
- On error / close: log and handle gracefully

### Protocol (Rust)

Mirrors the server protocol defined in [SERVER_SPEC.md](./SERVER_SPEC.md). The WASM crate owns all encode/decode.

**Client → Server (send):**

| Message                      | When                    |
| ---------------------------- | ----------------------- |
| `{ type: "join", username }` | On WebSocket open       |
| `{ type: "turn", dir: u8 }`  | On user direction input |

Directions: `0`=Up, `1`=Right, `2`=Down, `3`=Left

**Server → Client (receive):**

| Message       | Action                              |
| ------------- | ----------------------------------- |
| `state`       | Update `self.snakes`, `self.food`   |
| `crown`       | Update crown count for named player |
| `leaderboard` | Store for UI rendering              |
| `error`       | Log warning                         |

### Rendering (Rust)

- Board is a 64×32 grid of `<div class="tile">` elements with absolute CSS positioning
- Each tile is 10×10px with 1px margin
- `draw()` is called every `requestAnimationFrame` from JS
- `Tiles::refresh()` diffs old vs new state and only updates changed tiles
- Snake colors are assigned by player index using HSV color wheel (`snake_color(id)`)
- Food tile color: `#00FF00`
- Empty tile: no background color

### Direction input (JS → Rust)

- JS listens to `keydown` and calls `board.send_turn(dir: u8)` for arrow keys / WASD
- Rust validates the direction (no reversal) before sending to server

### Leaderboard / HUD (Rust or JS)

- Leaderboard data received from server every 5s (25 ticks)
- Render as an overlay or sidebar — all data sourced from WASM state
- JS may call `board.leaderboard()` to get a snapshot for rendering, or Rust mutates a DOM element directly

---

## Project Structure

```
snake-web-client/
├── build.sh                         ← builds Rust → WASM, outputs to js/src/wasm/
├── SERVER_SPEC.md                   ← server spec (reference)
├── CLIENT_SPEC.md                   ← this file
├── README.md                        ← setup and run instructions
│
├── rust/                            ← Rust WASM crate (all logic here)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                   ← Board, Tiles, Snake, protocol
│       ├── protocol.rs              ← msgpack encode/decode (ClientMessage, ServerMessage)
│       ├── game.rs                  ← game state types (TickState, SnakeState, LeaderboardEntry)
│       └── error.rs                 ← WebError, Result type
│
└── js/                              ← Vite + React app (thin host only)
    ├── package.json
    ├── vite.config.js
    ├── index.html
    └── src/
        ├── main.jsx                 ← React root
        ├── App.jsx                  ← init WASM, login flow, animation loop
        ├── components/
        │   └── Login.jsx
        └── wasm/                    ← generated by wasm-pack (do not edit)
```

---

## Build & Run

### 1. Build the WASM module

```bash
sh build.sh
```

This runs:

```bash
wasm-pack build rust/ --target web --out-dir js/src/wasm/
```

### 2. Run the dev server

```bash
cd js
npm install
npm start       # or: npm run dev
```

App is served at `http://localhost:5173` by default (Vite).

---

## WASM API (exposed to JS via `#[wasm_bindgen]`)

| Export               | Signature                              | Description                         |
| -------------------- | -------------------------------------- | ----------------------------------- |
| `setup_logs()`       | `fn()`                                 | Init `console_log` and panic hook   |
| `Board::create`      | `fn(token: String) -> Board`           | Build DOM tile grid, store token    |
| `Board::connect`     | `fn(&mut self) -> Result<()>`          | Open WebSocket to server            |
| `Board::send_turn`   | `fn(&mut self, dir: u8) -> Result<()>` | Encode and send direction change    |
| `Board::draw`        | `fn(&mut self) -> Result<()>`          | Render current state to DOM         |
| `Board::leaderboard` | `fn(&self) -> JsValue`                 | Return leaderboard snapshot as JSON |

> **Rule**: If logic can live in Rust, it must. JS only calls these exports.

---

## Non-Functional Requirements

### Separation of concerns

- **Rust**: WebSocket lifecycle, msgpack encoding/decoding, game state, color assignment, DOM tile mutation, direction validation
- **JS**: WASM initialization, login HTTP call, `requestAnimationFrame` loop, `keydown` listener, React component tree

### Testing

- Unit tests in `rust/src/` under `#[cfg(test)]`
- Test protocol encode/decode, color logic, tile state diffing

### Observability

- `console_error_panic_hook` registered on init (panics visible in browser console)
- `console_log` at `Debug` level via `log` crate macros
- Key events to log: WebSocket open/close/error, join sent, crown received, each state tick (at `debug` level)

---

## Session State

| Field               | Value       |
| ------------------- | ----------- |
| Current session     | 1           |
| Last completed task | —           |
| Status              | In Progress |

---

## Task Checklist

### Phase 1: Protocol

- [ ] **Task 1.1**: Add `rmp-serde` to `Cargo.toml` and create `src/protocol.rs`
  - `ClientMessage` enum: `Join { username }`, `Turn { dir: u8 }`
  - `ServerMessage` enum: `State`, `Crown`, `Leaderboard`, `Error`
  - `encode(msg: &ClientMessage) -> Vec<u8>`
  - `decode(bytes: &[u8]) -> Result<ServerMessage>`
  - Data types: `SnakeData`, `TickState`, `LeaderboardEntry`
  - **Unit tests**: round-trip encode/decode for each variant
  - Commit: `feat: add msgpack protocol types`

- [ ] **Task 1.2**: Create `src/game.rs` — game state types
  - `GameState`: `tick`, `food: [u16; 2]`, `snakes: Vec<SnakeData>`, `leaderboard: Vec<LeaderboardEntry>`
  - `GameState::apply_state(&mut self, tick: TickState)`
  - `GameState::apply_crown(&mut self, name: &str, crowns: u32)`
  - `GameState::apply_leaderboard(&mut self, entries: Vec<LeaderboardEntry>)`
  - **Unit tests**: state transitions
  - Commit: `feat: add game state model`

### Phase 2: Connection

- [ ] **Task 2.1**: Refactor `Board::connect` to use proper msgpack protocol
  - On open: encode and send `ClientMessage::Join { username }` (username stored in Board from login token)
  - On binary message: `decode()` → `match` on `ServerMessage` → update `self.state`
  - Replace placeholder `send_with_str` / `send_with_u8_array([0,1,2,3])` with real logic
  - Commit: `feat: wire WebSocket to msgpack protocol`

- [ ] **Task 2.2**: Implement `Board::send_turn(dir: u8)`
  - Encode `ClientMessage::Turn { dir }` → send as binary WS frame
  - Expose as `#[wasm_bindgen]`
  - Commit: `feat: implement send_turn wasm export`

### Phase 3: Input

- [ ] **Task 3.1**: Add `keydown` listener in `App.jsx`
  - Arrow keys and WASD → `board.send_turn(dir)` (0/1/2/3)
  - Remove listener on cleanup
  - Commit: `feat: wire keyboard input to wasm send_turn`

### Phase 4: Rendering

- [ ] **Task 4.1**: Update `Board::draw` to render live server state
  - Read `self.state.snakes` and `self.state.food` instead of dummy `0..50` loop
  - Assign snake colors by player index using `snake_color(id)`
  - Render food tile as `#00FF00`
  - Commit: `feat: render live game state in draw loop`

- [ ] **Task 4.2**: Add leaderboard rendering
  - Add `Board::leaderboard() -> JsValue` export (serialized to JSON)
  - Or: Rust mutates a dedicated DOM element directly
  - Commit: `feat: add leaderboard display`

### Phase 5: Polish

- [ ] **Task 5.1**: Error and disconnect handling
  - WS close/error: display reconnect UI or error message
  - Server `error` message: surface to user
  - Commit: `fix: handle disconnect and server errors`

- [ ] **Task 5.2**: Visual polish
  - Style the board, login screen, and leaderboard
  - Crown indicator next to snake name
  - Commit: `style: visual polish`
