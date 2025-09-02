// src/api/zscore/interAsset.ts
import {
  API_BASE_URL,
  ZSCORE_INTER_ASSET_ENDPOINT,
  joinUrl,
} from '@/utils/constants';

export type InterAssetParams = {
  coins?: string[];
  period: string;                 // '7d' | '30d' | '90d' | ...
  exchange: string;               // 'binance' | 'bybit' | ...
  marketType?: 'spot' | 'perps';
  topN?: number;
  window?: number;
  timeframe?: '1h' | '4h' | '1d';
  indexCoin?: string;
};

export type InterAssetZScoreResponse = {
  zscoreCorrelationMatrix: { coins: string[]; matrix: number[][] };
  zscoreBetaMatrix: { coins: string[]; matrix: number[][] }; // β vs index in column 0
  pairDivergence: Array<{
    pair: string; // "BTC-ETH"
    timeSeries: Array<{
      timestamp: number;
      zscore1: number;
      zscore2: number;
      divergence: number;
    }>;
  }>;
};

const BASE_URL = joinUrl(API_BASE_URL, ZSCORE_INTER_ASSET_ENDPOINT);

function toQuery(params: InterAssetParams): string {
  const q = new URLSearchParams();
  if (params.coins?.length) q.set('coins', params.coins.join(','));
  q.set('period', params.period);
  q.set('exchange', params.exchange);
  if (params.marketType) q.set('marketType', params.marketType);
  if (params.topN != null) q.set('topN', String(params.topN));
  if (params.window != null) q.set('window', String(params.window));
  if (params.timeframe) q.set('timeframe', params.timeframe);
  if (params.indexCoin) q.set('indexCoin', params.indexCoin);
  return q.toString();
}

export async function fetchInterAssetZScore(
  params: InterAssetParams,
  signal?: AbortSignal
): Promise<InterAssetZScoreResponse> {
  const url = `${BASE_URL}?${toQuery(params)}`;
  const res = await fetch(url, { signal });
  if (!res.ok) {
    const txt = await res.text().catch(() => '');
    throw new Error(`inter-asset zscore failed: ${res.status} ${txt}`);
  }
  return res.json();
}
