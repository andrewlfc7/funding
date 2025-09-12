// frontend/src/api/zscore/marketMicroStructure.ts
import axios from 'axios';
import { API_BASE_URL, ZSCORE_MICROSTRUCTURE_ENDPOINT, joinUrl } from '@/utils/constants';

export interface MicrostructureRequest {
  timeframe?: string;           // "1h" ONLY
  topN?: number;               // default 50
  exchange: string;
  marketType?: string;         // default "spot"
  period?: string;             // e.g. "7d"
}

export interface MicrostructureResponse {
  volumeFlows: FlowRow[];
  rotationMatrix: RotationMatrix;
  liquidityConcentration: LiquidityConcentration;
}

export interface FlowRow {
  symbol: string;
  volumeIn: number;           // buy USD total
  volumeOut: number;          // sell USD total
  netFlow: number;            // buy - sell
  netFlowZScore: number;
}

export interface RotationMatrix {
  coins: string[];
  flows: number[][];         // i -> j
}

export interface LiquidityConcentration {
  groups: LiqGroup[];
}

export interface LiqGroup {
  range: string;             // "Top 5", "6-10", ...
  volumeShare: number;
  countShare: number;
}

/**
 * Fetch market microstructure flow data
 */
export const getMarketMicrostructureFlow = async (
  params: MicrostructureRequest
): Promise<MicrostructureResponse> => {
  try {
    // Set default values if not provided
    const requestParams: MicrostructureRequest = {
      timeframe: params.timeframe || '1h',
      topN: params.topN || 50,
      exchange: params.exchange,
      marketType: params.marketType || 'spot',
      period: params.period || '7d',
    };

    const endpoint = joinUrl(API_BASE_URL, ZSCORE_MICROSTRUCTURE_ENDPOINT);
    const response = await axios.get<MicrostructureResponse>(
      endpoint,
      { params: requestParams }
    );

    return response.data;
  } catch (error) {
    if (axios.isAxiosError(error)) {
      throw new Error(
        `Failed to fetch market microstructure data: ${error.response?.data?.message || error.message}`
      );
    }
    throw new Error('Failed to fetch market microstructure data: Unknown error');
  }
};

/**
 * Hook-style function for React components
 */
export const useMarketMicrostructure = () => {
  const fetchMicrostructure = async (params: MicrostructureRequest) => {
    return await getMarketMicrostructureFlow(params);
  };

  return {
    fetchMicrostructure,
  };
};

export default {
  getMarketMicrostructureFlow,
  useMarketMicrostructure,
};