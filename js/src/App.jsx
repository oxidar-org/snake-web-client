import React from 'react';
import './App.css';

import Login from './components/Login';
import Leaderboard from './components/Leaderboard';

import init, {
  setup_logs as SetupLogs,
  Board,
  Direction,
} from "./wasm";

const KEY_TO_DIRECTION = {
  ArrowUp: Direction.Up,
  ArrowRight: Direction.Right,
  ArrowDown: Direction.Down,
  ArrowLeft: Direction.Left,
  w: Direction.Up,
  d: Direction.Right,
  s: Direction.Down,
  a: Direction.Left,
};

function App() {
  const animationRef = React.useRef();
  const boardRef = React.useRef();
  const [username, setUsername] = React.useState();
  const [viewOnly, setViewOnly] = React.useState(false);
  const [players, setPlayers] = React.useState([]);

  React.useEffect(() => {
    if (username == undefined && !viewOnly) return;

    init().then(() => {
      SetupLogs();

      const board = Board.create();
      if (!viewOnly) {
        board.join(username);
      }
      boardRef.current = board;

      // --- keyboard input (only in play mode) ---
      let handleKey;
      if (!viewOnly) {
        handleKey = (e) => {
          const dir = KEY_TO_DIRECTION[e.key];
          if (dir !== undefined) {
            e.preventDefault();
            board.turn(dir);
          }
        };
        window.addEventListener('keydown', handleKey);
      }

      // --- render loop ---
      const drawLoop = () => {
        board.draw();
        animationRef.current = requestAnimationFrame(drawLoop);
      };
      drawLoop();

      // --- leaderboard polling (server broadcasts every 25 ticks ≈ 5 s) ---
      const pollLeaderboard = () => {
        try {
          const data = JSON.parse(board.leaderboard());
          setPlayers(data);
        } catch (err) { console.log(err) }
      };
      const lbInterval = setInterval(pollLeaderboard, 1000);

      return () => {
        if (handleKey) window.removeEventListener('keydown', handleKey);
        cancelAnimationFrame(animationRef.current);
        clearInterval(lbInterval);
      };
    });
  }, [username, viewOnly]);

  if (username == undefined && !viewOnly) {
    return <Login onLogin={(u) => setUsername(u)} onWatch={() => setViewOnly(true)} />;
  }

  return (
    <div className="game-layout">
      <div className="stage" />
      <Leaderboard players={players} username={username} />
    </div>
  );
}

export default App;
