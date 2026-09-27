// Cosmic Byte Spectrum Control Application Logic

const DPI_STEPS = [
  200, 400, 600, 800, 1000, 1200, 1400, 1600, 1800, 2000,
  2400, 3200, 4000, 4800, 5600, 6400, 7200, 8000, 8800, 9600,
  10400, 11200, 12800
];

const RGB_MODES = [
  { id: 1, name: "DPI Breathing" },
  { id: 2, name: "Cycle Breathing" },
  { id: 3, name: "Static Light" },
  { id: 4, name: "Flowing Water" },
  { id: 5, name: "Mono Water" },
  { id: 6, name: "Comet Streak" },
  { id: 7, name: "Neon" },
  { id: 8, name: "Ambilight" },
  { id: 9, name: "Flicker" },
  { id: 10, name: "Star Trek" },
  { id: 11, name: "Ripple" },
  { id: 12, name: "Enraptured" },
  { id: 13, name: "Button Response" },
  { id: 14, name: "LED Off" },
  { id: 15, name: "Single Breath" },
  { id: 16, name: "Cycle Color" }
];

const BUTTON_ACTIONS = [
  { code: 1, name: "Left Click" },
  { code: 2, name: "Middle Click" },
  { code: 3, name: "Right Click" },
  { code: 4, name: "Back" },
  { code: 5, name: "Forward" },
  { code: 6, name: "DPI Loop" },
  { code: 14, name: "Rapid Fire" },
  { code: 15, name: "LED Loop" },
  { code: 16, name: "DPI +" },
  { code: 17, name: "DPI -" },
  { code: 41, name: "Mode Loop" },
  { code: 0, name: "Disabled" }
];

const BUTTON_LABELS = {
  "Left": "Button 1 (Left Click)",
  "Middle": "Button 2 (Scroll Wheel)",
  "Right": "Button 3 (Right Click)",
  "Back": "Button 4 (Side Back)",
  "Forward": "Button 5 (Side Forward)",
  "DpiLoop": "Button 6 (Top DPI Switch)"
};

let currentSettings = null;
let currentInfo = null;

// Initialize app
document.addEventListener("DOMContentLoaded", async () => {
  setupRgbDropdown();
  setupEventListeners();
  await refreshDevice();
});

function setupRgbDropdown() {
  const select = document.getElementById("rgbModeSelect");
  select.innerHTML = "";
  RGB_MODES.forEach(m => {
    const opt = document.createElement("option");
    opt.value = m.name;
    opt.textContent = `${m.id}. ${m.name}`;
    select.appendChild(opt);
  });
}

async function refreshDevice() {
  try {
    const [infoRes, settingsRes, profilesRes] = await Promise.all([
      fetch("/api/info").then(r => r.json()),
      fetch("/api/settings").then(r => r.json()),
      fetch("/api/profiles").then(r => r.json())
    ]);

    currentInfo = infoRes;
    currentSettings = settingsRes;

    updateStatus(infoRes.connected, infoRes.hidraw_path);
    renderProfiles(profilesRes);
    renderSettings(settingsRes);
  } catch (err) {
    console.error("Failed to connect to Spectrum device:", err);
    updateStatus(false, null);
  }
}

function updateStatus(connected, path) {
  const pill = document.getElementById("statusPill");
  const text = document.getElementById("statusText");
  const nodeEl = document.getElementById("deviceNode");

  if (connected) {
    pill.classList.remove("error");
    text.textContent = "CONNECTED (READY)";
    if (path) nodeEl.textContent = path;
  } else {
    pill.classList.add("error");
    text.textContent = "DISCONNECTED";
  }
}

function renderProfiles(profiles) {
  const select = document.getElementById("profileSelect");
  select.innerHTML = "";
  profiles.forEach(p => {
    const opt = document.createElement("option");
    opt.value = p;
    opt.textContent = p.charAt(0).toUpperCase() + p.slice(1);
    select.appendChild(opt);
  });
}

function renderSettings(s) {
  if (!s) return;

  // Render DPI
  updateDpiDisplay(s.active_dpi);
  renderStages(s.stages);

  // Render Polling
  const hz = String(s.polling_rate).replace(/[^0-9]/g, "");
  updatePollingDisplay(parseInt(hz, 10));

  // Render RGB
  renderRgb(s.rgb);

  // Render Buttons
  renderButtons(s.buttons);
}

