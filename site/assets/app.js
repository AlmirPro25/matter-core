let cachedRelease = null;
let triadSelectedSeries = "max";
const TRIAD_SERIES_STORAGE_KEY = "matter.triad.selectedSeries";
const TRIAD_REFRESH_HISTORY_STORAGE_KEY = "matter.triad.refreshHistory";
const TRIAD_SNAPSHOT_META_STORAGE_KEY = "matter.triad.snapshotMeta";
const TRIAD_DATA_MODE_STORAGE_KEY = "matter.triad.dataMode";
const TRIAD_ALLOWED_SERIES = new Set(["max", "core", "world", "frontier"]);
const TRIAD_REFRESH_INTERVAL_MS = 60000;
const TRIAD_REFRESH_MAX_BACKOFF_MS = 5 * 60000;
let triadRefreshInFlight = false;
let triadRefreshTimerId = null;
let triadRefreshFailureCount = 0;
let triadNextRefreshAtMs = 0;
let triadCountdownTimerId = null;
const triadRefreshHistory = [];
let triadHistoryActionStatusTimerId = null;
let triadDataMode = "live";
let triadSnapshotMeta = null;

function readStoredTriadSeries() {
  try {
    const value = window.localStorage.getItem(TRIAD_SERIES_STORAGE_KEY);
    if (value && TRIAD_ALLOWED_SERIES.has(value)) {
      return value;
    }
  } catch (_) {
    // Ignore storage failures.
  }
  return "max";
}

function persistTriadSeries(value) {
  try {
    window.localStorage.setItem(TRIAD_SERIES_STORAGE_KEY, value);
  } catch (_) {
    // Ignore storage failures.
  }
}

