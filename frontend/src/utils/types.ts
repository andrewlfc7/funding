export interface ExchangeData {
  funding_rate: number;
  open_interest: number;
}

export interface TokenRow {
  token: string;
  exchanges: Record<string, ExchangeData>;
}

export interface ApiResponse {
  last_updated: string;
  tokens: TokenRow[];
}

export interface ArbOpportunity {
  longExchange: string;
  shortExchange: string;
  longRate: number;
  shortRate: number;
  spread: number;
  combinedOI: number;
}



export type LineDataset = {
  label: string;
  data: number[];
  borderColor: string;
  backgroundColor: string;
  borderWidth: number;
  tension: number;
  pointRadius: number;
  pointHoverRadius: number;
  fill: boolean;
};

export type MarketType = 'spot' | 'perps';


export interface MarketData {
  dates: string[];
  returns: number[];
  volatility?: number[];
  ohlc?: Array<{ o: number; h: number; l: number; c: number; x: number }>;
}

/**
 * Represents the data for combined signal charts.
 */
export interface CombinedSignals {
  trend_avg: number[];
  mom_avg: number[];
  ewmac_avg: number[];
  breakout_avg: number[];
  composite: number[];
}

/**
 * Represents the main data structure for trend analysis signals.
 */
export interface TrendData {
  dates: string[];
  signals: {
    trend: Record<number, number[]>;
    momentum: Record<number, number[]>;
    ewmac: Record<number, number[]>;
    breakout: Record<number, number[]>;
    combined: CombinedSignals;
  };
}


// --- Z-Score & Leaders/Laggards Types ---
export interface ZScoreOverviewRequest {
  baseCoin: string;
  compareCoin?: string;
  timeframe: string;
  period: string;
  exchange: string;
}

export interface ZScoreOverviewResponse {
  zscoreTimeSeries: ZScoreDataPoint[];
  currentZScore: number;
  zscoreDistribution: {
    buckets: number[];
    counts: number[];
  };
}

export interface ZScoreDataPoint {
  timestamp: number;
  price: number;
  zscore: number;
  volume: number;
  returns1h: number;
  returns1d: number;
  logReturns1h: number;
  rollingVolume: number;
}

export interface LeadersLaggardsRequest {
  exchange: string;
  period: string;
  topN: number;
}

export interface LeadersLaggardsResponse {
  leaders: MarketLeader[];
  laggards: MarketLeader[];
  volumeSpikes: VolumeSpike[];
  decorrelated: DecorrelatedAsset[];
  leadLagMatrix: LeadLagMatrix;
}

export interface MarketLeader {
  symbol: string;
  zscore: number;
  returns: number;
  volume: number;
  rank: number;
}

export interface VolumeSpike {
  symbol: string;
  volumeZScore: number;
  priceChange: number;
}

export interface DecorrelatedAsset {
  symbol: string;
  correlationWithMarket: number;
  avgCorrelation: number;
}

export interface LeadLagMatrix {
  coins: string[];
  lags: number[];
  matrix: number[][][];
}




export interface ArbOpportunity {
  longExchange: string
  shortExchange: string
  longRate: number
  shortRate: number
  spread: number
  combinedOI: number
}

export interface ExchangeData {
  funding_rate: number
  open_interest: number
}

export interface TokenRow {
  token: string
  exchanges: Record<string, ExchangeData>
}

export type DisplayMode = 'rate' | 'annualized'
export type SpreadUnit = 'percentage' | 'bps'
export type SortDirection = 'asc' | 'desc'


export interface KlineDTO {
  ts: number
  open: number
  high: number
  low: number
  close: number
  volume: number
}

export interface RvPoint {
  ts: number
  ret: number
  vol?: number
}

