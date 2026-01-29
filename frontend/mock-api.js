// Mock API server for local development
// Run with: node mock-api.js

const http = require('http');
const url = require('url');

const PORT = 3001;

// Mock user database
const mockUsers = {
    'testuser': {
        password: 'TestPassword123',
        email: 'test@example.com',
        user_id: 'mock-user-id-123'
    }
};

// Mock responses
function handleRequest(req, res) {
    const parsedUrl = url.parse(req.url, true);
    const path = parsedUrl.pathname;

    // CORS headers
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');

    if (req.method === 'OPTIONS') {
        res.writeHead(200);
        res.end();
        return;
    }

    let body = '';
    req.on('data', chunk => { body += chunk; });
    req.on('end', () => {
        try {
            const data = body ? JSON.parse(body) : {};

            // Route handling
            if (path === '/api/auth/login' && req.method === 'POST') {
                handleLogin(data, res);
            } else if (path === '/api/auth/signup' && req.method === 'POST') {
                handleSignup(data, res);
            } else if (path === '/api/auth/forgot-password' && req.method === 'POST') {
                handleForgotPassword(data, res);
            } else if (path === '/api/auth/confirm-forgot-password' && req.method === 'POST') {
                handleConfirmForgotPassword(data, res);
            } else if (path === '/api/games' && req.method === 'GET') {
                handleGetGames(res);
            } else {
                res.writeHead(404, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({ error: 'Not found' }));
            }
        } catch (error) {
            res.writeHead(500, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'Internal server error' }));
        }
    });
}

function handleLogin(data, res) {
    const user = mockUsers[data.username];
    if (user && user.password === data.password) {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
            access_token: 'mock-token-' + Date.now(),
            refresh_token: 'mock-refresh-token',
            id_token: 'mock-id-token',
            expires_in: 3600,
            token_type: 'Bearer'
        }));
    } else {
        res.writeHead(401, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
            error: 'invalid_credentials',
            message: 'Invalid credentials'
        }));
    }
}

function handleSignup(data, res) {
    if (mockUsers[data.username]) {
        res.writeHead(409, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
            error: 'user_exists',
            message: 'User already exists'
        }));
    } else {
        mockUsers[data.username] = {
            password: data.password,
            email: data.email,
            user_id: 'mock-user-' + Date.now()
        };
        res.writeHead(201, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
            user_id: mockUsers[data.username].user_id,
            email: data.email,
            username: data.username,
            message: 'User created successfully'
        }));
    }
}

function handleForgotPassword(data, res) {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
        message: 'Password reset code sent to email (mock: code is 123456)'
    }));
}

function handleConfirmForgotPassword(data, res) {
    if (data.confirmation_code === '123456') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
            message: 'Password has been reset successfully'
        }));
    } else {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
            error: 'invalid_code',
            message: 'Invalid verification code'
        }));
    }
}

function handleGetGames(res) {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
        games: [
            {
                game_id: 'mini-zork',
                title: 'DORK I: The Last Great Empire',
                description: 'A miniature test adventure',
                version: 3,
                file_size: 28672
            }
        ]
    }));
}

const server = http.createServer(handleRequest);
server.listen(PORT, () => {
    console.log(`Mock API server running at http://localhost:${PORT}`);
    console.log('Test credentials: testuser / TestPassword123');
    console.log('Password reset code: 123456');
});
