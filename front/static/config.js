// Configuration file for frontend
// This file can be overridden in different environments

window.ENV = window.ENV || {};

// Default configuration
const defaultConfig = {
    API_BASE_URL: 'http://127.0.0.1:8080/api/v1',
    PEACH_ENTITY_ID: "8ac7a4c8961da56701961e61c57a0241"
};

// Merge with any existing environment variables
window.ENV = { ...defaultConfig, ...window.ENV };

console.log('Frontend configuration loaded:', window.ENV);