function updateDpiDisplay(dpi) {
  document.getElementById("activeDpiValue").textContent = dpi;

  // Find closest slider index
  let idx = DPI_STEPS.indexOf(dpi);
  if (idx === -1) {
    idx = DPI_STEPS.reduce((prev, curr, i) =>
      Math.abs(curr - dpi) < Math.abs(DPI_STEPS[prev] - dpi) ? i : prev, 0);
  }
  document.getElementById("dpiSlider").value = idx;

  // Update chips
  document.querySelectorAll(".preset-chip").forEach(chip => {
    if (parseInt(chip.dataset.dpi, 10) === dpi) {
      chip.classList.add("active");
    } else {
      chip.classList.remove("active");
    }
  });
}

function renderStages(stages) {
  const container = document.getElementById("stagesContainer");
  container.innerHTML = "";

  stages.forEach(st => {
    const row = document.createElement("div");
    row.className = `stage-row ${st.dpi === currentSettings?.active_dpi ? "active" : ""}`;

    const hexColor = st.color ?
      `#${((1 << 24) + (st.color.r << 16) + (st.color.g << 8) + st.color.b).toString(16).slice(1)}` :
      "#00f0ff";

    row.innerHTML = `
      <div class="stage-meta">
        <span class="stage-indicator" style="background:${hexColor}; color:${hexColor}"></span>
        <span class="stage-name">Stage ${st.stage_index + 1}</span>
      </div>
      <div class="stage-controls">
        <select class="stage-dpi-select" data-stage="${st.stage_index}">
          ${DPI_STEPS.map(d => `<option value="${d}" ${d === st.dpi ? "selected" : ""}>${d} DPI</option>`).join("")}
        </select>
        <button class="btn btn-secondary btn-sm btn-activate-stage" data-dpi="${st.dpi}">Select</button>
      </div>
    `;

    container.appendChild(row);
  });

  // Stage DPI select change listener
  container.querySelectorAll(".stage-dpi-select").forEach(sel => {
    sel.addEventListener("change", async (e) => {
      const stageIdx = parseInt(e.target.dataset.stage, 10);
      const newDpi = parseInt(e.target.value, 10);
      const stage = stages[stageIdx];
      stage.dpi = newDpi;
      await postJson("/api/dpi-stage", stage);
      showToast(`Updated Stage ${stageIdx + 1} to ${newDpi} DPI`);
    });
  });

  // Activate stage button listener
  container.querySelectorAll(".btn-activate-stage").forEach(btn => {
    btn.addEventListener("click", async (e) => {
      const dpi = parseInt(e.target.dataset.dpi, 10);
      await setDpi(dpi);
    });
  });
}

function updatePollingDisplay(rate) {
  document.getElementById("activeRateBadge").textContent = `${rate} Hz • ${(1000/rate).toFixed(1)} ms`;

  document.querySelectorAll(".polling-tile").forEach(tile => {
    if (parseInt(tile.dataset.rate, 10) === rate) {
      tile.classList.add("active");
    } else {
      tile.classList.remove("active");
    }
  });
}

function renderRgb(rgb) {
  if (!rgb) return;

  const hexColor = rgb.color ?
    `#${((1 << 24) + (rgb.color.r << 16) + (rgb.color.g << 8) + rgb.color.b).toString(16).slice(1)}` :
    "#00f0ff";

  document.getElementById("currentModeTag").textContent = `${rgb.mode} Mode`;
  document.getElementById("rgbModeSelect").value = rgb.mode;
  document.getElementById("rgbColorInput").value = hexColor;
  document.getElementById("rgbHexInput").value = hexColor;

  document.getElementById("brightnessSlider").value = rgb.brightness + 1;
  document.getElementById("brightnessValText").textContent = `Level ${rgb.brightness + 1}`;

  document.getElementById("speedSlider").value = rgb.speed + 1;
  document.getElementById("speedValText").textContent = `Level ${rgb.speed + 1}`;

  document.getElementById("reverseToggle").checked = !!rgb.direction;
  document.getElementById("symmetryToggle").checked = !!rgb.symmetry;

  // Visual schematic glow update
  updateVisualRgb(hexColor, rgb.brightness + 1);
}

