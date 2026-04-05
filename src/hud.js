const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const body = document.getElementById('hud-body');
const messageText = document.getElementById('message-text');

let defaultTextColor = '#ffffff';
let runtimeBackground = '#000000';

const PASTEL_COLORS = {
  white:  '#ffffff',
  red:    '#ff6b6b',
  green:  '#69db7c',
  blue:   '#74c0fc',
  yellow: '#ffe066',
  teal:   '#63e6be',
  orange: '#ffa94d',
  purple: '#b197fc',
  pink:   '#f783ac',
};

function resolveColor(color) {
  if (!color) return color;
  return PASTEL_COLORS[color.toLowerCase()] || color;
}
let currentTextColor = null;
let autoFit = true;
let fixedFontSize = 72;
let flashTimeout = null;

async function loadSettings() {
  try {
    const s = await invoke('get_settings');
    defaultTextColor = resolveColor(s.default_text_color) || '#ffffff';
    autoFit = s.auto_fit_font;
    fixedFontSize = s.fixed_font_size;
    body.style.setProperty('--text-color', currentTextColor || defaultTextColor);
    fitText();
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

function fitText() {
  if (!messageText.textContent) return;

  if (!autoFit) {
    messageText.style.fontSize = fixedFontSize + 'px';
    return;
  }

  const container = document.getElementById('message-container');
  const maxW = container.clientWidth - 32;
  const maxH = container.clientHeight - 32;

  let lo = 8;
  let hi = 500;

  while (lo < hi - 1) {
    const mid = Math.floor((lo + hi) / 2);
    messageText.style.fontSize = mid + 'px';
    if (messageText.scrollWidth <= maxW && messageText.scrollHeight <= maxH) {
      lo = mid;
    } else {
      hi = mid;
    }
  }

  messageText.style.fontSize = lo + 'px';
}

function clearMessage() {
  while (messageText.firstChild) {
    messageText.removeChild(messageText.firstChild);
  }
}

function setMessageLines(lines, color) {
  clearMessage();
  lines.forEach((line, i) => {
    if (i > 0) {
      messageText.appendChild(document.createElement('br'));
    }
    messageText.appendChild(document.createTextNode(line));
  });
  const c = resolveColor(color) || currentTextColor || defaultTextColor;
  body.style.setProperty('--text-color', c);
  fitText();
}

function flash(color, durationS) {
  if (flashTimeout) clearTimeout(flashTimeout);

  body.classList.remove('flash-fade');
  body.classList.add('flash-active');
  body.style.setProperty('--bg-color', resolveColor(color) || '#ffffff');

  void body.offsetHeight;

  flashTimeout = setTimeout(() => {
    body.classList.remove('flash-active');
    body.classList.add('flash-fade');
    body.style.setProperty('--bg-color', runtimeBackground);

    setTimeout(() => {
      body.classList.remove('flash-fade');
    }, 300);
  }, durationS * 1000);
}

listen('hud-update', (event) => {
  const data = event.payload;

  switch (data.type) {
    case 'Single':
      setMessageLines([data.message], data.color);
      break;

    case 'Lines':
      setMessageLines(data.messages, data.color);
      break;

    case 'Flash':
      setMessageLines([data.message], data.color);
      flash(data.color, data.duration_s);
      break;

    case 'Clear':
      clearMessage();
      break;

    case 'SetColor':
      currentTextColor = resolveColor(data.color);
      body.style.setProperty('--text-color', currentTextColor);
      break;

    case 'SetBackground':
      runtimeBackground = resolveColor(data.color);
      body.style.setProperty('--bg-color', runtimeBackground);
      break;

    case 'SetFontSize':
      if (data.size === 0) {
        autoFit = true;
        fitText();
      } else {
        autoFit = false;
        fixedFontSize = data.size;
        messageText.style.fontSize = data.size + 'px';
      }
      break;
  }
});

const resizeObserver = new ResizeObserver(() => fitText());
resizeObserver.observe(document.getElementById('message-container'));

loadSettings();