function loadStoredRefreshHistory() {
  try {
    const raw = window.localStorage.getItem(TRIAD_REFRESH_HISTORY_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((item) => item && typeof item.time === "string" && typeof item.source === "string")
      .map((item) => ({ time: item.time, source: item.source, ok: !!item.ok }))
      .slice(0, 5);
  } catch (_) {
    return [];
  }
}

function persistRefreshHistory() {
  try {
    window.localStorage.setItem(TRIAD_REFRESH_HISTORY_STORAGE_KEY, JSON.stringify(triadRefreshHistory));
  } catch (_) {
    // Ignore storage failures.
  }
}

function clearRefreshHistory() {
  triadRefreshHistory.length = 0;
  try {
    window.localStorage.removeItem(TRIAD_REFRESH_HISTORY_STORAGE_KEY);
  } catch (_) {
    // Ignore storage failures.
  }
  renderRuntimeRefreshHistory();
}

function setRefreshHistoryActionStatus(text, level = "ok", autoReset = true) {
  const node = document.getElementById("runtime-refresh-history-action-status");
  if (!node) return;
  node.classList.remove("is-ok", "is-error");
  node.classList.add(level === "error" ? "is-error" : "is-ok");
  node.textContent = text;
  if (triadHistoryActionStatusTimerId !== null) {
    window.clearTimeout(triadHistoryActionStatusTimerId);
    triadHistoryActionStatusTimerId = null;
  }
  if (autoReset) {
    triadHistoryActionStatusTimerId = window.setTimeout(() => {
      const currentNode = document.getElementById("runtime-refresh-history-action-status");
      if (!currentNode) return;
      currentNode.classList.remove("is-ok", "is-error");
      currentNode.textContent = "pronto";
      triadHistoryActionStatusTimerId = null;
    }, 3000);
  }
}

async function loadRelease() {
  const response = await fetch("release.json", { cache: "no-store" });
  if (!response.ok) {
    return;
  }

  const release = await response.json();
  cachedRelease = release;
  const artifacts = release.artifacts || [];
  const artifact =
    artifacts.find((item) => item.name === "matter-core-beta-setup.exe") ||
    artifacts.find((item) => item.name === "matter-core-windows-x64.zip") ||
    artifacts[0];
  if (!artifact) {
    return;
  }

  const sizeMb = artifact.size_bytes / 1024 / 1024;
  document.getElementById("release-channel").textContent = release.channel || "beta";
  document.getElementById("release-version").textContent = release.version || "beta";
  document.getElementById("artifact-name").textContent = artifact.name;
  document.getElementById("artifact-size").textContent = `${sizeMb.toFixed(2)} MB`;
  document.getElementById("artifact-sha").textContent = artifact.sha256;
  document.getElementById("artifact-date").textContent = release.generated_at;

  const signature = document.getElementById("artifact-signature");
  if (signature) {
    signature.textContent = artifact.signed ? "assinado" : "nao assinado";
  }
}

function setHealthStatus(status) {
  const node = document.getElementById("triad-health-status");
  if (!node) return;

  node.classList.remove("is-pass", "is-warn", "is-fail");
  if (status === "pass") node.classList.add("is-pass");
  if (status === "warn") node.classList.add("is-warn");
  if (status === "fail") node.classList.add("is-fail");
  node.textContent = status || "desconhecido";
}

function setHealthSource(text) {
  const node = document.getElementById("triad-health-source");
  if (!node) return;
  node.textContent = text;
}

function setTrendField(id, value) {
  const node = document.getElementById(id);
  if (!node) return;
  node.textContent = value;
}

function setRuntimeRefreshStatus(ok) {
  const node = document.getElementById("runtime-last-refresh");
  if (!node) return;
  const now = new Date();
  const time = now.toLocaleTimeString("pt-BR", { hour12: false });
  node.classList.remove("is-refresh-ok", "is-refresh-error");
  node.classList.add(ok ? "is-refresh-ok" : "is-refresh-error");
  node.textContent = `${time} (${ok ? "ok" : "erro"})`;
}

function formatRefreshInterval(ms) {
  if (ms % 60000 === 0) {
    const minutes = ms / 60000;
    return minutes >= 2 ? `${minutes}m` : "60s";
  }
  return `${Math.round(ms / 1000)}s`;
}

function setRuntimeRefreshInterval(ms) {
  const node = document.getElementById("runtime-refresh-interval");
  if (!node) return;
  node.textContent = formatRefreshInterval(ms);
}

function setRuntimeRefreshCountdown(text) {
  const node = document.getElementById("runtime-refresh-countdown");
  if (!node) return;
  node.textContent = text;
}

function setRuntimeRefreshFailures(count) {
  const node = document.getElementById("runtime-refresh-failures");
  if (!node) return;
  node.textContent = String(Math.max(0, Number(count) || 0));
}

function setRuntimeRefreshSource(source) {
  const node = document.getElementById("runtime-refresh-source");
  if (!node) return;
  node.textContent = source || "unknown";
}

function setRuntimeDataMode(mode) {
  triadDataMode = mode === "snapshot" ? "snapshot" : "live";
  persistDataMode(triadDataMode);
  const node = document.getElementById("runtime-data-mode");
  if (!node) return;
  node.classList.remove("is-pass", "is-warn");
  if (triadDataMode === "snapshot") {
    node.classList.add("is-warn");
  } else {
    node.classList.add("is-pass");
  }
  node.textContent = triadDataMode;
  updateRuntimeControlsState();
}

function updateRuntimeControlsState() {
  const refreshNow = document.getElementById("runtime-refresh-now");
  const resumeLive = document.getElementById("runtime-resume-live");
  if (!refreshNow) return;
  const snapshotMode = triadDataMode === "snapshot";
  refreshNow.disabled = snapshotMode || triadRefreshInFlight;
  if (snapshotMode) {
    refreshNow.title = "Desabilitado em modo snapshot. Use Retomar live.";
  } else {
    refreshNow.title = "";
  }
  if (resumeLive) {
    resumeLive.disabled = !snapshotMode || triadRefreshInFlight;
    resumeLive.title = snapshotMode ? "Retomar atualizacao live." : "Ja esta em modo live.";
  }
}

function persistDataMode(mode) {
  try {
    window.localStorage.setItem(TRIAD_DATA_MODE_STORAGE_KEY, mode === "snapshot" ? "snapshot" : "live");
  } catch (_) {
    // Ignore storage failures.
  }
}

function loadDataMode() {
  try {
    const value = window.localStorage.getItem(TRIAD_DATA_MODE_STORAGE_KEY);
    return value === "snapshot" ? "snapshot" : "live";
  } catch (_) {
    return "live";
  }
}

function setRuntimeSnapshotInfo(meta) {
  const node = document.getElementById("runtime-snapshot-info");
  if (!node) return;
  if (!meta || !meta.importedAt) {
    node.textContent = "n/a";
    return;
  }
  const local = new Date(meta.importedAt).toLocaleString("pt-BR", { hour12: false });
  const src = meta.fileName || "snapshot.json";
  node.textContent = `${local} | ${src}`;
}

function setRuntimeSnapshotHash(text) {
  const node = document.getElementById("runtime-snapshot-hash");
  if (!node) return;
  node.textContent = text || "n/a";
}

function persistSnapshotMeta(meta) {
  try {
    if (!meta) {
      window.localStorage.removeItem(TRIAD_SNAPSHOT_META_STORAGE_KEY);
      return;
    }
    window.localStorage.setItem(TRIAD_SNAPSHOT_META_STORAGE_KEY, JSON.stringify(meta));
  } catch (_) {
    // Ignore storage failures.
  }
}

function loadSnapshotMeta() {
  try {
    const raw = window.localStorage.getItem(TRIAD_SNAPSHOT_META_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed.importedAt !== "string") return null;
    return {
      importedAt: parsed.importedAt,
      fileName: typeof parsed.fileName === "string" ? parsed.fileName : "",
      hash: typeof parsed.hash === "string" ? parsed.hash : ""
    };
  } catch (_) {
    return null;
  }
}

async function computeTextSha256Short(text) {
  const data = new TextEncoder().encode(text);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  return hashHex.slice(0, 16);
}

function renderRuntimeRefreshHistory() {
  const node = document.getElementById("runtime-refresh-history");
  if (!node) return;
  if (!triadRefreshHistory.length) {
    node.innerHTML = "<li>sem dados</li>";
    return;
  }
  const html = triadRefreshHistory
    .map((item) => `<li>${item.time} | ${item.source} | ${item.ok ? "ok" : "erro"}</li>`)
    .join("");
  node.innerHTML = html;
}

function pushRuntimeRefreshHistory(source, ok) {
  const now = new Date();
  triadRefreshHistory.unshift({
    time: now.toLocaleTimeString("pt-BR", { hour12: false }),
    source: source || "unknown",
    ok: !!ok
  });
  if (triadRefreshHistory.length > 5) {
    triadRefreshHistory.length = 5;
  }
  persistRefreshHistory();
  renderRuntimeRefreshHistory();
}

function setRefreshNowBusy(isBusy) {
  const button = document.getElementById("runtime-refresh-now");
  if (!button) return;
  button.disabled = isBusy;
  button.textContent = isBusy ? "Atualizando..." : "Atualizar agora";
}

function formatCountdown(ms) {
  const totalSeconds = Math.max(0, Math.ceil(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
  }
  return `${seconds}s`;
}

function stopCountdownLoop() {
  if (triadCountdownTimerId === null) return;
  window.clearInterval(triadCountdownTimerId);
  triadCountdownTimerId = null;
}

function startCountdownLoop() {
  stopCountdownLoop();
  triadCountdownTimerId = window.setInterval(() => {
    if (!triadNextRefreshAtMs || document.visibilityState !== "visible") {
      return;
    }
    const remainingMs = triadNextRefreshAtMs - Date.now();
    setRuntimeRefreshCountdown(formatCountdown(remainingMs));
  }, 250);
}

function setTrendClass(status) {
  const node = document.getElementById("triad-trend-class");
  if (!node) return;
  node.classList.remove("is-pass", "is-warn", "is-fail");
  if (status === "stable") {
    node.classList.add("is-pass");
  } else if (status === "warning") {
    node.classList.add("is-warn");
  } else if (status === "critical") {
    node.classList.add("is-fail");
  }
  node.textContent = status;
}

function computeTrendClass(maxP95, warnP95, failP95) {
  if (maxP95 >= failP95) return "critical";
  if (maxP95 >= warnP95) return "warning";
  return "stable";
}

function classifyLatency(value, warn, fail) {
  if (value >= fail) return "fail";
  if (value >= warn) return "warn";
  return "pass";
}

function buildSparklinePath(values, width, height, minY, maxY) {
  if (!values || values.length < 2) return "";
  const span = Math.max(1, maxY - minY);
  const stepX = width / (values.length - 1);
  let path = "";
  for (let i = 0; i < values.length; i += 1) {
    const x = i * stepX;
    const y = height - ((values[i] - minY) / span) * height;
    path += `${i === 0 ? "M" : " L"}${x.toFixed(2)} ${y.toFixed(2)}`;
  }
  return path;
}

function extractJsonObjects(raw) {
  const objects = [];
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;
  for (let i = 0; i < raw.length; i += 1) {
    const ch = raw[i];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === "\"") {
        inString = false;
      }
      continue;
    }
    if (ch === "\"") {
      inString = true;
      continue;
    }
    if (ch === "{") {
      if (depth === 0) {
        start = i;
      }
      depth += 1;
      continue;
    }
    if (ch === "}") {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        objects.push(raw.slice(start, i + 1));
        start = -1;
      }
    }
  }
  return objects;
}