function updateVisualRgb(color, brightness) {
  const halo = document.getElementById("mouseGlowHalo");
  const ambient = document.getElementById("ambientAura");
  const stripL = document.getElementById("rgbStripLeft");
  const stripR = document.getElementById("rgbStripRight");
  const wheel = document.getElementById("schematicWheel");
  const logo = document.getElementById("schematicLogo");

  const opacity = (brightness / 5) * 0.7;

  halo.style.background = color;
  halo.style.opacity = opacity;

  ambient.style.background = `radial-gradient(circle, ${color} 0%, transparent 70%)`;
  ambient.style.opacity = (brightness / 5) * 0.35;

  stripL.style.stroke = color;
  stripR.style.stroke = color;
  wheel.style.stroke = color;
  logo.style.fill = color;
}

function renderButtons(buttons) {
  const container = document.getElementById("buttonsGrid");
  container.innerHTML = "";

  const buttonOrder = ["Left", "Middle", "Right", "Back", "Forward", "DpiLoop"];
  const buttonNumbers = { "Left": 1, "Middle": 2, "Right": 3, "Back": 4, "Forward": 5, "DpiLoop": 6 };

  buttonOrder.forEach(btnKey => {
    const act = buttons[btnKey] || "LeftClick";
    const item = document.createElement("div");
    item.className = "button-map-item";

    item.innerHTML = `
      <div class="btn-label-group">
        <span class="btn-number">${buttonNumbers[btnKey]}</span>
        <span class="btn-name">${BUTTON_LABELS[btnKey] || btnKey}</span>
      </div>
      <select class="styled-select btn-action-select" data-button="${btnKey}">
        ${BUTTON_ACTIONS.map(a => `<option value="${a.name}" ${a.name.toLowerCase().replace(/[^a-z0-9]/g, "") === act.toLowerCase().replace(/[^a-z0-9]/g, "") ? "selected" : ""}>${a.name}</option>`).join("")}
      </select>
    `;

    container.appendChild(item);
  });

  // Action change listener
  container.querySelectorAll(".btn-action-select").forEach(sel => {
    sel.addEventListener("change", async (e) => {
      const btn = e.target.dataset.button;
      const action = e.target.value;
      await postJson("/api/button", { button: btn, action: action });
      showToast(`Mapped ${btn} to ${action}`);
    });
  });
}

