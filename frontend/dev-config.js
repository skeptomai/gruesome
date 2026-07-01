// Frontend API configuration.
//
// Self-hosted deployment: the platform server serves BOTH this SPA and the
// `/api/*` routes on the same origin, so the API base is empty (relative) — e.g.
// `fetch('/api/games')`. That needs no per-environment URLs and no CORS.
//
// Set apiMode to 'mock' to develop the UI against frontend/mock-api.js without a
// backend. (The old AWS CloudFront/API-Gateway URLs have been removed.)

const DEV_CONFIG = {
    // 'same-origin' (self-hosted, default) or 'mock'
    apiMode: 'same-origin',

    mockApi: 'http://localhost:3001',
    selfHostedApi: '', // same origin — server hosts the SPA and /api together

    getApiBase() {
        return this.apiMode === 'mock' ? this.mockApi : this.selfHostedApi;
    }
};

// Export for use in app.js
window.DEV_CONFIG = DEV_CONFIG;