function normalizeTriadSample(sample) {
  const observed =
    sample &&
    sample.latency_budget &&
    sample.latency_budget.observed_ms;
  return {
    core: Number((sample && sample.core_ms) ?? (observed && observed.core)),
    world: Number((sample && sample.world_ms) ?? (observed && observed.world)),
    frontier: Number((sample && sample.frontier_ms) ?? (observed && observed.frontier)),
    ts: String((sample && sample.generated_at) || "")
  };
}

function renderTriadSparkline(samples) {
  if (!samples || samples.length < 2) return;

  const points = samples
    .map((sample) => normalizeTriadSample(sample))
    .filter(
      (sample) =>
        Number.isFinite(sample.core) && Number.isFinite(sample.world) && Number.isFinite(sample.frontier)
    );
  if (points.length < 2) return;

  const coreValues = points.map((p) => p.core);
  const worldValues = points.map((p) => p.world);
  const frontierValues = points.map((p) => p.frontier);
  const allValues = coreValues.concat(worldValues, frontierValues);
  const minY = Math.min(...allValues);
  const maxY = Math.max(...allValues);
  const width = 156;
  const height = 32;
  const healthThresholds = cachedRelease && cachedRelease.runtime_health_thresholds;
  const healthSummary = cachedRelease && cachedRelease.runtime_health_summary;
  const warn =
    Number((healthThresholds && healthThresholds.warn_p95_ms) || 0) ||
    Number((healthSummary && healthSummary.max_p95_ms) || 0);
  const fail =
    Number((healthThresholds && healthThresholds.fail_p95_ms) || 0) ||
    (warn > 0 ? warn * 1.5 : 0);

  const corePath = document.getElementById("triad-trend-sparkline-core");
  const worldPath = document.getElementById("triad-trend-sparkline-world");
  const frontierPath = document.getElementById("triad-trend-sparkline-frontier");
  const severityGroup = document.getElementById("triad-trend-sparkline-severity");
  if (!corePath || !worldPath || !frontierPath || !severityGroup) return;

  corePath.setAttribute("d", buildSparklinePath(coreValues, width, height, minY, maxY));
  worldPath.setAttribute("d", buildSparklinePath(worldValues, width, height, minY, maxY));
  frontierPath.setAttribute("d", buildSparklinePath(frontierValues, width, height, minY, maxY));
  corePath.style.opacity = triadSelectedSeries === "core" || triadSelectedSeries === "max" ? "1" : "0.18";
  worldPath.style.opacity = triadSelectedSeries === "world" || triadSelectedSeries === "max" ? "1" : "0.18";
  frontierPath.style.opacity = triadSelectedSeries === "frontier" || triadSelectedSeries === "max" ? "1" : "0.18";
  severityGroup.innerHTML = "";
  const stepX = width / (points.length - 1);
  const span = Math.max(1, maxY - minY);
  for (let i = 0; i < points.length; i += 1) {
    const selectedValue =
      triadSelectedSeries === "core"
        ? points[i].core
        : triadSelectedSeries === "world"
          ? points[i].world
          : triadSelectedSeries === "frontier"
            ? points[i].frontier
            : Math.max(points[i].core, points[i].world, points[i].frontier);
    const x = i * stepX;
    const y = height - ((selectedValue - minY) / span) * height;
    const level = classifyLatency(selectedValue, warn, fail);
    const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    circle.setAttribute("cx", x.toFixed(2));
    circle.setAttribute("cy", y.toFixed(2));
    circle.setAttribute("r", "1.8");
    circle.setAttribute("class", `triad-sparkline-severity-point is-${level}`);
    const title = document.createElementNS("http://www.w3.org/2000/svg", "title");
    title.textContent = `${points[i].ts || "sample"} | core ${points[i].core.toFixed(1)} ms | world ${points[i].world.toFixed(1)} ms | frontier ${points[i].frontier.toFixed(1)} ms`;
    circle.appendChild(title);
    severityGroup.appendChild(circle);
  }
}

