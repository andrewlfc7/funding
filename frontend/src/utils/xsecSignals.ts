export interface XsecSignal {
  id: 'trend' | 'momentum' | 'ewmac' | 'breakout' | 'composite'
  name: string
  color: string
  yAxisLabel: string
}

export const xsecSignals: XsecSignal[] = [
  { id: 'trend',     name: 'Trend',     color: 'rgb(255, 99, 132)',  yAxisLabel: 'Z' },
  { id: 'momentum',  name: 'Mom',       color: 'rgb(54, 162, 235)',  yAxisLabel: 'Z' },
  { id: 'ewmac',     name: 'EWMA',      color: 'rgb(75, 192, 192)',  yAxisLabel: 'Z' },
  { id: 'breakout',  name: 'Breakout',  color: 'rgb(153, 102, 255)', yAxisLabel: 'Z' },
  { id: 'composite', name: 'Composite', color: 'rgb(255, 159, 64)',  yAxisLabel: 'Z' },
]
