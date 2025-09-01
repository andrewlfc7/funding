// src/utils/chartSetup.ts
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  TimeScale,
  Tooltip,
  Legend,
} from 'chart.js'

import {
  CandlestickController,
  OhlcController,
  CandlestickElement,
  OhlcElement,
} from 'chartjs-chart-financial'

// Needed for time axis
import 'chartjs-adapter-date-fns'

// Register base Chart.js modules
ChartJS.register(
  CategoryScale,
  LinearScale,
  TimeScale,
  Tooltip,
  Legend
)

// Register financial charts
ChartJS.register(
  CandlestickController,
  OhlcController,
  CandlestickElement,
  OhlcElement
)

export { ChartJS }