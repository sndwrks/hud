const { invoke } = window.__TAURI__.core;
const { getVersion } = window.__TAURI__.app;
const { WebviewWindow } = window.__TAURI__.webviewWindow;

const COLOR_HEX = {
  white: '#ffffff', red: '#ff6b6b', green: '#69db7c', blue: '#74c0fc',
  yellow: '#ffe066', teal: '#63e6be', orange: '#ffa94d', purple: '#b197fc',
  pink: '#f783ac',
};

const fields = {
  udpPort: document.getElementById('udp-port'),
  tcpPort: document.getElementById('tcp-port'),
  hudWidth: document.getElementById('hud-width'),
  hudHeight: document.getElementById('hud-height'),
  alwaysOnTop: document.getElementById('always-on-top'),
  autoFit: document.getElementById('auto-fit'),
  fixedFontSize: document.getElementById('fixed-font-size'),
};

/* custom color select */
const colorSelect = document.getElementById('text-color');
const colorTrigger = colorSelect.querySelector('.color-select-trigger');
const colorSwatch = colorSelect.querySelector('.color-select-swatch');
const colorLabel = colorSelect.querySelector('.color-select-label');
const colorItems = colorSelect.querySelectorAll('.color-select-item');
let selectedColor = 'white';

function setColorValue(value) {
  selectedColor = value;
  colorSwatch.style.background = COLOR_HEX[value] || '#ffffff';
  colorLabel.textContent = value.charAt(0).toUpperCase() + value.slice(1);
  colorItems.forEach(item => {
    item.classList.toggle('selected', item.dataset.value === value);
  });
}

colorTrigger.addEventListener('click', () => {
  colorSelect.classList.toggle('open');
});

colorItems.forEach(item => {
  item.addEventListener('click', () => {
    setColorValue(item.dataset.value);
    colorSelect.classList.remove('open');
  });
});

document.addEventListener('click', (e) => {
  if (!colorSelect.contains(e.target)) {
    colorSelect.classList.remove('open');
  }
});

const fixedFontGroup = document.getElementById('fixed-font-group');
const statusEl = document.getElementById('status');
const saveBtn = document.getElementById('save-btn');

function toggleFixedFont() {
  if (fields.autoFit.checked) {
    fixedFontGroup.classList.add('hidden');
  } else {
    fixedFontGroup.classList.remove('hidden');
  }
}

fields.autoFit.addEventListener('change', toggleFixedFont);

async function loadSettings() {
  try {
    const s = await invoke('get_settings');
    fields.udpPort.value = s.udp_port;
    fields.tcpPort.value = s.tcp_port;
    fields.hudWidth.value = s.hud_width;
    fields.hudHeight.value = s.hud_height;
    setColorValue(s.default_text_color);
    fields.alwaysOnTop.checked = s.always_on_top;
    fields.autoFit.checked = s.auto_fit_font;
    fields.fixedFontSize.value = s.fixed_font_size;
    toggleFixedFont();
  } catch (e) {
    showStatus('Failed to load settings', 'error');
  }
}

function showStatus(msg, type) {
  statusEl.textContent = msg;
  statusEl.className = 'status ' + type;
  setTimeout(() => {
    statusEl.textContent = '';
    statusEl.className = 'status';
  }, 3000);
}

saveBtn.addEventListener('click', async () => {
  const settings = {
    udp_port: parseInt(fields.udpPort.value, 10),
    tcp_port: parseInt(fields.tcpPort.value, 10),
    hud_width: parseInt(fields.hudWidth.value, 10),
    hud_height: parseInt(fields.hudHeight.value, 10),
    default_text_color: selectedColor,
    always_on_top: fields.alwaysOnTop.checked,
    auto_fit_font: fields.autoFit.checked,
    fixed_font_size: parseInt(fields.fixedFontSize.value, 10),
  };

  try {
    await invoke('save_settings', { settings });
    await invoke('restart_osc', {});
    showStatus('Settings saved', 'success');
  } catch (e) {
    showStatus('Error: ' + e, 'error');
  }
});

document.getElementById('help-btn').addEventListener('click', async () => {
  const helpWin = await WebviewWindow.getByLabel('help');
  if (helpWin) {
    await helpWin.show();
    await helpWin.setFocus();
  }
});

loadSettings();
getVersion().then(v => { document.getElementById('app-version').textContent = 'v' + v; });
