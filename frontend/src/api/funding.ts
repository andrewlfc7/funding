// src/api/funding.ts
import axios from 'axios'
import { API_BASE_URL, FUNDING_ENDPOINT, joinUrl } from '@/utils/constants'
import type { ApiResponse } from '@/utils/types'
import { handleApiError } from './error'

export async function fetchFundingRates(): Promise<ApiResponse> {
  try {
    const url = joinUrl(API_BASE_URL, FUNDING_ENDPOINT)
    console.log('[Funding] GET', url)
    const { data } = await axios.get<ApiResponse>(url)
    return data
  } catch (e) {
    throw new Error(handleApiError(e, 'fetchFundingRates'))
  }
}
