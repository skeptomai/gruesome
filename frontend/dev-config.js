// Frontend API configuration — supports BOTH hosting modes from one build.
//
// The backend base URL is chosen from the hostname the SPA is served from:
//   - AWS domains (CloudFront)      → the AWS API Gateway URL
//   - anything else (self-hosted)   → same-origin (the platform server serves
//                                     the SPA and /api together; relative URLs,
//                                     no CORS)
//
// Set apiMode to 'mock' to develop the UI against frontend/mock-api.js with no
// backend, or to 'aws'/'same-origin' to force a mode regardless of hostname.

const DEV_CONFIG = {
    // 'auto' (default) | 'mock' | 'aws' | 'same-origin'
    apiMode: 'auto',

    mockApi: 'http://localhost:3001',

    // AWS-hosted backend (used only when served from the AWS domains, or apiMode='aws').
    awsProductionApi: 'https://api.gruesome.skeptomai.com',
    awsStagingApi: 'https://api-staging.gruesome.skeptomai.com',

    // AWS CloudFront domains that should talk to the AWS API Gateway.
    awsHosts: {
        'gruesome.skeptomai.com': 'awsProductionApi',
        'staging.gruesome.skeptomai.com': 'awsStagingApi',
    },

    getApiBase() {
        if (this.apiMode === 'mock') return this.mockApi;
        if (this.apiMode === 'aws') return this.awsProductionApi;
        if (this.apiMode === 'same-origin') return '';
        // auto: AWS domains → AWS API; everything else → same-origin (self-hosted).
        const key = this.awsHosts[window.location.hostname];
        return key ? this[key] : '';
    }
};

// Export for use in app.js
window.DEV_CONFIG = DEV_CONFIG;