function setupTriadSeriesToggle() {
  const buttons = Array.from(document.querySelectorAll("[data-triad-series]"));
  if (!buttons.length) return;
  triadSelectedSeries = readStoredTriadSeries();
  buttons.forEach((b) => {
    const series = b.getAttribute("data-triad-series") || "";
    b.classList.toggle("is-active", series === triadSelectedSeries);
  });
  buttons.forEach((button) => {
    button.addEventListener("click", () => {
      triadSelectedSeries = button.getAttribute("data-triad-series") || "max";
      if (!TRIAD_ALLOWED_SERIES.has(triadSelectedSeries)) {
        triadSelectedSeries = "max";
      }
      buttons.forEach((b) => b.classList.remove("is-active"));
      button.classList.add("is-active");
      persistTriadSeries(triadSelectedSeries);
      loadTriadHistorySparkline().catch(() => {});
    });
  });
}

function setTrendSummary(summary, sourceLabel) {
  if (!summary) return;
  setTrendField("triad-trend-core-p95", `${summary.core_p95_ms} ms`);
  setTrendField("triad-trend-world-p95", `${summary.world_p95_ms} ms`);
  setTrendField("triad-trend-frontier-p95", `${summary.frontier_p95_ms} ms`);
  setTrendField("triad-trend-source", sourceLabel);
  if (
    Number.isFinite(summary.core_latest_ms) &&
    Number.isFinite(summary.core_median_ms) &&
    Number.isFinite(summary.world_latest_ms) &&
    Number.isFinite(summary.world_median_ms) &&
    Number.isFinite(summary.frontier_latest_ms) &&
    Number.isFinite(summary.frontier_median_ms)
  ) {
    const coreDelta = Math.round(summary.core_latest_ms - summary.core_median_ms);
    const worldDelta = Math.round(summary.world_latest_ms - summary.world_median_ms);
    const frontierDelta = Math.round(summary.frontier_latest_ms - summary.frontier_median_ms);
    setTrendField(
      "triad-trend-delta",
      `core ${coreDelta >= 0 ? "+" : ""}${coreDelta} ms | world ${worldDelta >= 0 ? "+" : ""}${worldDelta} ms | frontier ${frontierDelta >= 0 ? "+" : ""}${frontierDelta} ms`
    );
  } else {
    setTrendField("triad-trend-delta", "n/a");
  }

  const healthSummary = cachedRelease && cachedRelease.runtime_health_summary;
  const healthThresholds = cachedRelease && cachedRelease.runtime_health_thresholds;
  if (healthSummary) {
    const maxP95 = Math.max(summary.core_p95_ms, summary.world_p95_ms, summary.frontier_p95_ms);
    const warn =
      Number((healthThresholds && healthThresholds.warn_p95_ms) || 0) ||
      Number(healthSummary.max_p95_ms) ||
      0;
    const fail =
      Number((healthThresholds && healthThresholds.fail_p95_ms) || 0) ||
      (warn > 0 ? warn * 1.5 : 0);
    setTrendField("triad-threshold-warn", `${warn} ms`);
    setTrendField("triad-threshold-fail", `${fail} ms`);
    const trendClass = computeTrendClass(
      maxP95,
      warn,
      fail
    );
    setTrendClass(trendClass);
  }
}

