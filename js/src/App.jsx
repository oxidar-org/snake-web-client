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
  const [players, setPlayers] = React.useState([]);

  React.useEffect(() => {
    if (username == undefined) return;

    init().then(() => {
      SetupLogs();

      const board = Board.create();
      board.join(username);
      boardRef.current = board;

      // --- keyboard input ---
      const handleKey = (e) => {
        const dir = KEY_TO_DIRECTION[e.key];
        if (dir !== undefined) {
          e.preventDefault();
          board.turn(dir);
        }
      };
      window.addEventListener('keydown', handleKey);

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
        window.removeEventListener('keydown', handleKey);
        cancelAnimationFrame(animationRef.current);
        clearInterval(lbInterval);
      };
    });
  }, [username]);

  if (username == undefined) {
    return <Login onLogin={(u) => setUsername(u)} />;
  }

  return (
    <div className="game-layout">
      <div className="stage" />
      <Leaderboard players={players} />
    </div>
  );
}

export default App;
