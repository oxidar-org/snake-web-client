import React from 'react';
import './Login.css';

const Login = ({ onLogin, onWatch }) => {
    const [username, setUsername] = React.useState('');

    const handleSubmit = (e) => {
        e.preventDefault();

        if (username.trim()) {
            onLogin(username);
        }
    };

    return (
        <div className="login-container">
            <form className="login-form" onSubmit={handleSubmit}>
                <input
                    type="text"
                    placeholder="username"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    autoFocus
                />
                <button type="submit" disabled={!username.trim()}>
                    Start
                </button>
                <button type="button" onClick={onWatch}>
                    Watch
                </button>
            </form>
        </div>
    );
};

export default Login;