async function loadTriadHealth() {
  const response = await fetch("downloads/status-triad-health.json", { cache: "no-store" });
  if (response.ok) {
    const health = await response.json();
    setHealthStatus(health.status);

    const maxP95 = document.getElementById("triad-health-max-p95");
    if (maxP95 && health.summary) {
      maxP95.textContent = `${health.summary.max_p95_ms} ms`;
    }

    const samples = document.getElementById("triad-health-samples");
    if (samples && health.summary) {
      samples.textContent = `${health.summary.window_samples}/${health.summary.total_samples}`;
    }
    setHealthSource("downloads/status-triad-health.json");
    return;
  }

  const fallback = cachedRelease && cachedRelease.runtime_health_summary;
  if (fallback) {
    setHealthStatus(fallback.status);
    const maxP95 = document.getElementById("triad-health-max-p95");
    if (maxP95) {
      maxP95.textContent = `${fallback.max_p95_ms} ms`;
    }
    const samples = document.getElementById("triad-health-samples");
    if (samples) {
      samples.textContent = `${fallback.window_samples}/${fallback.total_samples}`;
    }
    setHealthSource("release.json runtime_health_summary");
  }
}

async function loadTriadTrend() {
  const response = await fetch("downloads/status-triad-trend-report.json", { cache: "no-store" });
  if (response.ok) {
    const trend = await response.json();
    const triad = trend && trend.triad;
    if (triad && triad.core && triad.world && triad.frontier) {
      setTrendSummary(
        {
          core_p95_ms: Number(triad.core.p95_ms),
          world_p95_ms: Number(triad.world.p95_ms),
          frontier_p95_ms: Number(triad.frontier.p95_ms),
          core_latest_ms: Number(triad.core.latest_ms),
          core_median_ms: Number(triad.core.median_ms),
          world_latest_ms: Number(triad.world.latest_ms),
          world_median_ms: Number(triad.world.median_ms),
          frontier_latest_ms: Number(triad.frontier.latest_ms),
          frontier_median_ms: Number(triad.frontier.median_ms)
        },
        "downloads/status-triad-trend-report.json"
      );
      return;
    }
  }

  const fallback = cachedRelease && cachedRelease.runtime_trend_summary;
  if (fallback) {
    setTrendSummary(
      {
        core_p95_ms: Number(fallback.core_p95_ms),
        world_p95_ms: Number(fallback.world_p95_ms),
        frontier_p95_ms: Number(fallback.frontier_p95_ms)
      },
      "release.json runtime_trend_summary"
    );
  }
}

async function loadTriadHistorySparkline() {
  const response = await fetch("downloads/status-triad-history.ndjson", { cache: "no-store" });
  if (!response.ok) {
    return;
  }

  const raw = await response.text();
  const chunks = extractJsonObjects(raw);
  const parsed = [];
  for (let i = 0; i < chunks.length; i += 1) {
    try {
      parsed.push(JSON.parse(chunks[i]));
    } catch (_) {
      // Ignore malformed JSON chunks and render with valid samples only.
    }
  }
  if (parsed.length < 2) {
    return;
  }

  const recent = parsed.slice(-12);
  renderTriadSparkline(recent);
}

async function refreshRuntimeCard(source = "auto") {
  if (triadDataMode === "snapshot" && source !== "manual" && source !== "resume-live") {
    return;
  }
  if (triadRefreshInFlight) {
    return;
  }
  triadRefreshInFlight = true;
  setRefreshNowBusy(true);
  updateRuntimeControlsState();
  setRuntimeRefreshSource(source);
  try {
    await loadRelease();
    await Promise.all([loadTriadHealth(), loadTriadTrend(), loadTriadHistorySparkline()]);
    setRuntimeRefreshStatus(true);
    pushRuntimeRefreshHistory(source, true);
    triadRefreshFailureCount = 0;
    setRuntimeRefreshFailures(triadRefreshFailureCount);
    startRuntimeRefreshLoop();
  } catch (_) {
    // Keep page stable on transient refresh failures.
    setRuntimeRefreshStatus(false);
    pushRuntimeRefreshHistory(source, false);
    triadRefreshFailureCount += 1;
    setRuntimeRefreshFailures(triadRefreshFailureCount);
    startRuntimeRefreshLoop();
  } finally {
    triadRefreshInFlight = false;
    setRefreshNowBusy(false);
    updateRuntimeControlsState();
  }
}

function currentRefreshIntervalMs() {
  if (triadRefreshFailureCount <= 0) {
    return TRIAD_REFRESH_INTERVAL_MS;
  }
  const exp = Math.min(4, triadRefreshFailureCount);
  return Math.min(TRIAD_REFRESH_MAX_BACKOFF_MS, TRIAD_REFRESH_INTERVAL_MS * Math.pow(2, exp));
}

