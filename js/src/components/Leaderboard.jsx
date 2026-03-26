import React from 'react';

function Leaderboard({ players, username }) {
  return (
    <aside className="leaderboard">
      <h2 className="leaderboard__title">Leaderboard</h2>
      {(!players || players.length === 0) ? (
        <p className="leaderboard__empty">Waiting for players…</p>
      ) : (
        <ol className="leaderboard__list">
          {players.map((p, i) => (
            <li
              key={p.name}
              className={`leaderboard__row ${!p.alive ? 'leaderboard__row--dead' : ''}`}
            >
              <span className="leaderboard__rank">#{i + 1}</span>
              {/* Name colored to match the player's worm */}
              <span
                className="leaderboard__name"
                style={{ color: p.color ?? '#fff', fontWeight: p.name === username ? 'bold' : 'normal' }}
              >
                {p.name}
              </span>
              <span className="leaderboard__meta">
                {p.crowns > 0 && (
                  <span className="leaderboard__crown">👑 {p.crowns}</span>
                )}
                {/* Length always shown as two digits: 04, 15 */}
                <span className="leaderboard__length">
                  ⬜ {String(p.length).padStart(2, '0')}
                </span>
              </span>
            </li>
          ))}
        </ol>
      )}
    </aside>
  );
}

export default Leaderboard;
