// API Configuration
export const API_BASE_URL = import.meta.env.VITE_API_URL
export const API_ENDPOINT = import.meta.env.VITE_API_ENDPOINT
export const REFRESH_INTERVAL = parseInt(import.meta.env.VITE_REFRESH_INTERVAL)

// Validation
if (!API_BASE_URL) {
  throw new Error('VITE_API_URL environment variable is required')
}
if (!API_ENDPOINT) {
  throw new Error('VITE_API_ENDPOINT environment variable is required')
}
if (!REFRESH_INTERVAL || isNaN(REFRESH_INTERVAL)) {
  throw new Error('VITE_REFRESH_INTERVAL environment variable is required and must be a number')
}

// Constants for calculations
export const FUNDING_PERIODS_PER_DAY = 3 // Most exchanges have 8-hour funding periods
export const DAYS_PER_YEAR = 365

// Thresholds
export const MIN_SPREAD_THRESHOLD_BPS = 2
export const SPREAD_THRESHOLDS = {
  EXCELLENT: 15,
  GOOD: 8,
  DECENT: 4,
  HIGH: 10,
  MEDIUM: 5
}