function startRuntimeRefreshLoop() {
  const intervalMs = currentRefreshIntervalMs();
  if (triadRefreshTimerId !== null) {
    const currentMs = Number(document.body.getAttribute("data-runtime-refresh-ms") || "0");
    if (currentMs === intervalMs) {
      return;
    }
    stopRuntimeRefreshLoop();
  }
  document.body.setAttribute("data-runtime-refresh-ms", String(intervalMs));
  setRuntimeRefreshInterval(intervalMs);
  triadNextRefreshAtMs = Date.now() + intervalMs;
  setRuntimeRefreshCountdown(formatCountdown(intervalMs));
  startCountdownLoop();
  triadRefreshTimerId = window.setInterval(() => {
    if (document.visibilityState !== "visible") {
      return;
    }
    triadNextRefreshAtMs = Date.now() + intervalMs;
    refreshRuntimeCard("auto").catch(() => {});
  }, intervalMs);
}

function stopRuntimeRefreshLoop() {
  if (triadRefreshTimerId === null) return;
  window.clearInterval(triadRefreshTimerId);
  triadRefreshTimerId = null;
  document.body.removeAttribute("data-runtime-refresh-ms");
  triadNextRefreshAtMs = 0;
  setRuntimeRefreshCountdown("pausado");
  stopCountdownLoop();
}

function setupRefreshNowButton() {
  const button = document.getElementById("runtime-refresh-now");
  if (!button) return;
  button.addEventListener("click", () => {
    refreshRuntimeCard("manual").catch(() => {});
  });
}

function setupClearRefreshHistoryButton() {
  const button = document.getElementById("runtime-refresh-history-clear");
  if (!button) return;
  button.addEventListener("click", () => {
    clearRefreshHistory();
    setRefreshHistoryActionStatus("historico limpo", "ok");
  });
}

function setupExportRefreshHistoryButton() {
  const button = document.getElementById("runtime-refresh-history-export");
  if (!button) return;
  button.addEventListener("click", () => {
    const payload = {
      exported_at: new Date().toISOString(),
      items: triadRefreshHistory
    };
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    const stamp = new Date().toISOString().replace(/[:.]/g, "-");
    a.href = url;
    a.download = `runtime-refresh-history-${stamp}.json`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
    setRefreshHistoryActionStatus("json exportado", "ok");
  });
}

function normalizeRefreshHistoryItems(items) {
  if (!Array.isArray(items)) return [];
  return items
    .filter((item) => item && typeof item.time === "string" && typeof item.source === "string")
    .map((item) => ({ time: item.time, source: item.source, ok: !!item.ok }))
    .slice(0, 5);
}

function setupImportRefreshHistoryButton() {
  const button = document.getElementById("runtime-refresh-history-import");
  const fileInput = document.getElementById("runtime-refresh-history-file");
  if (!button || !fileInput) return;

  button.addEventListener("click", () => {
    fileInput.value = "";
    fileInput.click();
  });

  fileInput.addEventListener("change", async () => {
    const file = fileInput.files && fileInput.files[0];
    if (!file) return;
    try {
      const raw = await file.text();
      const parsed = JSON.parse(raw);
      const items = normalizeRefreshHistoryItems(parsed && parsed.items);
      triadRefreshHistory.length = 0;
      triadRefreshHistory.push(...items);
      persistRefreshHistory();
      renderRuntimeRefreshHistory();
      setRefreshHistoryActionStatus("json importado", "ok");
    } catch (_) {
      setRefreshHistoryActionStatus("erro ao importar", "error");
    }
  });
}

function readNodeText(id) {
  const node = document.getElementById(id);
  if (!node) return "";
  return (node.textContent || "").trim();
}

function buildRuntimeStatusSnapshot() {
  return {
    format_version: "matter-runtime-status-v1",
    exported_at: new Date().toISOString(),
    data_mode: triadDataMode,
    release: {
      channel: readNodeText("release-channel"),
      version: readNodeText("release-version"),
      generated_at: readNodeText("artifact-date")
    },
    runtime: {
      health_status: readNodeText("triad-health-status"),
      health_max_p95: readNodeText("triad-health-max-p95"),
      trend_class: readNodeText("triad-trend-class"),
      trend_core_p95: readNodeText("triad-trend-core-p95"),
      trend_world_p95: readNodeText("triad-trend-world-p95"),
      trend_frontier_p95: readNodeText("triad-trend-frontier-p95"),
      refresh_last: readNodeText("runtime-last-refresh"),
      refresh_interval: readNodeText("runtime-refresh-interval"),
      refresh_next: readNodeText("runtime-refresh-countdown"),
      refresh_failures: readNodeText("runtime-refresh-failures"),
      refresh_source: readNodeText("runtime-refresh-source")
    },
    refresh_history: triadRefreshHistory.slice(0, 5),
    snapshot_meta: triadSnapshotMeta
      ? {
          imported_at: triadSnapshotMeta.importedAt || "",
          file_name: triadSnapshotMeta.fileName || "",
          hash: triadSnapshotMeta.hash || ""
        }
      : null
  };
}