function setupEventListeners() {
  // DPI Slider
  const slider = document.getElementById("dpiSlider");
  slider.addEventListener("input", (e) => {
    const dpi = DPI_STEPS[e.target.value];
    document.getElementById("activeDpiValue").textContent = dpi;
  });
  slider.addEventListener("change", async (e) => {
    const dpi = DPI_STEPS[e.target.value];
    await setDpi(dpi);
  });

  // Quick Preset Chips
  document.querySelectorAll(".preset-chip").forEach(chip => {
    chip.addEventListener("click", async () => {
      const dpi = parseInt(chip.dataset.dpi, 10);
      await setDpi(dpi);
    });
  });

  // Polling Rate Tiles
  document.querySelectorAll(".polling-tile").forEach(tile => {
    tile.addEventListener("click", async () => {
      const rate = parseInt(tile.dataset.rate, 10);
      updatePollingDisplay(rate);
      await postJson("/api/polling", { rate });
      showToast(`Report rate set to ${rate} Hz`);
    });
  });

  // RGB Mode Select
  document.getElementById("rgbModeSelect").addEventListener("change", applyRgbForm);

  // RGB Color Pickers
  document.getElementById("rgbColorInput").addEventListener("input", (e) => {
    document.getElementById("rgbHexInput").value = e.target.value;
    updateVisualRgb(e.target.value, parseInt(document.getElementById("brightnessSlider").value, 10));
  });
  document.getElementById("rgbColorInput").addEventListener("change", applyRgbForm);

  document.getElementById("rgbHexInput").addEventListener("change", (e) => {
    let val = e.target.value.trim();
    if (!val.startsWith("#")) val = "#" + val;
    document.getElementById("rgbColorInput").value = val;
    applyRgbForm();
  });

  // Palette Swatches
  document.querySelectorAll(".color-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      document.querySelectorAll(".color-btn").forEach(b => b.classList.remove("active"));
      btn.classList.add("active");
      const color = btn.dataset.color;
      document.getElementById("rgbColorInput").value = color;
      document.getElementById("rgbHexInput").value = color;
      applyRgbForm();
    });
  });

  // Sliders
  document.getElementById("brightnessSlider").addEventListener("input", (e) => {
    document.getElementById("brightnessValText").textContent = `Level ${e.target.value}`;
    updateVisualRgb(document.getElementById("rgbColorInput").value, parseInt(e.target.value, 10));
  });
  document.getElementById("brightnessSlider").addEventListener("change", applyRgbForm);

  document.getElementById("speedSlider").addEventListener("input", (e) => {
    document.getElementById("speedValText").textContent = `Level ${e.target.value}`;
  });
  document.getElementById("speedSlider").addEventListener("change", applyRgbForm);

  // Direction & Symmetry
  document.getElementById("reverseToggle").addEventListener("change", applyRgbForm);
  document.getElementById("symmetryToggle").addEventListener("change", applyRgbForm);

  // Profile Select
  document.getElementById("profileSelect").addEventListener("change", async (e) => {
    const name = e.target.value;
    await postJson("/api/profiles/load", { name });
    await refreshDevice();
    showToast(`Loaded profile '${name}'`);
  });

  // Save to Onboard Flash
  document.getElementById("btnSaveOnboard").addEventListener("click", async () => {
    const btn = document.getElementById("btnSaveOnboard");
    btn.disabled = true;
    try {
      await postJson("/api/save", {});
      showToast("✔ Saved settings permanently to mouse EEPROM!");
    } finally {
      setTimeout(() => { btn.disabled = false; }, 800);
    }
  });

  // Apply & Launch Hardware Replug Guide
  document.getElementById("btnApplyReplug").addEventListener("click", async () => {
    const btn = document.getElementById("btnApplyReplug");
    btn.disabled = true;
    try {
      await applyRgbForm();
      await postJson("/api/save", {});
      openReplugModal();
    } finally {
      setTimeout(() => { btn.disabled = false; }, 800);
    }
  });

  // Modal Close & Skip
  document.getElementById("modalCloseBtn").addEventListener("click", closeReplugModal);
  document.getElementById("btnSkipReplug").addEventListener("click", closeReplugModal);

  // Sync from Mouse
  document.getElementById("btnSyncHardware").addEventListener("click", async () => {
    await refreshDevice();
    showToast("Synced settings from mouse hardware.");
  });

  // Reset Buttons
  document.getElementById("btnResetButtons").addEventListener("click", async () => {
    const defaults = {
      "Left": "LeftClick",
      "Middle": "MiddleClick",
      "Right": "RightClick",
      "Back": "Back",
      "Forward": "Forward",
      "DpiLoop": "DpiLoop"
    };
    for (const [btn, act] of Object.entries(defaults)) {
      await postJson("/api/button", { button: btn, action: act });
    }
    await refreshDevice();
    showToast("Reset all buttons to default functions.");
  });
}

async function setDpi(dpi) {
  updateDpiDisplay(dpi);
  await postJson("/api/dpi", { dpi });
  showToast(`Sensor resolution set to ${dpi} DPI`);
}

async function applyRgbForm() {
  const mode = document.getElementById("rgbModeSelect").value;
  const color = document.getElementById("rgbColorInput").value;
  const brightness = parseInt(document.getElementById("brightnessSlider").value, 10) - 1;
  const speed = parseInt(document.getElementById("speedSlider").value, 10) - 1;
  const reverse = document.getElementById("reverseToggle").checked;
  const symmetry = document.getElementById("symmetryToggle").checked;

  const payload = {
    mode,
    color,
    brightness,
    speed,
    direction: reverse,
    symmetry
  };

  updateVisualRgb(color, brightness + 1);
  await postJson("/api/rgb", payload);
  document.getElementById("currentModeTag").textContent = `${mode} Mode`;
}

async function postJson(url, data) {
  try {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data)
    });
    const result = await res.json();
    if (!res.ok || result.error) {
      showToast(`Error: ${result.error || "Operation failed"}`);
    }
    return result;
  } catch (err) {
    console.error(`POST ${url} failed:`, err);
    showToast(`Error: ${err.message}`);
  }
}

function showToast(msg) {
  const t = document.getElementById("toast");
  t.textContent = msg;
  t.classList.add("show");
  setTimeout(() => {
    t.classList.remove("show");
  }, 3000);
}

// Replug Workflow State
let replugPollInterval = null;
let replugCountdownTimer = null;

function openReplugModal() {
  const modal = document.getElementById("replugModal");
  modal.classList.remove("hidden");
  setReplugStep(2); // Step 1 is already complete (saved to flash), next is unplug
  startReplugMonitor();
}

