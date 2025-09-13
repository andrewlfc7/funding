import type { MarketData, TrendData, LineDataset, CombinedSignals } from './types'
import { ChartConfiguration } from 'chart.js';

// Utility function to adjust color opacity
function adjustColorOpacity(color: string, opacity: number): string {
  if (color.startsWith('rgb(')) {
    return color.replace('rgb(', 'rgba(').replace(')', `, ${opacity})`);
  }
  if (color.startsWith('#')) {
    const r = parseInt(color.slice(1, 3), 16);
    const g = parseInt(color.slice(3, 5), 16);
    const b = parseInt(color.slice(5, 7), 16);
    return `rgba(${r}, ${g}, ${b}, ${opacity})`;
  }
  return color;
}

// CORRECTED: This function now correctly expects MarketData
export function createCandlestickConfig(data: MarketData, coin: string): ChartConfiguration {
  return {
    type: 'candlestick' as const,
    data: {
      datasets: [{
        label: `${coin} Price`,
        data: data.ohlc || []
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          display: false
        },
        tooltip: {
          callbacks: {
            label: function(context: any) {
              const point = context.raw;
              return [
                `Open: $${point.o.toLocaleString()}`,
                `High: $${point.h.toLocaleString()}`,
                `Low: $${point.l.toLocaleString()}`,
                `Close: $${point.c.toLocaleString()}`
              ];
            }
          }
        }
      },
      scales: {
        x: {
          type: 'time',
          time: {
            unit: 'day',
            displayFormats: {
              day: 'MMM dd'
            }
          },
          grid: {
            display: false
          },
          ticks: {
            font: {
              size: 10
            }
          }
        },
        y: {
          position: 'left',
          title: {
            display: true,
            text: 'Price ($)',
            font: {
              size: 11
            }
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            font: {
              size: 10
            }
          }
        }
      }
    }
  };
}

// UNCHANGED: This function correctly expects TrendData
export function createSignalsChartConfig(
  data: TrendData,
  activeSignals: Partial<Record<keyof CombinedSignals, boolean>>
): ChartConfiguration {
  const labels = data.dates ?? [];
  const datasets: any[] = [];

  const signalConfigs = {
    trend_avg:   { label: 'Trend',     color: 'rgb(255, 99, 132)' },
    mom_avg:     { label: 'Momentum',  color: 'rgb(54, 162, 235)' },
    ewmac_avg:   { label: 'EWMAC',     color: 'rgb(75, 192, 192)' },
    breakout_avg:{ label: 'Breakout',  color: 'rgb(153, 102, 255)' },
    composite:   { label: 'Composite', color: 'rgb(255, 159, 64)' }
  } as const satisfies Record<keyof CombinedSignals, { label: string; color: string }>;

  type SignalKey = keyof typeof signalConfigs;

  (Object.keys(signalConfigs) as SignalKey[]).forEach((key) => {
    const cfg = signalConfigs[key];
    const series = data.signals.combined[key];
    if (activeSignals[key] && Array.isArray(series)) {
      datasets.push({
        label: cfg.label,
        data: series,
        borderColor: cfg.color,
        backgroundColor: cfg.color.replace('rgb', 'rgba').replace(')', ', 0.1)'),
        borderWidth: 2,
        tension: 0.4,
        pointRadius: 0,
        pointHoverRadius: 6,
        fill: false,
      });
    }
  });

  return {
    type: 'line' as const,
    data: {
      labels,
      datasets
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: {
        mode: 'index' as const,
        intersect: false,
      },
      plugins: {
        legend: {
          display: true,
          position: 'top' as const,
          labels: {
            font: {
              size: 11
            },
            usePointStyle: true,
            pointStyle: 'line'
          }
        },
        tooltip: {
          mode: 'index' as const,
          intersect: false,
          callbacks: {
            label: function(context: any) {
              return `${context.dataset.label}: ${context.parsed.y.toFixed(4)}`;
            }
          }
        }
      },
      scales: {
        x: {
          display: true,
          grid: {
            display: false
          },
          ticks: {
            maxTicksLimit: 8,
            font: {
              size: 10
            }
          }
        },
        y: {
          display: true,
          position: 'left' as const,
          title: {
            display: true,
            text: 'Signal Value',
            font: {
              size: 11
            }
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            font: {
              size: 10
            }
          }
        }
      }
    }
  };
}

