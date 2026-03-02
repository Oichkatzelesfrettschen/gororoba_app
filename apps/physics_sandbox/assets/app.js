function getFormObject(form) {
  const data = new FormData(form);
  const out = {};
  for (const [key, value] of data.entries()) {
    out[key] = Number(value);
  }
  return out;
}

function setMode(mode) {
  const label = document.getElementById('mode-label');
  const layers = Array.from(document.querySelectorAll('.learning-layer'));
  const buttons = Array.from(document.querySelectorAll('.mode-btn'));

  buttons.forEach((button) => {
    const active = button.dataset.mode === mode;
    button.classList.toggle('is-active', active);
  });

  layers.forEach((layer) => {
    const visible = layer.dataset.layer === mode;
    layer.classList.toggle('is-hidden', !visible);
  });

  if (label) {
    if (mode === 'story') {
      label.textContent = 'Story Mode';
    } else if (mode === 'research') {
      label.textContent = 'Research Mode';
    } else {
      label.textContent = 'Explorer Mode';
    }
  }
}

function drawSimulationPlot(samples) {
  const canvas = document.getElementById('plot');
  const ctx = canvas.getContext('2d');
  const width = canvas.width;
  const height = canvas.height;

  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = '#08131f';
  ctx.fillRect(0, 0, width, height);

  if (!samples || samples.length < 2) {
    return;
  }

  const maxStep = samples[samples.length - 1].step;
  const maxAbsX = Math.max(...samples.map((sample) => Math.abs(sample.x)), 0.001);
  const maxEnergy = Math.max(...samples.map((sample) => sample.energy), 0.001);

  const xToCanvas = (step) => (step / maxStep) * (width - 40) + 20;
  const yToCanvasX = (value) => height * 0.36 - (value / maxAbsX) * (height * 0.26);
  const yToCanvasEnergy = (value) => height - (value / maxEnergy) * (height * 0.42) - 18;

  ctx.strokeStyle = '#2a4663';
  ctx.beginPath();
  ctx.moveTo(0, height * 0.36);
  ctx.lineTo(width, height * 0.36);
  ctx.moveTo(0, height - 18);
  ctx.lineTo(width, height - 18);
  ctx.stroke();

  ctx.strokeStyle = '#2fd4ca';
  ctx.lineWidth = 2;
  ctx.beginPath();
  samples.forEach((sample, idx) => {
    const px = xToCanvas(sample.step);
    const py = yToCanvasX(sample.x);
    if (idx === 0) {
      ctx.moveTo(px, py);
    } else {
      ctx.lineTo(px, py);
    }
  });
  ctx.stroke();

  ctx.strokeStyle = '#f0a447';
  ctx.lineWidth = 1.8;
  ctx.beginPath();
  samples.forEach((sample, idx) => {
    const px = xToCanvas(sample.step);
    const py = yToCanvasEnergy(sample.energy);
    if (idx === 0) {
      ctx.moveTo(px, py);
    } else {
      ctx.lineTo(px, py);
    }
  });
  ctx.stroke();

  ctx.fillStyle = '#a0bbd3';
  ctx.font = '12px monospace';
  ctx.fillText('teal: displacement x(t)', 18, 16);
  ctx.fillText('amber: energy E(t)', 220, 16);
}

function drawPhasePlot(samples) {
  const canvas = document.getElementById('phase-plot');
  const ctx = canvas.getContext('2d');
  const width = canvas.width;
  const height = canvas.height;

  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = '#08131f';
  ctx.fillRect(0, 0, width, height);

  if (!samples || samples.length < 3) {
    return;
  }

  const dt = samples[1].step - samples[0].step || 1;
  const points = [];
  for (let i = 1; i < samples.length; i += 1) {
    const x = samples[i].x;
    const v = (samples[i].x - samples[i - 1].x) / dt;
    points.push({ x, v });
  }

  const maxAbsX = Math.max(...points.map((point) => Math.abs(point.x)), 0.001);
  const maxAbsV = Math.max(...points.map((point) => Math.abs(point.v)), 0.001);
  const xToCanvas = (x) => width * 0.5 + (x / maxAbsX) * (width * 0.42);
  const yToCanvas = (v) => height * 0.5 - (v / maxAbsV) * (height * 0.4);

  ctx.strokeStyle = '#2a4663';
  ctx.beginPath();
  ctx.moveTo(0, height * 0.5);
  ctx.lineTo(width, height * 0.5);
  ctx.moveTo(width * 0.5, 0);
  ctx.lineTo(width * 0.5, height);
  ctx.stroke();

  ctx.strokeStyle = '#4ad2f5';
  ctx.lineWidth = 1.8;
  ctx.beginPath();
  points.forEach((point, idx) => {
    const px = xToCanvas(point.x);
    const py = yToCanvas(point.v);
    if (idx === 0) {
      ctx.moveTo(px, py);
    } else {
      ctx.lineTo(px, py);
    }
  });
  ctx.stroke();

  const last = points[points.length - 1];
  ctx.fillStyle = '#f0a447';
  ctx.beginPath();
  ctx.arc(xToCanvas(last.x), yToCanvas(last.v), 4, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = '#a0bbd3';
  ctx.font = '12px monospace';
  ctx.fillText('phase portrait: x vs estimated v', 16, 16);
}

async function postJson(url, payload) {
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
    },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`Request failed (${response.status}): ${body}`);
  }
  return response.json();
}

document.addEventListener('DOMContentLoaded', () => {
  const simulateForm = document.getElementById('simulate-form');
  const benchmarkForm = document.getElementById('benchmark-form');
  const simulateOutput = document.getElementById('simulate-output');
  const benchmarkOutput = document.getElementById('benchmark-output');
  const plotNote = document.getElementById('plot-note');

  Array.from(document.querySelectorAll('.mode-btn')).forEach((button) => {
    button.addEventListener('click', () => setMode(button.dataset.mode || 'explorer'));
  });
  setMode('explorer');

  simulateForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const payload = getFormObject(simulateForm);
    simulateOutput.textContent = 'Running simulation...';
    try {
      const result = await postJson('/api/simulate', payload);
      simulateOutput.textContent = JSON.stringify(result, null, 2);
      drawSimulationPlot(result.samples || []);
      drawPhasePlot(result.samples || []);
      plotNote.textContent = `stable=${result.stable}, drift=${result.energy_drift.toFixed(6)}, max_abs_x=${result.max_abs_x.toFixed(4)}`;
    } catch (error) {
      simulateOutput.textContent = String(error);
    }
  });

  benchmarkForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const payload = getFormObject(benchmarkForm);
    benchmarkOutput.textContent = 'Running benchmark...';
    try {
      const result = await postJson('/api/benchmark', payload);
      benchmarkOutput.textContent = JSON.stringify(result, null, 2);
    } catch (error) {
      benchmarkOutput.textContent = String(error);
    }
  });
});
