import type { ExchangeData, ArbOpportunity } from './types'
import { MIN_SPREAD_THRESHOLD_BPS } from './constants'

export function sumOpenInterest(exchanges: Record<string, ExchangeData>): number {
  return Object.values(exchanges).reduce((sum, ex) => sum + ex.open_interest, 0)
}

export function calculateArbSpread(exchanges: Record<string, ExchangeData>): number | null {
  const rates = Object.values(exchanges).map(ex => ex.funding_rate)
  
  if (rates.length < 2) return null
  
  const maxRate = Math.max(...rates)
  const minRate = Math.min(...rates)
  
  return (maxRate - minRate) * 10000 // Convert to basis points
}

export function findBestArbOpportunity(exchanges: Record<string, ExchangeData>): ArbOpportunity | null {
  const exchangeList = Object.entries(exchanges)
  if (exchangeList.length < 2) return null

  let bestArb: ArbOpportunity | null = null
  let maxSpread = 0

  // Check all exchange combinations
  for (let i = 0; i < exchangeList.length; i++) {
    for (let j = i + 1; j < exchangeList.length; j++) {
      const [ex1Name, ex1Data] = exchangeList[i]
      const [ex2Name, ex2Data] = exchangeList[j]
      
      const spread = Math.abs(ex1Data.funding_rate - ex2Data.funding_rate) * 10000
      
      if (spread > maxSpread && spread >= MIN_SPREAD_THRESHOLD_BPS) {
        const isEx1Higher = ex1Data.funding_rate > ex2Data.funding_rate
        
        maxSpread = spread
        bestArb = {
          longExchange: isEx1Higher ? ex2Name : ex1Name,
          shortExchange: isEx1Higher ? ex1Name : ex2Name,
          longRate: isEx1Higher ? ex2Data.funding_rate : ex1Data.funding_rate,
          shortRate: isEx1Higher ? ex1Data.funding_rate : ex2Data.funding_rate,
          spread: spread,
          combinedOI: ex1Data.open_interest + ex2Data.open_interest
        }
      }
    }
  }

  return bestArb
}