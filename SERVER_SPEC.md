# Oxidar Multiplayer Snake — Game Server Spec

## Project Overview

**Project name**: oxidar-snake

**Description**: A multiplayer snake game server built in Rust for the [oxidar.org](https://oxidar.org) coding session. Players connect via WebSocket, control their snake on a shared board, and compete for food to grow and earn crowns.

**Purpose**: Provide the authoritative game server that session participants connect to with their own custom-built clients (terminal, web, native). The server defines the protocol contract; participants are free to implement clients however they want.

**Goals**:

- Server-authoritative game loop with fixed tick rate
- MessagePack binary protocol over WebSocket
- Support up to 32 concurrent players plus spectators
- Reconnection support (resume where you left off)
- Leaderboard based on crowns earned

---

## Tech Stack

- **Language**: Rust (2024 edition)
- **Async runtime**: `tokio`
- **WebSocket**: `tokio-tungstenite`
- **Serialization**: `rmp-serde` (MessagePack), `serde` / `serde_derive`
- **Configuration**: `toml`
- **Error handling**: `anyhow`
- **Randomization**: `rand`
- **Observability**: `tracing` + `tracing-subscriber`
- **Testing**: built-in `#[cfg(test)]` modules + `tests/` directory for integration tests

---

## Functional Requirements

### Game Rules

- **Board**: 64×32 grid, toroidal (wraps on all edges)
- **Max players**: 32
- **Collision**: None — snakes overlap freely
- **Food**: Exactly 1 item on the board. When eaten, new food spawns at a random cell on the next tick
- **Starting length**: 4 segments
- **Win condition**: Reach length 16 → earn a **crown**, snake resets to length 4, keeps playing
- **Crowns**: Count per player, purely for leaderboard
- **Death**: None — snakes never die
- **Movement**: Classic snake — always moving, player only changes direction. Cannot reverse (UP↔DOWN, LEFT↔RIGHT)

### Connection & Identity

- **Transport**: WebSocket, binary frames, MessagePack-encoded
- **Player**: Connects and sends a `join` message with a `username` → becomes a player
- **Spectator**: Connects but never sends `join` → receives broadcasts, cannot send moves
- **Reconnect**: Same username reconnects to the same snake (position, direction, length, crowns preserved). Snake resumes moving immediately on next tick
- **Disconnect**: Snake removed from the board immediately. State preserved server-side for 60s (configurable), then purged

### Server Tick Model (default 200ms)

Each tick:

1. Apply queued direction changes (last valid input per player)
2. Move all snakes one cell in current direction (wrap on edges)
3. Check food: if head == food, grow snake, spawn new food
4. Check win: if length == target, award crown, reset to start length
5. Broadcast state to all connections

### Protocol Messages

**Client → Server:**

`join` — register as player:

```
{ "type": "join", "username": "alice" }
```

`turn` — change direction:

```
{ "type": "turn", "dir": 0 }
```

Directions: 0=Up, 1=Right, 2=Down, 3=Left

**Server → Client:**

`state` — broadcast every tick:

```
{
  "type": "state",
  "tick": 12345,
  "food": [12, 7],
  "snakes": [
    { "name": "alice", "body": [[10,5],[10,6],[10,7],[10,8]], "dir": 0, "crowns": 2 }
  ]
}
```

`crown` — broadcast on crown earned:

```
{ "type": "crown", "name": "alice", "crowns": 3 }
```

`leaderboard` — broadcast every 25 ticks (5s):

```
{
  "type": "leaderboard",
  "players": [
    { "name": "alice", "crowns": 5, "length": 12, "alive": true },
    { "name": "bob", "crowns": 3, "length": 7, "alive": false }
  ]
}
```

- `alive`: true if currently connected
- `length`: current snake length (0 if disconnected)

`error` — sent to single client:

```
{ "type": "error", "msg": "username already connected" }
```

### Configuration (TOML)

```toml
[game]
board_width = 64
board_height = 32
max_players = 32
tick_ms = 200
snake_start_length = 4
snake_win_length = 16
disconnect_timeout_s = 60
leaderboard_interval_ticks = 25

[server]
host = "0.0.0.0"
port = 9001
```

---

## Non-Functional Requirements

### Git Workflow

- Every task = one commit
- Commit format: `type: description`
  - Types: `feat`, `fix`, `test`, `docs`, `chore`, `refactor`
- Never commit broken code — tests must pass before committing
- Repository initialized in Phase 1 with `.gitignore`

### Testing

- Every implementation task must include unit tests
- Integration tests in a dedicated phase (Phase 4)
- All tests must pass before committing: `cargo test`

### Observability

- `tracing` subscriber initialized on startup
- Key functions instrumented with `#[instrument]` or `tracing::info!` / `tracing::warn!` / `tracing::error!`:
  - Player join/disconnect/reconnect events
  - Crown awards
  - Errors (malformed messages, duplicate usernames, capacity exceeded)
  - Game loop start/tick count (periodic, not every tick)

### Performance

- Fixed tick rate must not drift under normal load (use `tokio::time::interval`)
- MessagePack serialization keeps per-tick broadcast compact

---

## Project Structure

```
oxidar-snake/
├── .gitignore
├── Cargo.toml
├── config.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── game/
│   │   ├── mod.rs
│   │   ├── board.rs
│   │   ├── snake.rs
│   │   └── engine.rs
│   ├── net/
│   │   ├── mod.rs
│   │   ├── server.rs
│   │   ├── session.rs
│   │   └── protocol.rs
│   └── leaderboard.rs
└── tests/
    └── integration.rs
```

---

## Session State

| Field               | Value       |
| ------------------- | ----------- |
| Current session     | 3           |
| Last completed task | 6.3         |
| Status              | In Progress |

---

## Task Checklist

### Phase 1: Project Setup

- [x] **Task 1.1**: Initialize project and git repository
  - Run `cargo init --name oxidar-snake`
  - Create `.gitignore` (include `/target`, `*.swp`, `.DS_Store`, `*.log`)
  - Initialize git repository
  - **Unit tests**: None (scaffold only)
  - Commit: `chore: initialize project and git repository`

- [x] **Task 1.2**: Create configuration system
  - Create `config.toml` with all default values from the reference above
  - Create `src/config.rs`:
    - `Config` struct with `game: GameConfig` and `server: ServerConfig`
    - `GameConfig` struct: `board_width`, `board_height`, `max_players`, `tick_ms`, `snake_start_length`, `snake_win_length`, `disconnect_timeout_s`, `leaderboard_interval_ticks`
    - `ServerConfig` struct: `host`, `port`
    - `Config::load(path: &str) -> anyhow::Result<Config>`
  - Update `src/main.rs`: load config from `config.toml` (or CLI arg path), initialize `tracing_subscriber`, print config, exit
  - Add all crate dependencies to `Cargo.toml`
  - Add tracing: log loaded config values at `info` level
  - **Unit tests**: `Config::load` with a valid TOML string, verify all fields parsed correctly
  - Commit: `feat: add TOML configuration loading and tracing setup`

### Phase 2: Game Logic

- [x] **Task 2.1**: Implement board and coordinate system
  - Create `src/game/mod.rs` with module declarations
  - Create `src/game/board.rs`:
    - `Position { x: u16, y: u16 }` — a cell coordinate (derive `Clone`, `Copy`, `PartialEq`, `Eq`, `Debug`)
    - `Board` struct: `width: u16`, `height: u16`, `food: Position`
    - `Board::new(width, height) -> Board` — spawns initial food at random position
    - `Board::wrap(&self, x: i32, y: i32) -> Position` — wraps coordinates toroidally
    - `Board::spawn_food(&mut self, rng: &mut impl Rng)` — places food at random cell
    - `Board::food(&self) -> Position`
  - Wire `game` module into `main.rs`
  - **Unit tests**:
    - `wrap(64, 0)` on 64-wide board → `(0, 0)`
    - `wrap(-1, 0)` → `(63, 0)`
    - `wrap(0, -1)` on 32-tall board → `(0, 31)`
    - `wrap(0, 32)` → `(0, 0)`
    - Food spawning produces position within board bounds
  - Commit: `feat: add toroidal board with coordinate wrapping and food spawning`

- [x] **Task 2.2**: Implement snake state and movement
  - Create `src/game/snake.rs`:
    - `Direction` enum: `Up(0)`, `Right(1)`, `Down(2)`, `Left(3)` with `#[repr(u8)]`. Implement `opposite(&self) -> Direction`
    - `Snake` struct: `name: String`, `body: VecDeque<Position>`, `dir: Direction`, `crowns: u32`, `next_dir: Option<Direction>`, `growing: u32`
    - `Snake::new(name, start_pos, dir, length, board) -> Snake` — body trails behind head using `board.wrap()`
    - `Snake::queue_turn(&mut self, dir: Direction)` — set `next_dir` if not a reversal, else ignore
    - `Snake::apply_turn(&mut self)` — apply queued direction if present
    - `Snake::advance(&mut self, board: &Board)` — move head one cell, pop tail unless `growing > 0`
    - `Snake::grow(&mut self)` — increment `growing` counter
    - `Snake::head(&self) -> Position`
    - `Snake::len(&self) -> usize`
  - **Unit tests**:
    - Advance: head moves, length unchanged
    - Grow then advance: length increases by 1
    - Queue reversal: ignored (direction unchanged)
    - Queue valid turn, apply, advance: direction changed
    - Movement wraps at board edges
  - Commit: `feat: add snake with movement, growth, and direction validation`

- [x] **Task 2.3**: Implement game engine tick logic
  - Create `src/game/engine.rs`:
    - `GameEngine` struct: `board: Board`, `active: HashMap<String, Snake>`, `disconnected: HashMap<String, (Snake, Instant)>`, `tick: u64`, `start_length`, `win_length`, `max_players`
    - `GameEngine::new(config: &GameConfig) -> GameEngine`
    - `GameEngine::add_player(&mut self, name: String) -> Result<()>` — reconnect from disconnected map if exists, else create new. Error if already active or at capacity
    - `GameEngine::remove_player(&mut self, name: &str)` — move to disconnected map with timestamp
    - `GameEngine::queue_turn(&mut self, name: &str, dir: Direction)`
    - `GameEngine::tick(&mut self) -> TickResult` — apply turns, move snakes, check food, check win, return snapshot
    - `GameEngine::purge_stale(&mut self, timeout_s: u64)` — remove expired disconnected snakes
    - `TickResult` struct: `tick`, `food`, `snakes: Vec<SnakeState>`, `crowns: Vec<CrownEvent>`
    - `SnakeState`: `name`, `body`, `dir`, `crowns`
    - `CrownEvent`: `name`, `crowns`
  - Add tracing: `info!` on player add/remove/reconnect, `info!` on crown awarded, `warn!` on capacity exceeded
  - **Unit tests**:
    - Add 2 players, tick, verify both moved
    - Snake head on food → tick → snake grew, food respawned elsewhere
    - Snake reaches win length → tick → crown awarded, snake reset to start length
    - Remove player → not in active. Reconnect → back in active with preserved state
    - Add player at max capacity → error
    - Purge stale: disconnected snake older than timeout is removed
  - Commit: `feat: add game engine with tick logic, crown detection, and reconnection`

### Phase 3: Networking & Protocol

- [x] **Task 3.1**: Define MessagePack protocol types
  - Create `src/net/mod.rs` with module declarations
  - Create `src/net/protocol.rs`:
    - `ClientMessage` enum (serde internally tagged by `"type"`):
      - `Join { username: String }`
      - `Turn { dir: u8 }`
    - `ServerMessage` enum (serde internally tagged by `"type"`):
      - `State { tick: u64, food: [u16; 2], snakes: Vec<SnakeData> }`
      - `Crown { name: String, crowns: u32 }`
      - `Leaderboard { players: Vec<LeaderboardEntry> }`
      - `Error { msg: String }`
    - `SnakeData`: `name`, `body: Vec<[u16; 2]>`, `dir: u8`, `crowns: u32`
    - `LeaderboardEntry`: `name`, `crowns`, `length: u16`, `alive: bool`
    - `encode(msg: &ServerMessage) -> Vec<u8>`
    - `decode(bytes: &[u8]) -> Result<ClientMessage>`
    - `impl From<&TickResult> for ServerMessage` (State variant)
    - `impl From<&CrownEvent> for ServerMessage` (Crown variant)
  - Wire `net` module into `main.rs`
  - **Unit tests**:
    - Round-trip: encode State → decode → verify fields
    - Decode valid Join message from raw msgpack bytes
    - Decode valid Turn message
    - Decode invalid bytes → error
  - Commit: `feat: add MessagePack protocol types with encode/decode`

- [x] **Task 3.2**: Implement session management
  - Create `src/net/session.rs`:
    - `SessionId(u64)` — unique per connection
    - `Session` enum: `Player { username: String }` | `Spectator`
    - `SessionManager` struct:
      - `connect(&mut self) -> SessionId` — register as spectator
      - `promote(&mut self, id: SessionId, username: String) -> Result<()>` — error if username taken
      - `disconnect(&mut self, id: SessionId) -> Option<Session>`
      - `get(&self, id: SessionId) -> Option<&Session>`
      - `player_sessions(&self) -> impl Iterator<Item = (SessionId, &str)>`
      - `all_sessions(&self) -> impl Iterator<Item = SessionId>`
  - Add tracing: `info!` on connect/promote/disconnect, `warn!` on duplicate username
  - **Unit tests**:
    - Connect → spectator. Promote → player. Disconnect → removed
    - Promote two sessions with same username → error on second
    - Disconnect player, promote new session with same username → success
  - Commit: `feat: add session manager for player and spectator tracking`

- [x] **Task 3.3**: Implement leaderboard
  - Create `src/leaderboard.rs`:
    - `Leaderboard::compute(engine: &GameEngine) -> Vec<LeaderboardEntry>`
    - Includes active snakes (alive=true, length=current) and disconnected (alive=false, length=0)
    - Sorted: crowns descending, then length descending
  - Wire module into `main.rs`
  - **Unit tests**:
    - 3 players with different crown counts → correct ordering
    - Tie in crowns → length tiebreaker
    - Mix connected/disconnected → correct alive flags and length=0 for disconnected
  - Commit: `feat: add leaderboard ranking computation`

- [x] **Task 3.4**: Implement WebSocket server and connection handling
  - Create `src/net/server.rs`:
    - `tokio-tungstenite` WebSocket server on configured host:port
    - On connect: assign `SessionId`, register as spectator
    - On binary message: decode → handle `Join` (promote + add_player) or `Turn` (queue_turn). Send `error` on invalid states
    - On disconnect: remove session, remove player from engine if was player
    - Maintain a sender handle per connection for broadcasting
  - Add tracing: `info!` on new connection, `info!` on join, `warn!` on errors sent to clients
  - **Unit tests**: None (requires integration context — tested in Phase 4)
  - Commit: `feat: add WebSocket server with connection and message handling`

- [x] **Task 3.5**: Wire game loop — ticking and broadcasting
  - Modify `src/net/server.rs` (or create `src/game_loop.rs`):
    - Spawn `tokio::time::interval` task at configured tick rate
    - Each tick: `engine.tick()` → encode `State` → broadcast to all connections
    - Every `leaderboard_interval_ticks` ticks: compute leaderboard → broadcast
    - On crown events: broadcast `Crown` messages
    - Periodically: `engine.purge_stale(disconnect_timeout_s)`
  - Modify `src/main.rs`: wire config → engine → server → game loop
  - Add tracing: `info!` on game loop start, `debug!` every 100 ticks with player count
  - **Unit tests**: None (tested via integration in Phase 4)
  - Commit: `feat: wire game loop with tick broadcasting and leaderboard`

### Phase 4: Verification & Integration

- [x] **Task 4.1**: Spectator mode verification
  - Verify/fix:
    - Connect without sending `join` → receives `state` and `leaderboard` broadcasts
    - Spectator sends `turn` → receives `error`
    - Spectators not in snake list
    - Spectators don't count toward `max_players`
  - Add tracing: `warn!` when spectator attempts to send a turn
  - **Unit tests**: Add test cases to session manager: spectator cannot queue turns
  - Commit: `test: verify and harden spectator mode`

- [x] **Task 4.2**: Reconnection logic verification
  - Verify/fix:
    - Connect as "alice", disconnect, reconnect → same position, direction, length, crowns
    - Snake resumes moving immediately on next tick
    - No duplicate "alice" in snake list
    - Second socket with same username while first connected → `error`
    - Reconnect after `disconnect_timeout_s` → fresh snake (state purged)
  - **Unit tests**: Add engine tests for reconnection edge cases
  - Commit: `test: verify and harden reconnection logic`

- [x] **Task 4.3**: Error handling and edge cases
  - Verify/fix:
    - Non-MessagePack binary data → `error` response, connection stays open
    - `turn` with `dir=5` (invalid) → silently ignored or error
    - `join` with empty username → error
    - Multiple `join` on same connection → error on second
    - 32 players + 33rd → error
    - Text WebSocket frames → error or ignored
  - Add tracing: `warn!` on each malformed/invalid input
  - **Unit tests**: Add protocol decode tests for edge cases, engine tests for capacity
  - Commit: `fix: handle malformed input and edge cases gracefully`

- [x] **Task 4.4**: Integration test
  - Create `tests/integration.rs`:
    - Start server in background tokio task with test config (small board, fast tick)
    - Connect 2 clients, send `join` for each
    - Send `turn` from client 1
    - Receive `state` messages → both snakes present and moving
    - Disconnect client 1 → client 2 receives state without client 1's snake
    - Reconnect client 1 → snake reappears
    - Connect spectator (no join) → receives state
  - **Unit tests**: N/A (this is the integration test)
  - Commit: `test: add end-to-end integration test`

### Phase 5: Documentation

- [x] **Task 5.1**: Write client developer README
  - Create `README.md` targeted at session attendees building their own clients
  - Content:
    - Brief project overview (what the server is, what attendees will build)
    - How to run the server (`cargo run`, config options)
    - Connection details (WebSocket URL, binary frames, MessagePack)
    - Full protocol reference: all client→server and server→client messages with field descriptions
    - Direction encoding (0=Up, 1=Right, 2=Down, 3=Left)
    - Game rules summary (board size, wrapping, food, crowns, no death)
    - Reconnection behavior
    - Spectator mode
    - Quick-start example flow (connect → join → receive state → send turns)
    - Tips for common languages/libraries (Python, JS/TS, Rust) for WebSocket + MessagePack
  - Commit: `docs: add client developer README for session attendees`

### Phase 6: Deployment

- [x] **Task 6.1**: Add Dockerfile with multi-stage build
  - Create `Dockerfile` with two stages:
    - **Builder**: `rust:1.93.1-slim-bullseye`
      - Install build dependencies
      - Copy `Cargo.toml`, `Cargo.lock`, and `src/`
      - Build release binary with `cargo build --release`
    - **Runtime**: `debian:bookworm-slim`
      - Install minimal runtime dependencies (`ca-certificates`, `curl` for health checks)
      - Copy compiled binary from builder stage
      - Copy `config.toml` as default config
      - Expose ports 9001 (WebSocket) and 9002 (health check)
      - Add `HEALTHCHECK CMD curl -f http://localhost:9002/health || exit 1`
      - Set entrypoint to the binary
  - Create `.dockerignore` (target/, .git/, \*.md, tests/)
  - **Unit tests**: None (Docker build verification)
  - Commit: `feat: add multi-stage Dockerfile for build and runtime`

- [x] **Task 6.2**: Add health check endpoint, PORT env var support, and graceful shutdown
  - Add a `GET /health` HTTP endpoint on a **dedicated health port** (default 9002, configured via `health_port` in `config.toml`) — separate from the WebSocket port to keep concerns cleanly separated
  - Read WebSocket port from `PORT` env var with fallback to `config.toml` value (`std::env::var("PORT")`)
  - Handle `SIGTERM` for graceful shutdown: stop accepting new connections, let in-flight games drain, then exit (`tokio::signal::unix::signal(SignalKind::terminate())`)
  - **Unit tests**: Test that the health endpoint responds with 200
  - Commit: `feat: add health check, PORT env var, and graceful shutdown`

- [x] **Task 6.3**: Deploy to Railway
  - Create a Railway project linked to the repository
  - Configure Railway to build using the Dockerfile from Task 6.1
  - Railway's `PORT` env var is read automatically by the app (Task 6.2)
  - Generate a public Railway domain for WebSocket access
  - Verify the deployed service starts and accepts WebSocket connections
  - Verify `/health` endpoint responds through the Railway domain
  - **Unit tests**: None (deployment verification)
  - Commit: `feat: add Railway deployment configuration`

---

## Implementation Notes

### Rust Patterns

- Use `#[derive(Debug, Clone, Serialize, Deserialize)]` liberally on all data structs
- Use `anyhow::Result` for all fallible functions — keep error handling simple and consistent
- Prefer `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared game state between the connection handler tasks and the game loop task
- Consider a channel-based architecture: connection tasks send commands (`Join`, `Turn`, `Disconnect`) to the game loop via `tokio::sync::mpsc`, and the game loop broadcasts state via `tokio::sync::broadcast`

### MessagePack with Serde

- Use `#[serde(tag = "type")]` for internally tagged enum representation
- `rmp_serde::to_vec_named` for encoding (preserves field names for self-describing format)
- `rmp_serde::from_slice` for decoding
- Example:
  ```rust
  #[derive(Serialize, Deserialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  enum ClientMessage {
      Join { username: String },
      Turn { dir: u8 },
  }
  ```

### Testing Patterns

- Unit tests: `#[cfg(test)] mod tests { ... }` at the bottom of each source file
- For board/snake tests, create a small board (e.g. 8×8) to make wrapping easy to reason about
- For engine tests, use a deterministic RNG seed (`rand::SeedableRng`) so food placement is predictable
- Integration tests: `tests/integration.rs` using `tokio::test`

### Git Commit Guidelines

- Format: `type: description`
- Types: `feat`, `fix`, `test`, `docs`, `chore`, `refactor`
- Keep descriptions concise and lowercase
- Never commit code that doesn't compile or has failing tests

### Tracing / Observability

- Initialize `tracing_subscriber::fmt::init()` in `main`
- Use `#[instrument]` on key functions: `add_player`, `remove_player`, `tick`
- Use `info!` for business events (join, crown, disconnect)
- Use `warn!` for recoverable errors (malformed input, capacity exceeded)
- Use `error!` for unexpected failures
- Use `debug!` for periodic stats (tick count, player count)

---

## Instructions for Claude Code

### Workflow per task

1. **Implement** the code described in the task
2. **Write unit tests** as specified
3. **Run `cargo fmt`** — format all code
4. **Run `cargo test`** — all tests must pass
5. **Run `cargo clippy`** — no warnings
6. **Commit** with the specified commit message
7. **Update this SPEC.md**: mark the task as `[x]`, update Session State table

### Spec maintenance

- After completing each task, check off `[x]` the task in this file
- After each session, update the Session State section (session number, last task, status)

### Rules

- Never commit broken code
- Never skip tests
- If a task's tests fail, fix before committing
- If a task requires changes to a previous module's public API, that's fine — update the code and tests accordingly
- Keep tracing instrumentation consistent across all modules

---

## Human Review Points

### After Phase 1 (Setup)

- Run `cargo build` — should compile cleanly
- Run `cargo run` — should print config and exit
- Verify `.gitignore` includes `/target`
- Verify git log has 2 commits

### After Phase 2 (Game Logic)

- Run `cargo test` — all unit tests pass
- Review `src/game/` modules: board wrapping, snake movement, engine tick, crown logic
- Verify git log has 3 new commits (one per task)

### After Phase 3 (Networking)

- Run `cargo test` — all unit tests pass
- Start server: `cargo run` — should listen on port 9001
- Test with `websocat ws://localhost:9001` — connection should be accepted
- Send a msgpack `join` message — verify server logs the join event
- Verify git log has 5 new commits

### After Phase 4 (Verification & Integration)

- Run `cargo test` — all tests pass including integration test
- Start server, connect 2 clients, verify game state broadcasts
- Test spectator mode (connect without join)
- Test reconnection (disconnect and reconnect with same username)
- Test error cases (bad input, duplicate username, capacity)
- Verify git log has 4 new commits
- **Total: 14 commits across 14 tasks**

### After Phase 6 (Deployment)

- Run `docker build -t oxidar-snake .` — should build cleanly
- Run `docker run -p 9001:9001 -p 9002:9002 oxidar-snake` — server should start and accept connections
- Verify image size is minimal (runtime image based on bookworm-slim)
- Verify `GET /health` returns 200 on the health port (9002)
- Verify the server reads `PORT` from env: `PORT=8080 cargo run` should bind to 8080
- Verify graceful shutdown: send `SIGTERM` to a running server, confirm it exits cleanly
- Verify Railway deployment succeeds (check build and deploy logs)
- Connect to the Railway domain via WebSocket — server should accept connections
- Verify `/health` responds through the Railway domain

### How to run tests

```bash
# All tests (unit + integration)
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration

# With output
cargo test -- --nocapture

# Clippy
cargo clippy -- -D warnings
```

---

## Out of Scope

- Client implementations (that's the participants' job)
- Persistence / database — everything is in-memory
- HTTPS / TLS — run behind a reverse proxy if needed
- Authentication beyond username string
