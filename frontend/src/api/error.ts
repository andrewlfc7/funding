// src/api/error.ts
import axios from 'axios'

export function handleApiError(error: unknown, context = 'API request'): string {
  if (axios.isAxiosError(error)) {
    const status = error.response?.status
    const url = error.config?.url
    console.error(`${context} failed`, { status, url, detail: error.message, data: error.response?.data })

    if (status === 404) return 'Endpoint not found. Check backend routes & frontend config.'
    if (status === 500) return 'Server error. Please try again later.'
    if (error.code === 'ECONNABORTED') return 'Request timeout. Check your connection.'
    return error.message || 'Request failed.'
  }

  console.error(`${context} failed`, error)
  return error instanceof Error ? error.message : 'Unexpected error occurred.'
}
