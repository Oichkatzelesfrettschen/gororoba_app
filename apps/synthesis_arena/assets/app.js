"use strict";

const outputNode = document.getElementById("output");
const challengeSelect = document.getElementById("challenge");
const evaluateForm = document.getElementById("evaluate-form");
const benchmarkForm = document.getElementById("benchmark-form");

function showJson(data) {
  outputNode.textContent = JSON.stringify(data, null, 2);
}

function setMode(mode) {
  const label = document.getElementById("mode-label");
  const panels = Array.from(document.querySelectorAll(".learning-panel"));
  const buttons = Array.from(document.querySelectorAll(".mode-btn"));

  buttons.forEach((button) => {
    const active = button.dataset.mode === mode;
    button.classList.toggle("is-active", active);
  });

  panels.forEach((panel) => {
    const visible = panel.dataset.layer === mode;
    panel.classList.toggle("hidden", !visible);
  });

  if (label) {
    if (mode === "story") {
      label.textContent = "Story Mode";
    } else if (mode === "research") {
      label.textContent = "Research Mode";
    } else {
      label.textContent = "Explorer Mode";
    }
  }
}

function readEvaluationPayload() {
  return {
    challenge_id: challengeSelect.value,
    throughput: Number(document.getElementById("throughput").value),
    precision: Number(document.getElementById("precision").value),
    efficiency: Number(document.getElementById("efficiency").value),
    resilience: Number(document.getElementById("resilience").value),
    seed: Number(document.getElementById("seed").value)
  };
}

function drawChart(evaluation) {
  const canvas = document.getElementById("arena-chart");
  if (!canvas || !evaluation || !Array.isArray(evaluation.metrics)) {
    return;
  }

  const ctx = canvas.getContext("2d");
  const width = canvas.width;
  const height = canvas.height;

  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = "#091425";
  ctx.fillRect(0, 0, width, height);

  const metrics = evaluation.metrics;
  if (metrics.length === 0) {
    return;
  }

  const maxValue = Math.max(
    ...metrics.map((metric) => Math.max(metric.value, metric.gate)),
    1
  );
  const barWidth = Math.max(30, Math.floor((width - 70) / metrics.length) - 18);

  metrics.forEach((metric, index) => {
    const x = 35 + index * (barWidth + 18);
    const valueHeight = (metric.value / maxValue) * (height - 70);
    const gateHeight = (metric.gate / maxValue) * (height - 70);
    const barY = height - valueHeight - 28;
    const gateY = height - gateHeight - 28;

    ctx.fillStyle = metric.passed ? "#35cfab" : "#eb695e";
    ctx.fillRect(x, barY, barWidth, valueHeight);

    ctx.strokeStyle = "#f3b45e";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(x - 2, gateY);
    ctx.lineTo(x + barWidth + 2, gateY);
    ctx.stroke();

    ctx.fillStyle = "#a6bed6";
    ctx.font = "12px monospace";
    ctx.fillText(metric.metric, x, height - 10);
  });

  ctx.fillStyle = "#dce8f8";
  ctx.font = "13px monospace";
  ctx.fillText(`composite: ${evaluation.composite_score}`, 16, 18);
  ctx.fillText(`target: ${evaluation.target_score}`, 230, 18);
  ctx.fillText(`all gates: ${evaluation.passed_all_gates ? "yes" : "no"}`, 390, 18);
}

async function loadChallenges() {
  const response = await fetch("/api/challenges");
  const challenges = await response.json();

  challengeSelect.innerHTML = "";
  challenges.forEach((challenge) => {
    const option = document.createElement("option");
    option.value = challenge.id;
    option.textContent = `${challenge.name} (${challenge.id})`;
    challengeSelect.appendChild(option);
  });

  showJson({ challenges });
}

evaluateForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const payload = readEvaluationPayload();
  const response = await fetch("/api/evaluate", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload)
  });
  const result = await response.json();
  showJson(result);
  drawChart(result);
});

benchmarkForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const payload = {
    request: readEvaluationPayload(),
    iterations: Number(document.getElementById("iterations").value)
  };
  const response = await fetch("/api/benchmark", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload)
  });
  const result = await response.json();
  showJson(result);
  if (result && result.sample) {
    drawChart(result.sample);
  }
});

Array.from(document.querySelectorAll(".mode-btn")).forEach((button) => {
  button.addEventListener("click", () => setMode(button.dataset.mode || "explorer"));
});

setMode("explorer");
loadChallenges().catch((error) => {
  showJson({ error: String(error) });
});
