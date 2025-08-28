import axios from 'axios'
import { API_BASE_URL, API_ENDPOINT } from './constants'
import type { ApiResponse } from './types'

export async function fetchFundingRates(): Promise<ApiResponse> {
  console.log('Fetching data from:', `${API_BASE_URL}${API_ENDPOINT}`)
  const response = await axios.get<ApiResponse>(`${API_BASE_URL}${API_ENDPOINT}`)
  
  // Log the raw response
  console.log('Raw API Response:', response.data)
  
  // Validate response
  if (!response.data.tokens || !Array.isArray(response.data.tokens)) {
    throw new Error('Invalid API response: missing or invalid tokens array')
  }
  
  // Log statistics
  console.log(`Received ${response.data.tokens.length} tokens`)
  if (response.data.tokens.length > 0) {
    console.log('Sample tokens (first 3):', response.data.tokens.slice(0, 3))
  }
  
  return response.data
}

export function handleApiError(error: unknown): string {
  console.error('Failed to fetch funding rates:', error)
  
  if (axios.isAxiosError(error)) {
    console.error('Axios error details:', {
      message: error.message,
      status: error.response?.status,
      statusText: error.response?.statusText,
      data: error.response?.data,
      url: error.config?.url
    })
    
    if (error.response?.status === 404) {
      return 'API endpoint not found. Please check your configuration.'
    } else if (error.response?.status === 500) {
      return 'Server error. Please try again later.'
    } else if (error.code === 'ECONNABORTED') {
      return 'Request timeout. Please check your connection.'
    }
  }
  
  return error instanceof Error ? error.message : 'Failed to fetch funding rates'
}