// UNCHANGED: This function correctly expects TrendData
export function createIndividualSignalConfig(
  data: TrendData,
  signalType: 'trend' | 'momentum' | 'ewmac' | 'breakout',
  lookbacks: number[],
  activeLookbacks: Record<number, boolean>,
  signalConfig: { title: string; color: string }
): ChartConfiguration {
  const labels = data.dates || [];
  const datasets: LineDataset[] = [];

  lookbacks.forEach((lb, index) => {
    const signalSeries = data.signals[signalType][lb];
    if (activeLookbacks[lb] && signalSeries) {
      const opacity = 0.3 + (0.7 * (index / lookbacks.length));
      const color = adjustColorOpacity(signalConfig.color, opacity);
      
      datasets.push({
        label: `${lb}d`,
        data: signalSeries,
        borderColor: color,
        backgroundColor: color.replace('rgb', 'rgba').replace(')', ', 0.1)'),
        borderWidth: 2,
        tension: 0.4,
        pointRadius: 0,
        pointHoverRadius: 6,
        fill: false
      });
    }
  });
  
  return {
    type: 'line' as const,
    data: {
      labels,
      datasets
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: {
        mode: 'index' as const,
        intersect: false,
      },
      plugins: {
        legend: {
          display: true,
          position: 'top' as const,
          labels: {
            font: {
              size: 11
            },
            usePointStyle: true,
            pointStyle: 'line'
          }
        },
        tooltip: {
          mode: 'index' as const,
          intersect: false,
          callbacks: {
            label: function(context: any) {
              return `${context.dataset.label}: ${context.parsed.y.toFixed(4)}`;
            }
          }
        }
      },
      scales: {
        x: {
          display: true,
          grid: {
            display: false
          },
          ticks: {
            maxTicksLimit: 8,
            font: {
              size: 10
            }
          }
        },
        y: {
          display: true,
          position: 'left' as const,
          title: {
            display: true,
            text: signalConfig.title,
            font: {
              size: 11
            }
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            font: {
              size: 10
            }
          }
        }
      }
    }
  };
}

// CORRECTED: This function now correctly expects MarketData
export function createReturnsChartConfig(data: MarketData, metric: 'returns' | 'volatility' | 'both'): ChartConfiguration {
  const labels = data.dates || [];
  const datasets = [];
  
  if ((metric === 'returns' || metric === 'both') && data.returns) {
    datasets.push({
      label: 'Returns (%)',
      data: data.returns.map((r: number) => r * 100), // Convert to percentage
      borderColor: 'rgb(75, 192, 192)',
      backgroundColor: 'rgba(75, 192, 192, 0.1)',
      borderWidth: 2,
      tension: 0.4,
      pointRadius: 0,
      pointHoverRadius: 6,
      fill: false,
      yAxisID: 'y'
    });
  }
  
  if ((metric === 'volatility' || metric === 'both') && data.volatility) {
    datasets.push({
      label: 'Volatility',
      data: data.volatility,
      borderColor: 'rgb(255, 99, 132)',
      backgroundColor: 'rgba(255, 99, 132, 0.1)',
      borderWidth: 2,
      tension: 0.4,
      pointRadius: 0,
      pointHoverRadius: 6,
      fill: false,
      yAxisID: metric === 'both' ? 'y1' : 'y'
    });
  }
  
  const scales: any = {
    x: {
      display: true,
      grid: {
        display: false
      },
      ticks: {
        maxTicksLimit: 8,
        font: {
          size: 10
        }
      }
    },
    y: {
      display: true,
      position: 'left' as const,
      title: {
        display: true,
        text: metric === 'volatility' ? 'Volatility' : 'Returns (%)',
        font: {
          size: 11
        }
      },
      grid: {
        color: 'rgba(255, 255, 255, 0.05)'
      },
      ticks: {
        font: {
          size: 10
        }
      }
    }
  };
  
  if (metric === 'both') {
    scales.y1 = {
      display: true,
      position: 'right' as const,
      title: {
        display: true,
        text: 'Volatility',
        font: {
          size: 11
        }
      },
      grid: {
        drawOnChartArea: false
      },
      ticks: {
        font: {
          size: 10
        }
      }
    };
  }
  
  return {
    type: 'line' as const,
    data: {
      labels,
      datasets
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: {
        mode: 'index' as const,
        intersect: false,
      },
      plugins: {
        legend: {
          display: true,
          position: 'top' as const,
          labels: {
            font: {
              size: 11
            },
            usePointStyle: true,
            pointStyle: 'line'
          }
        },
        tooltip: {
          mode: 'index' as const,
          intersect: false,
          callbacks: {
            label: function(context: any) {
              const value = context.parsed.y;
              return `${context.dataset.label}: ${value.toFixed(2)}${metric === 'returns' || (metric === 'both' && context.dataset.yAxisID === 'y') ? '%' : ''}`;
            }
          }
        }
      },
      scales
    }
  };
}