async function copyTextToClipboard(text) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const ta = document.createElement("textarea");
  ta.value = text;
  ta.setAttribute("readonly", "");
  ta.style.position = "fixed";
  ta.style.opacity = "0";
  document.body.appendChild(ta);
  ta.select();
  document.execCommand("copy");
  ta.remove();
}

function setupCopyRuntimeStatusButton() {
  const button = document.getElementById("runtime-copy-status");
  if (!button) return;
  button.addEventListener("click", async () => {
    try {
      const payload = buildRuntimeStatusSnapshot();
      await copyTextToClipboard(JSON.stringify(payload, null, 2));
      setRefreshHistoryActionStatus("status copiado", "ok");
    } catch (_) {
      setRefreshHistoryActionStatus("erro ao copiar status", "error");
    }
  });
}

function setupExportRuntimeStatusButton() {
  const button = document.getElementById("runtime-export-status");
  if (!button) return;
  button.addEventListener("click", () => {
    try {
      const payload = buildRuntimeStatusSnapshot();
      const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      const stamp = new Date().toISOString().replace(/[:.]/g, "-");
      a.href = url;
      a.download = `runtime-status-${stamp}.json`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      setRefreshHistoryActionStatus("status exportado", "ok");
    } catch (_) {
      setRefreshHistoryActionStatus("erro ao exportar status", "error");
    }
  });
}

function applyRuntimeSnapshot(snapshot) {
  if (!snapshot || !snapshot.runtime) return false;
  const formatVersion = String(snapshot.format_version || "");
  if (formatVersion && formatVersion !== "matter-runtime-status-v1") {
    return false;
  }
  const runtime = snapshot.runtime;
  const release = snapshot.release || {};

  setTrendField("release-channel", String(release.channel || ""));
  setTrendField("release-version", String(release.version || ""));
  setTrendField("artifact-date", String(release.generated_at || ""));
  setTrendField("triad-health-status", String(runtime.health_status || ""));
  setTrendField("triad-health-max-p95", String(runtime.health_max_p95 || ""));
  setTrendField("triad-trend-class", String(runtime.trend_class || ""));
  setTrendField("triad-trend-core-p95", String(runtime.trend_core_p95 || ""));
  setTrendField("triad-trend-world-p95", String(runtime.trend_world_p95 || ""));
  setTrendField("triad-trend-frontier-p95", String(runtime.trend_frontier_p95 || ""));
  setTrendField("runtime-last-refresh", String(runtime.refresh_last || ""));
  setTrendField("runtime-refresh-interval", String(runtime.refresh_interval || ""));
  setTrendField("runtime-refresh-countdown", String(runtime.refresh_next || ""));
  setTrendField("runtime-refresh-failures", String(runtime.refresh_failures || ""));
  setTrendField("runtime-refresh-source", String(runtime.refresh_source || "import"));

  if (Array.isArray(snapshot.refresh_history)) {
    triadRefreshHistory.length = 0;
    triadRefreshHistory.push(...normalizeRefreshHistoryItems(snapshot.refresh_history));
    persistRefreshHistory();
    renderRuntimeRefreshHistory();
  }

  const importedMeta = snapshot.snapshot_meta;
  if (importedMeta && typeof importedMeta === "object") {
    triadSnapshotMeta = {
      importedAt: String(importedMeta.imported_at || new Date().toISOString()),
      fileName: String(importedMeta.file_name || ""),
      hash: String(importedMeta.hash || "")
    };
  } else {
    triadSnapshotMeta = null;
  }
  persistSnapshotMeta(triadSnapshotMeta);
  setRuntimeSnapshotInfo(triadSnapshotMeta);
  setRuntimeSnapshotHash(triadSnapshotMeta && triadSnapshotMeta.hash);

  if (snapshot.data_mode === "snapshot") {
    setRuntimeDataMode("snapshot");
  } else {
    setRuntimeDataMode("live");
  }
  return true;
}

function setupImportRuntimeStatusButton() {
  const button = document.getElementById("runtime-import-status");
  const fileInput = document.getElementById("runtime-import-status-file");
  if (!button || !fileInput) return;

  button.addEventListener("click", () => {
    fileInput.value = "";
    fileInput.click();
  });

  fileInput.addEventListener("change", async () => {
    const file = fileInput.files && fileInput.files[0];
    if (!file) return;
    try {
      const raw = await file.text();
      const parsed = JSON.parse(raw);
      const ok = applyRuntimeSnapshot(parsed);
      if (ok) {
        if (!triadSnapshotMeta) {
          const shortHash = await computeTextSha256Short(raw);
          triadSnapshotMeta = { importedAt: new Date().toISOString(), fileName: file.name || "", hash: shortHash };
          persistSnapshotMeta(triadSnapshotMeta);
          setRuntimeSnapshotInfo(triadSnapshotMeta);
          setRuntimeSnapshotHash(shortHash);
        }
        if (triadDataMode === "snapshot") {
          stopRuntimeRefreshLoop();
        } else if (document.visibilityState === "visible") {
          startRuntimeRefreshLoop();
        }
        setRefreshHistoryActionStatus("status importado", "ok");
      } else {
        setRefreshHistoryActionStatus("snapshot invalido", "error");
      }
    } catch (_) {
      setRefreshHistoryActionStatus("erro ao importar status", "error");
    }
  });
}

