import React from 'react';
import { Line } from 'react-chartjs-2';
import { Chart as ChartJS, CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend, TimeScale } from 'chart.js';
import 'chartjs-adapter-date-fns';

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend, TimeScale);

export default function TimeSeriesChart({ data, options }) {
  const max = data.datasets.map(x => x.data.reduce((acc, curr) => Math.max(acc, curr), 0)).reduce((acc, curr) => Math.max(acc, curr), 0);

  options = options ?? {
    responsive: true,
    plugins: {
      legend: {
        position: 'top',
        // Prevent toggle behavior
        onClick: () => {}
      },
      tooltip: {
        mode: 'index',
        intersect: false,
        callbacks: {
          label: function(tooltipItem) {
            return `${tooltipItem.dataset.label}: ${tooltipItem.raw} servers`;
          }
        }
      },
    },
    scales: {
      x: {
        type: 'time',
        time: {
          unit: 'hour',
                tooltipFormat: 'HH:mm'  // Formats tooltip to show hour and minutes

        }
      },
      y: {
        beginAtZero: true,
        // Bump the max above the max value
        suggestedMax: Math.ceil(max * 1.1),
      }
    },
    interaction: {
      mode: 'nearest',
      axis: 'x',
      intersect: false
    },
  };

  return <Line data={data} options={options} />;
}

