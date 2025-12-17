// Development configuration
// Controls whether to use production API or mock API

const DEV_CONFIG = {
    // Set to 'mock' for local testing without backend, 'production' to test against real API
    apiMode: 'production',

    // API endpoints
    productionApi: 'https://api.gruesome.skeptomai.com',
    stagingApi: 'https://api-staging.gruesome.skeptomai.com',
    mockApi: 'http://localhost:3001',

    // Get current API base URL
    getApiBase() {
        // Check if running locally
        if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
            return this.apiMode === 'mock' ? this.mockApi : this.productionApi;
        }

        // Check if on staging
        if (window.location.hostname === 'staging.gruesome.skeptomai.com') {
            return this.stagingApi;
        }

        // Production
        return this.productionApi;
    }
};

// Export for use in app.js
window.DEV_CONFIG = DEV_CONFIG;