function setupResumeLiveButton() {
  const button = document.getElementById("runtime-resume-live");
  if (!button) return;
  button.addEventListener("click", () => {
    setRuntimeDataMode("live");
    triadSnapshotMeta = null;
    persistSnapshotMeta(null);
    setRuntimeSnapshotInfo(null);
    setRuntimeSnapshotHash(null);
    refreshRuntimeCard("resume-live").catch(() => {});
    if (document.visibilityState === "visible") {
      startRuntimeRefreshLoop();
    }
    setRefreshHistoryActionStatus("modo live retomado", "ok");
  });
}

function setupResetPanelButton() {
  const button = document.getElementById("runtime-reset-panel");
  if (!button) return;
  button.addEventListener("click", () => {
    const confirmed = window.confirm("Resetar painel local e voltar ao modo live?");
    if (!confirmed) {
      setRefreshHistoryActionStatus("reset cancelado", "ok");
      return;
    }
    clearRefreshHistory();
    triadSnapshotMeta = null;
    persistSnapshotMeta(null);
    setRuntimeSnapshotInfo(null);
    setRuntimeSnapshotHash(null);
    triadRefreshFailureCount = 0;
    setRuntimeRefreshFailures(0);
    setRuntimeDataMode("live");
    refreshRuntimeCard("manual").catch(() => {});
    if (document.visibilityState === "visible") {
      startRuntimeRefreshLoop();
    }
    setRefreshHistoryActionStatus("painel resetado", "ok");
  });
}

function setupRuntimeShortcuts() {
  document.addEventListener("keydown", (event) => {
    const target = event.target;
    const isEditable =
      target &&
      (target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT" ||
        target.isContentEditable);
    if (isEditable) {
      return;
    }
    const withMod = (event.ctrlKey || event.metaKey) && event.shiftKey;
    const key = event.key.toLowerCase();
    const isResumeCombo = withMod && key === "r";
    const isRefreshNowCombo = withMod && key === "u";
    const isExportStatusCombo = withMod && key === "e";
    const isCopyStatusCombo = withMod && key === "c";
    const isResetPanelCombo = withMod && key === "x";
    if (!isResumeCombo && !isRefreshNowCombo && !isExportStatusCombo && !isCopyStatusCombo && !isResetPanelCombo) {
      return;
    }
    event.preventDefault();
    if (isResumeCombo) {
      const resumeLive = document.getElementById("runtime-resume-live");
      if (resumeLive && !resumeLive.disabled) {
        resumeLive.click();
      }
      return;
    }
    const refreshNow = document.getElementById("runtime-refresh-now");
    if (isRefreshNowCombo) {
      if (refreshNow && !refreshNow.disabled) {
        refreshNow.click();
      }
      return;
    }
    const exportStatus = document.getElementById("runtime-export-status");
    if (isExportStatusCombo && exportStatus && !exportStatus.disabled) {
      exportStatus.click();
      return;
    }
    const copyStatus = document.getElementById("runtime-copy-status");
    if (isCopyStatusCombo && copyStatus && !copyStatus.disabled) {
      copyStatus.click();
      return;
    }
    const resetPanel = document.getElementById("runtime-reset-panel");
    if (isResetPanelCombo && resetPanel && !resetPanel.disabled) {
      resetPanel.click();
    }
  });
}

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    if (triadDataMode === "live") {
      refreshRuntimeCard("resume-visibility").catch(() => {});
      startRuntimeRefreshLoop();
    }
  } else {
    stopRuntimeRefreshLoop();
  }
});

triadRefreshHistory.push(...loadStoredRefreshHistory());
renderRuntimeRefreshHistory();
triadSnapshotMeta = loadSnapshotMeta();
setRuntimeSnapshotInfo(triadSnapshotMeta);
setRuntimeSnapshotHash(triadSnapshotMeta && triadSnapshotMeta.hash);
setRuntimeRefreshFailures(0);

setupTriadSeriesToggle();
setupRefreshNowButton();
setupClearRefreshHistoryButton();
setupExportRefreshHistoryButton();
setupImportRefreshHistoryButton();
setupCopyRuntimeStatusButton();
setupExportRuntimeStatusButton();
setupImportRuntimeStatusButton();
setupResumeLiveButton();
setupResetPanelButton();
setupRuntimeShortcuts();
setRuntimeDataMode(loadDataMode());
setRefreshHistoryActionStatus("pronto", "ok", false);

if (triadDataMode === "snapshot") {
  stopRuntimeRefreshLoop();
} else {
  refreshRuntimeCard("startup");
  if (document.visibilityState === "visible") {
    startRuntimeRefreshLoop();
  }
}