function closeReplugModal() {
  const modal = document.getElementById("replugModal");
  modal.classList.add("hidden");
  if (replugPollInterval) {
    clearInterval(replugPollInterval);
    replugPollInterval = null;
  }
  if (replugCountdownTimer) {
    clearInterval(replugCountdownTimer);
    replugCountdownTimer = null;
  }
}

function setReplugStep(step) {
  const s1 = document.getElementById("step1");
  const s2 = document.getElementById("step2");
  const s3 = document.getElementById("step3");
  const s4 = document.getElementById("step4");
  const d1 = document.getElementById("div1");
  const d2 = document.getElementById("div2");
  const d3 = document.getElementById("div3");
  const animBox = document.getElementById("modalAnimBox");
  const usbIcon = document.getElementById("modalUsbIcon");
  const countEl = document.getElementById("countdownNumber");
  const instr = document.getElementById("modalInstruction");
  const badge = document.getElementById("modalIconBadge");

  // Reset states
  [s1, s2, s3, s4].forEach(s => s.className = "step-item");
  [d1, d2, d3].forEach(d => d.className = "step-divider");
  badge.className = "modal-icon-badge";
  animBox.className = "status-animation-box pulsing";
  usbIcon.classList.remove("hidden");
  countEl.classList.add("hidden");

  if (step === 2) {
    s1.classList.add("completed");
    s2.classList.add("active");
    d1.classList.add("active");
    badge.classList.add("warning");
    instr.textContent = "Please unplug your mouse USB cable from your computer now.";
  } else if (step === 3) {
    s1.classList.add("completed");
    s2.classList.add("completed");
    s3.classList.add("active");
    d1.classList.add("active");
    d2.classList.add("active");
    badge.classList.add("warning");
    usbIcon.classList.add("hidden");
    countEl.classList.remove("hidden");
  } else if (step === 3.5) {
    s1.classList.add("completed");
    s2.classList.add("completed");
    s3.classList.add("active");
    d1.classList.add("active");
    d2.classList.add("active");
    badge.classList.add("warning");
    usbIcon.classList.remove("hidden");
    countEl.classList.add("hidden");
    instr.textContent = "Hardware discharged! Now plug the mouse USB cable back in.";
  } else if (step === 4) {
    s1.classList.add("completed");
    s2.classList.add("completed");
    s3.classList.add("completed");
    s4.classList.add("active", "completed");
    d1.classList.add("active");
    d2.classList.add("active");
    d3.classList.add("active");
    badge.classList.add("success");
    animBox.classList.remove("pulsing");
    instr.textContent = "✔ Mouse reconnected! New hardware state active and synchronized.";
  }
}

function startReplugMonitor() {
  if (replugPollInterval) clearInterval(replugPollInterval);

  let state = "waiting_unplug";

  replugPollInterval = setInterval(async () => {
    try {
      const info = await fetch("/api/info").then(r => r.json());
      const connected = !!info.connected;
      updateStatus(connected, info.hidraw_path);

      if (state === "waiting_unplug" && !connected) {
        state = "counting";
        setReplugStep(3);
        let secondsLeft = 4;
        const countEl = document.getElementById("countdownNumber");
        const instr = document.getElementById("modalInstruction");
        countEl.textContent = secondsLeft;
        instr.textContent = `Mouse unplugged. Discharging hardware registers: ${secondsLeft}s...`;

        replugCountdownTimer = setInterval(() => {
          secondsLeft -= 1;
          if (secondsLeft > 0) {
            countEl.textContent = secondsLeft;
            instr.textContent = `Mouse unplugged. Discharging hardware registers: ${secondsLeft}s...`;
          } else {
            clearInterval(replugCountdownTimer);
            replugCountdownTimer = null;
            state = "waiting_replug";
            setReplugStep(3.5);
          }
        }, 1000);
      } else if ((state === "waiting_replug" || state === "counting") && connected) {
        if (replugCountdownTimer) {
          clearInterval(replugCountdownTimer);
          replugCountdownTimer = null;
        }
        state = "done";
        clearInterval(replugPollInterval);
        replugPollInterval = null;
        setReplugStep(4);
        await refreshDevice();
        showToast("✔ Hardware reset complete! Active settings synchronized.");
        setTimeout(() => {
          closeReplugModal();
        }, 1800);
      }
    } catch (e) {
      console.warn("Poll check error:", e);
    }
  }, 500);
}
