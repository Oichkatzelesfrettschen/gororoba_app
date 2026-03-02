function parseHistoryRows() {
  const rows = Array.from(document.querySelectorAll('section.history tbody tr'));
  return rows.map((row) => {
    const cells = row.querySelectorAll('td');
    if (cells.length < 6) {
      return null;
    }
    return {
      runId: Number(cells[0].textContent || 0),
      pipeline: (cells[1].textContent || '').trim(),
      profile: (cells[2].textContent || '').trim(),
      metric: Number(cells[3].textContent || 0),
      gate: (cells[4].textContent || '').trim(),
      duration: Number(cells[5].textContent || 0),
    };
  }).filter(Boolean);
}

function drawHistoryChart(data) {
  const canvas = document.getElementById('history-chart');
  if (!canvas || data.length === 0) {
    return;
  }
  const ctx = canvas.getContext('2d');
  const width = canvas.width;
  const height = canvas.height;

  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = '#06101b';
  ctx.fillRect(0, 0, width, height);

  const maxMetric = Math.max(...data.map((item) => item.metric), 0.001);
  const maxDuration = Math.max(...data.map((item) => item.duration), 1);
  const barWidth = Math.max(8, Math.floor((width - 30) / data.length) - 4);

  data.forEach((item, idx) => {
    const x = 20 + idx * (barWidth + 4);
    const metricHeight = (item.metric / maxMetric) * (height * 0.5);
    const durationHeight = (item.duration / maxDuration) * (height * 0.38);

    ctx.fillStyle = '#2fd6c5';
    ctx.fillRect(x, height - metricHeight - 12, barWidth, metricHeight);

    ctx.fillStyle = '#f4ab4c';
    ctx.fillRect(x, height - durationHeight - 12, Math.max(2, Math.floor(barWidth * 0.35)), durationHeight);

    if (item.gate === 'fail') {
      ctx.fillStyle = '#ff5f5f';
      ctx.fillRect(x, 6, barWidth, 4);
    }
  });

  ctx.fillStyle = '#97b8d8';
  ctx.font = '12px monospace';
  ctx.fillText('teal bars: metric value | amber bars: duration', 16, 16);
}

function runToyOscillator() {
  const canvas = document.getElementById('toy-oscillator');
  const phaseCanvas = document.getElementById('phase-portrait');
  if (!canvas) {
    return;
  }
  const ctx = canvas.getContext('2d');
  const width = canvas.width;
  const height = canvas.height;
  const phaseCtx = phaseCanvas ? phaseCanvas.getContext('2d') : null;
  const phaseWidth = phaseCanvas ? phaseCanvas.width : 0;
  const phaseHeight = phaseCanvas ? phaseCanvas.height : 0;

  let t = 0;
  let phasePoints = [];

  function drawPhasePortrait(points) {
    if (!phaseCtx || points.length < 2) {
      return;
    }
    phaseCtx.clearRect(0, 0, phaseWidth, phaseHeight);
    phaseCtx.fillStyle = '#04111c';
    phaseCtx.fillRect(0, 0, phaseWidth, phaseHeight);

    const maxAbsX = Math.max(...points.map((point) => Math.abs(point.x)), 0.001);
    const maxAbsV = Math.max(...points.map((point) => Math.abs(point.v)), 0.001);
    const xToCanvas = (x) => phaseWidth * 0.5 + (x / maxAbsX) * (phaseWidth * 0.42);
    const yToCanvas = (v) => phaseHeight * 0.5 - (v / maxAbsV) * (phaseHeight * 0.4);

    phaseCtx.strokeStyle = '#29465f';
    phaseCtx.beginPath();
    phaseCtx.moveTo(0, phaseHeight * 0.5);
    phaseCtx.lineTo(phaseWidth, phaseHeight * 0.5);
    phaseCtx.moveTo(phaseWidth * 0.5, 0);
    phaseCtx.lineTo(phaseWidth * 0.5, phaseHeight);
    phaseCtx.stroke();

    phaseCtx.strokeStyle = '#2fd6c5';
    phaseCtx.lineWidth = 1.8;
    phaseCtx.beginPath();
    points.forEach((point, idx) => {
      const px = xToCanvas(point.x);
      const py = yToCanvas(point.v);
      if (idx === 0) {
        phaseCtx.moveTo(px, py);
      } else {
        phaseCtx.lineTo(px, py);
      }
    });
    phaseCtx.stroke();

    const last = points[points.length - 1];
    phaseCtx.fillStyle = '#f5b158';
    phaseCtx.beginPath();
    phaseCtx.arc(xToCanvas(last.x), yToCanvas(last.v), 4, 0, Math.PI * 2);
    phaseCtx.fill();

    phaseCtx.fillStyle = '#9db9d8';
    phaseCtx.font = '12px monospace';
    phaseCtx.fillText('phase portrait: x vs v (damped spiral indicates stability)', 12, 16);
  }

  const draw = () => {
    t += 0.04;
    const x = Math.sin(t) * Math.exp(-0.02 * t);
    const v = Math.cos(t) * Math.exp(-0.02 * t) - 0.02 * x;

    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = '#050d16';
    ctx.fillRect(0, 0, width, height);

    ctx.strokeStyle = '#35516f';
    ctx.beginPath();
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();

    const bobX = width * 0.5 + x * (width * 0.35);
    ctx.strokeStyle = '#f5b158';
    ctx.beginPath();
    ctx.moveTo(width * 0.5, 20);
    ctx.lineTo(bobX, height * 0.5);
    ctx.stroke();

    ctx.fillStyle = '#36d6c8';
    ctx.beginPath();
    ctx.arc(bobX, height * 0.5, 14, 0, Math.PI * 2);
    ctx.fill();

    phasePoints.push({ x, v });
    if (phasePoints.length > 320) {
      phasePoints = phasePoints.slice(phasePoints.length - 320);
    }
    drawPhasePortrait(phasePoints);

    requestAnimationFrame(draw);
  };

  draw();
}

document.addEventListener('DOMContentLoaded', () => {
  const data = parseHistoryRows();
  drawHistoryChart(data);
  runToyOscillator();
});
