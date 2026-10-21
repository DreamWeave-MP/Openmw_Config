// openmw-config's hero: a chain of openmw.cfg files, resolved live the way the crate resolves it.
//
// Every file in the chain is a pane of smoked glass engraved with its own lines. The root stands at
// the back and each config it names stands in front of the last, because a later config merges over
// the ones before it. The loader walks the chain depth first, as OpenMW's readConfiguration does: a
// light leaves the root along its config= links and wakes each file as it is read, and mods/ reads
// its own patches/ before the chain moves on. Beside the deck stands the result, the content= load
// order, one cartridge per plugin.
//
// What wins is drawn where it happens. A config's replace=content strikes out the content= lines of
// every config before it, and their cartridges shatter. The last encoding= decides how every pane is
// decoded: win1251 turns the engraving Cyrillic, win1250 grows it hačeks. The last
// fallback=Weather_Clear_Sky_Sunrise_Color is the colour of the sky behind the chain, and the last
// directory in the chain carries the quill that marks it as ?userconfig?, where save_user() writes.
//
// None of that moves unless someone rearranges the chain, which the page never mentions. Hold a pane
// still for a moment and it lifts free; drag it and the root's config= entries reorder, the chain is
// walked again and everything above is recomputed. One order has its own consequence.
//
// The scene renders to a half-float target; a bright pass and blurs make the bloom, and the composite
// applies ACES tone mapping, a vignette and dithering. Colours come from the site's CSS tokens. The
// chain stands beside the hero's text, measured at every layout, or above it on a phone. Nothing runs
// while the hero is off screen or the tab is hidden, and the resolution drops if frames run slow.
// Under prefers-reduced-motion one frame is drawn, and changes snap. Until the first frame, and
// without WebGL, a still of the chain stands in its place.

import * as THREE from './vendor/three.module.min.js';

const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;

// The stage: everything is built in these units and scaled to the space beside the text.
const STAGE = { width: 4.4, height: 2.6 };
const STAGE_ASPECT = STAGE.width / STAGE.height;
// The still covers a little more than the stage: perspective carries the deck's corners past it.
export const STILL_PAD = { left: 0.08, right: 0.09, top: 0.035, bottom: 0.13 };
const PANE = { width: 1.6, height: 0.98, depth: 0.03, radius: 0.07 };
const CHIP = { width: 0.98, height: 0.15, depth: 0.05, gap: 0.042 };
const TOWER = { x: 1.58, top: 0.86 };
const HOLD = { mouse: 340, touch: 460 };
const SLOP = { mouse: 6, touch: 11 };

// The chain -----------------------------------------------------------------------------------------

// The files. `lines` are what each openmw.cfg says, in order; the root's config= lines are written
// from the current order. Colours are only for telling the files apart.
const SUNRISE = 'Weather_Clear_Sky_Sunrise_Color';
const CONFIGS = {
  root: {
    id: 'root', path: '/etc/openmw/openmw.cfg', hue: '#b7d98c', fixed: true,
    lines: [
      ['resources', '/usr/share/games/openmw/resources'],
      ['data', '"/games/Morrowind/Data Files"'],
      ['content', 'Morrowind.esm'],
      ['encoding', 'win1252'],
      ['fallback', `${SUNRISE},117,141,164`],
    ],
  },
  purge: {
    id: 'purge', path: 'purge/openmw.cfg', name: 'purge', hue: '#f2c14e',
    lines: [
      ['replace', 'content'],
      ['content', 'Morrowind.esm'],
      ['content', 'Fargoth.esp'],
      ['fallback', `${SUNRISE},255,196,92`],
    ],
  },
  expansions: {
    id: 'expansions', path: 'expansions/openmw.cfg', name: 'expansions', hue: '#ff8a4c',
    lines: [
      ['data', '"/games/Morrowind/Data Files"'],
      ['content', 'Tribunal.esm'],
      ['content', 'Bloodmoon.esm'],
      ['encoding', 'win1251'],
      ['fallback', `${SUNRISE},255,120,64`],
    ],
  },
  mods: {
    id: 'mods', path: 'mods/openmw.cfg', name: 'mods', hue: '#5ac8ff', child: 'patches',
    lines: [
      ['data', 'Tamriel_Data'],
      ['content', 'Tamriel_Data.esm'],
      ['content', 'TR_Mainland.esm'],
      ['encoding', 'win1250'],
      ['fallback', `${SUNRISE},84,176,255`],
      ['config', 'patches'],
    ],
  },
  patches: {
    id: 'patches', path: 'mods/patches/openmw.cfg', hue: '#6fe0c8', nested: true,
    lines: [
      ['data', '.'],
      ['content', 'Patch for Purists.esm'],
    ],
  },
  user: {
    id: 'user', path: '~/.config/openmw/openmw.cfg', name: '"?userconfig?"', hue: '#ff7a9c',
    lines: [
      ['content', 'Better Balmora.esp'],
      ['encoding', 'win1252'],
      ['fallback', `${SUNRISE},183,217,140`],
    ],
  },
};
const DEFAULT_ORDER = ['purge', 'expansions', 'mods', 'user'];

// The chain as the crate reads it. config= is followed depth first, so a config's own config=
// entries load right after it. replace=content discards the content= entries of every config
// before its own and keeps all of its own. encoding= and each fallback key keep their last value.
// The last directory in the chain is the user's config directory.
export function resolve(order) {
  const loaded = [CONFIGS.root];
  for (const id of order) {
    loaded.push(CONFIGS[id]);
    if (CONFIGS[id].child) loaded.push(CONFIGS[CONFIGS[id].child]);
  }
  let content = [];
  const discarded = new Set();
  let encoding = null;
  let fallback = null;
  for (const config of loaded) {
    if (config.lines.some(([key, value]) => key === 'replace' && value === 'content')) {
      for (const entry of content) discarded.add(`${entry.from.id}:${entry.name}`);
      content = [];
    }
    for (const [key, value] of config.lines) {
      if (key === 'content') content.push({ name: value, from: config, key: `${config.id}:${value}` });
      if (key === 'encoding') encoding = { value, from: config };
      if (key === 'fallback' && value.startsWith(SUNRISE)) fallback = { value: value.split(',').slice(1).map(Number), from: config };
    }
  }
  return { loaded, content, discarded, encoding, fallback, user: loaded[loaded.length - 1], order: [...order] };
}

// Encodings: how every engraving reads once the chain's last encoding= is known.
const CYRILLIC = { a: 'а', b: 'б', c: 'ц', d: 'д', e: 'е', f: 'ф', g: 'г', h: 'х', i: 'и', j: 'й', k: 'к', l: 'л', m: 'м', n: 'н', o: 'о', p: 'п', q: 'к', r: 'р', s: 'с', t: 'т', u: 'у', v: 'в', w: 'ш', x: 'кс', y: 'ы', z: 'з' };
const HACEK = { a: 'á', c: 'č', d: 'ď', e: 'ě', i: 'í', n: 'ň', o: 'ó', r: 'ř', s: 'š', t: 'ť', u: 'ů', y: 'ý', z: 'ž' };
function decode(text, encoding) {
  const table = encoding === 'win1251' ? CYRILLIC : encoding === 'win1250' ? HACEK : null;
  if (!table) return text;
  let out = '';
  for (const character of text) {
    const lower = character.toLowerCase();
    const mapped = table[lower];
    if (!mapped) out += character;
    else out += character === lower ? mapped : mapped.charAt(0).toUpperCase() + mapped.slice(1);
  }
  return out;
}

// Helpers -------------------------------------------------------------------------------------------

function cssValue(name, fallback) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

function cssColor(name, fallback) {
  const color = new THREE.Color(fallback);
  const raw = cssValue(name, '');
  if (raw) {
    try { color.setStyle(raw); } catch { /* an unparsable token keeps the fallback */ }
  }
  return color;
}

function random(seed) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const damp = (value, target, rate, dt) => value + (target - value) * (1 - Math.exp(-rate * dt));

// Engraving -----------------------------------------------------------------------------------------

// A pane's engraving: its path, then its lines. The line that wins glows in the file's colour, the
// lines that lost fade, and content= lines a later replace= discarded are struck through. Draggable
// panes carry a small grip of six dots on their right edge, the one hint there is.
function paneCanvas(config, state, fonts, colors, size) {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = Math.round(size * PANE.height / PANE.width);
  const context = canvas.getContext('2d');
  const k = size / 1024;
  const margin = 52 * k;
  const maxWidth = canvas.width - margin * 2 - 40 * k;
  const fit = (text, x, y) => {
    const width = context.measureText(text).width;
    if (width <= maxWidth) {
      context.fillText(text, x, y);
      return width;
    }
    context.save();
    context.translate(x, y);
    context.scale(maxWidth / width, 1);
    context.fillText(text, 0, 0);
    context.restore();
    return maxWidth;
  };
  const encoding = state.encoding;
  context.textBaseline = 'alphabetic';
  context.font = `700 ${40 * k}px ${fonts.mono}`;
  context.fillStyle = config.hue;
  context.globalAlpha = 0.95;
  fit(`# ${decode(config.path, encoding)}`, margin, 82 * k);

  const lines = config.id === 'root'
    ? [...config.lines, ...state.order.map((id) => ['config', CONFIGS[id].name])]
    : config.lines;
  const lineHeight = Math.min(56 * k, (canvas.height - 156 * k) / Math.max(lines.length, 1));
  context.font = `500 ${Math.min(34, lineHeight / k * 0.66) * k}px ${fonts.mono}`;
  lines.forEach(([key, value], index) => {
    const y = 150 * k + index * lineHeight;
    let status = 'normal';
    if (key === 'content' && state.discarded.has(`${config.id}:${value}`)) status = 'discarded';
    if (key === 'encoding') status = state.encodingFrom === config.id ? 'win' : 'lose';
    if (key === 'fallback') status = state.fallbackFrom === config.id ? 'win' : 'lose';
    if (key === 'replace') status = 'win';
    const alpha = status === 'lose' ? 0.32 : status === 'discarded' ? 0.28 : 0.92;
    const keyText = decode(key, encoding);
    const valueText = decode(value, encoding);
    context.globalAlpha = alpha;
    context.fillStyle = status === 'win' ? config.hue : colors.key;
    context.shadowColor = status === 'win' ? config.hue : 'transparent';
    context.shadowBlur = status === 'win' ? 16 * k : 0;
    const keyWidth = context.measureText(keyText).width;
    context.fillText(keyText, margin, y);
    context.fillStyle = colors.faint;
    context.fillText('=', margin + keyWidth, y);
    const equalsWidth = context.measureText('=').width;
    context.fillStyle = status === 'win' ? '#ffffff' : colors.text;
    const valueWidth = fit(valueText, margin + keyWidth + equalsWidth, y);
    context.shadowBlur = 0;
    if (status === 'discarded') {
      context.globalAlpha = 0.7;
      context.strokeStyle = config.hue;
      context.lineWidth = 3 * k;
      context.beginPath();
      context.moveTo(margin - 6 * k, y - 9 * k);
      context.lineTo(margin + keyWidth + equalsWidth + valueWidth + 6 * k, y - 9 * k);
      context.stroke();
    }
  });
  if (!config.fixed && !config.nested) {
    context.globalAlpha = 0.34;
    context.fillStyle = colors.text;
    const gx = canvas.width - 34 * k;
    const gy = canvas.height / 2;
    for (let row = -1; row <= 1; row++) {
      for (let column = 0; column < 2; column++) {
        context.beginPath();
        context.arc(gx + column * 12 * k - 6 * k, gy + row * 13 * k, 3.2 * k, 0, Math.PI * 2);
        context.fill();
      }
    }
  }
  return canvas;
}

// A cartridge's label: its place in the load order and the plugin's name, with its file's colour.
function chipCanvas(index, name, hue, encoding, fonts, colors, size) {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = Math.round(size * CHIP.height / CHIP.width);
  const context = canvas.getContext('2d');
  const k = size / 512;
  context.fillStyle = 'rgba(0,0,0,0)';
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = hue;
  context.globalAlpha = 0.95;
  context.fillRect(0, 0, 9 * k, canvas.height);
  context.textBaseline = 'middle';
  context.font = `700 ${34 * k}px ${fonts.mono}`;
  context.fillText(String(index).padStart(2, '0'), 26 * k, canvas.height / 2 + 2 * k);
  context.fillStyle = colors.text;
  context.font = `500 ${32 * k}px ${fonts.mono}`;
  const text = decode(name, encoding);
  const x = 90 * k;
  const maxWidth = canvas.width - x - 16 * k;
  const width = context.measureText(text).width;
  if (width > maxWidth) {
    context.save();
    context.translate(x, canvas.height / 2 + 2 * k);
    context.scale(maxWidth / width, 1);
    context.fillText(text, 0, 0);
    context.restore();
  } else {
    context.fillText(text, x, canvas.height / 2 + 2 * k);
  }
  return canvas;
}

// The quill that marks the user's config directory, where save_user() writes.
function quillCanvas(hue, encoding, fonts, colors) {
  const canvas = document.createElement('canvas');
  canvas.width = 320;
  canvas.height = 96;
  const context = canvas.getContext('2d');
  context.strokeStyle = hue;
  context.fillStyle = hue;
  context.lineWidth = 3;
  context.shadowColor = hue;
  context.shadowBlur = 10;
  context.beginPath();
  context.moveTo(20, 84);
  context.bezierCurveTo(26, 50, 52, 18, 84, 8);
  context.bezierCurveTo(72, 34, 58, 58, 20, 84);
  context.globalAlpha = 0.9;
  context.fill();
  context.beginPath();
  context.moveTo(14, 90);
  context.lineTo(58, 44);
  context.stroke();
  context.shadowBlur = 0;
  context.fillStyle = colors.text;
  context.font = `600 26px ${fonts.mono}`;
  context.textBaseline = 'middle';
  context.fillText(decode('?userconfig?', encoding), 100, 50);
  return canvas;
}

function headerCanvas(encoding, fonts, colors) {
  const canvas = document.createElement('canvas');
  canvas.width = 512;
  canvas.height = 64;
  const context = canvas.getContext('2d');
  context.fillStyle = colors.faint;
  context.font = `600 26px ${fonts.mono}`;
  context.textBaseline = 'middle';
  context.fillText(decode('# content= load order', encoding), 8, 34);
  return canvas;
}

function canvasTexture(canvas, anisotropy) {
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = anisotropy;
  texture.generateMipmaps = true;
  texture.minFilter = THREE.LinearMipmapLinearFilter;
  return texture;
}

// Geometry ------------------------------------------------------------------------------------------

function roundedRect(width, height, radius) {
  const shape = new THREE.Shape();
  const x = -width / 2;
  const y = -height / 2;
  shape.moveTo(x + radius, y);
  shape.lineTo(x + width - radius, y);
  shape.quadraticCurveTo(x + width, y, x + width, y + radius);
  shape.lineTo(x + width, y + height - radius);
  shape.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  shape.lineTo(x + radius, y + height);
  shape.quadraticCurveTo(x, y + height, x, y + height - radius);
  shape.lineTo(x, y + radius);
  shape.quadraticCurveTo(x, y, x + radius, y);
  return shape;
}

function paneGeometry() {
  const geometry = new THREE.ExtrudeGeometry(roundedRect(PANE.width, PANE.height, PANE.radius), {
    depth: PANE.depth, bevelEnabled: true, bevelThickness: 0.012, bevelSize: 0.012, bevelSegments: 3, curveSegments: 10,
  });
  geometry.translate(0, 0, -PANE.depth / 2);
  geometry.computeVertexNormals();
  return geometry;
}

function chipGeometry() {
  const geometry = new THREE.ExtrudeGeometry(roundedRect(CHIP.width, CHIP.height, 0.022), {
    depth: CHIP.depth, bevelEnabled: true, bevelThickness: 0.008, bevelSize: 0.008, bevelSegments: 2, curveSegments: 6,
  });
  geometry.translate(0, 0, -CHIP.depth / 2);
  return geometry;
}

// A slotted run of positions from the back of the deck to its front.
function slotPosition(index, target) {
  return target.set(-1.42 + index * 0.3, 0.68 - index * 0.33, -0.8 + index * 0.4);
}

// Light ---------------------------------------------------------------------------------------------

// A dark room with softboxes, filtered once into an environment map for the glass and the metal.
function environment(renderer, accent) {
  const scene = new THREE.Scene();
  const disposables = [];
  const add = (geometry, color, position) => {
    const material = new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(...position);
    mesh.lookAt(0, 0, 0);
    scene.add(mesh);
    disposables.push(geometry, material);
  };
  const room = new THREE.Mesh(new THREE.BoxGeometry(30, 18, 30), new THREE.MeshBasicMaterial({ color: new THREE.Color(0.02, 0.025, 0.02), side: THREE.BackSide }));
  scene.add(room);
  disposables.push(room.geometry, room.material);
  add(new THREE.PlaneGeometry(14, 3), new THREE.Color(0.9, 1.0, 0.92).multiplyScalar(3.2), [-6, 8, 6]);
  add(new THREE.PlaneGeometry(1.2, 14), new THREE.Color(0.85, 0.92, 1.0).multiplyScalar(4), [9, 0, 4]);
  add(new THREE.PlaneGeometry(1.0, 12), accent.clone().multiplyScalar(4), [-10, 0, -4]);
  add(new THREE.PlaneGeometry(20, 0.5), new THREE.Color(1.0, 0.86, 0.66).multiplyScalar(3), [0, -1, -13]);
  add(new THREE.PlaneGeometry(6, 6), new THREE.Color(1, 1, 1).multiplyScalar(1.4), [3, 4, 12]);
  const generator = new THREE.PMREMGenerator(renderer);
  const target = generator.fromScene(scene, 0.03);
  generator.dispose();
  for (const item of disposables) item.dispose();
  return target;
}

// Shaders -------------------------------------------------------------------------------------------

const FULLSCREEN_VERTEX = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`;

const NOISE = /* glsl */ `
  float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
  float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
  }
  float fbm(vec2 p) {
    float sum = 0.0;
    float amplitude = 0.5;
    for (int i = 0; i < 5; i++) {
      sum += amplitude * noise(p);
      p = p * 2.03 + vec2(17.1, 9.2);
      amplitude *= 0.5;
    }
    return sum;
  }
`;

// The sky behind the chain: the hero's own gradient, a dawn low behind the deck in the colour of the
// winning fallback=Weather_Clear_Sky_Sunrise_Color, and slow
// mist drifting through it.
const SKY_FRAGMENT = /* glsl */ `
  uniform vec3 uTop;
  uniform vec3 uBottom;
  uniform vec3 uSunrise;
  uniform vec3 uAccent;
  uniform vec2 uCenter;
  uniform vec2 uResolution;
  uniform float uRadius;
  uniform float uTime;
  uniform float uFlash;
  varying vec2 vUv;
  ${NOISE}
  void main() {
    vec3 color = mix(uBottom, uTop, vUv.y);
    vec2 aspect = vec2(uResolution.x / max(uResolution.y, 1.0), 1.0);
    vec2 d = (vUv - uCenter) * aspect / max(uRadius, 0.001);
    float r2 = dot(d, d);
    float horizon = exp(-pow(abs(d.y + 0.62), 1.6) * 2.6) * exp(-d.x * d.x * 0.16);
    float glow = exp(-r2 * 0.5);
    float mist = fbm(d * 1.2 + vec2(uTime * 0.018, -uTime * 0.011));
    color += uSunrise * horizon * (0.07 + 0.05 * mist);
    color += uSunrise * glow * 0.03 * (0.6 + 0.4 * mist);
    color += uAccent * exp(-r2 * 0.2) * 0.02 * mist;
    color += uSunrise * uFlash * glow * 0.35;
    gl_FragColor = vec4(color, 1.0);
  }
`;

// The config= links: flowing dashes from a file to the file it names, brightest while the loader's
// light is travelling along them.
const LINK_VERTEX = /* glsl */ `
  attribute float aT;
  varying float vT;
  void main() {
    vT = aT;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;
const LINK_FRAGMENT = /* glsl */ `
  uniform vec3 uColor;
  uniform float uTime;
  uniform float uBoost;
  uniform float uOpacity;
  varying float vT;
  void main() {
    float dash = smoothstep(0.35, 0.5, fract(vT * 9.0 - uTime * 0.9)) * (1.0 - smoothstep(0.5, 0.65, fract(vT * 9.0 - uTime * 0.9)));
    float ends = smoothstep(0.0, 0.08, vT) * (1.0 - smoothstep(0.92, 1.0, vT));
    float intensity = (0.25 + dash * 0.9) * (1.0 + uBoost * 4.0) * ends;
    gl_FragColor = vec4(uColor * intensity * uOpacity, 1.0);
  }
`;

// Motes of dust drifting through the light.
const MOTE_VERTEX = /* glsl */ `
  uniform float uTime;
  uniform float uPixel;
  attribute vec4 aSeed;
  varying float vAlpha;
  void main() {
    vec3 p = vec3((aSeed.x - 0.5) * 4.6, (aSeed.y - 0.5) * 3.2, (aSeed.z - 0.5) * 2.6);
    p.x += sin(uTime * 0.07 + aSeed.w * 30.0) * 0.22;
    p.y += mod(uTime * 0.03 * (0.4 + aSeed.w), 3.2);
    p.y = mod(p.y + 1.6, 3.2) - 1.6;
    vec4 view = modelViewMatrix * vec4(p, 1.0);
    gl_Position = projectionMatrix * view;
    vAlpha = (0.25 + 0.75 * fract(aSeed.w * 7.3)) * (1.0 - smoothstep(1.1, 1.6, abs(p.y)));
    gl_PointSize = uPixel * (0.006 + aSeed.w * 0.012) / max(-view.z, 0.1);
  }
`;
const MOTE_FRAGMENT = /* glsl */ `
  uniform vec3 uColor;
  varying float vAlpha;
  void main() {
    vec2 d = gl_PointCoord - 0.5;
    float r = dot(d, d);
    float disc = exp(-r * 18.0) + (1.0 - smoothstep(0.2, 0.25, r)) * 0.15;
    gl_FragColor = vec4(uColor * disc * vAlpha, 1.0);
  }
`;

// Any NaN or infinity a driver produces is zeroed and bright values capped before the bloom, which
// would otherwise smear a single bad pixel into a black square.
const SCRUB = /* glsl */ `
  vec3 scrub(vec3 c) {
    if (any(isnan(c)) || any(isinf(c)) || c.r != c.r || c.g != c.g || c.b != c.b) return vec3(0.0);
    return clamp(c, 0.0, 64.0);
  }
`;

const BRIGHT_FRAGMENT = /* glsl */ `
  uniform sampler2D tInput;
  uniform float uThreshold;
  varying vec2 vUv;
  ${SCRUB}
  void main() {
    vec3 c = scrub(texture2D(tInput, vUv).rgb);
    float luma = dot(c, vec3(0.2126, 0.7152, 0.0722));
    gl_FragColor = vec4(c * smoothstep(uThreshold, uThreshold + 0.6, luma), 1.0);
  }
`;

const BLUR_FRAGMENT = /* glsl */ `
  uniform sampler2D tInput;
  uniform vec2 uDirection;
  varying vec2 vUv;
  void main() {
    vec3 sum = texture2D(tInput, vUv).rgb * 0.2270270270;
    sum += texture2D(tInput, vUv + uDirection * 1.3846153846).rgb * 0.3162162162;
    sum += texture2D(tInput, vUv - uDirection * 1.3846153846).rgb * 0.3162162162;
    sum += texture2D(tInput, vUv + uDirection * 3.2307692308).rgb * 0.0702702703;
    sum += texture2D(tInput, vUv - uDirection * 3.2307692308).rgb * 0.0702702703;
    gl_FragColor = vec4(sum, 1.0);
  }
`;

const COMPOSITE_FRAGMENT = /* glsl */ `
  uniform sampler2D tScene;
  uniform sampler2D tBloomNear;
  uniform sampler2D tBloomFar;
  uniform vec2 uCenter;
  uniform float uTime;
  varying vec2 vUv;
  vec3 aces(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
  }
  float dither(vec2 p) {
    return fract(sin(dot(p + fract(uTime), vec2(12.9898, 78.233))) * 43758.5453) - 0.5;
  }
  ${SCRUB}
  void main() {
    vec2 offset = (vUv - uCenter) * 0.0012;
    vec3 color;
    color.r = scrub(texture2D(tScene, vUv + offset).rgb).r;
    color.g = scrub(texture2D(tScene, vUv).rgb).g;
    color.b = scrub(texture2D(tScene, vUv - offset).rgb).b;
    color += scrub(texture2D(tBloomNear, vUv).rgb) * 0.65 + scrub(texture2D(tBloomFar, vUv).rgb) * 0.5;
    color = aces(color * 0.95);
    vec2 v = vUv - 0.5;
    color *= 1.0 - dot(v, v) * 0.35;
    color = pow(max(color, vec3(0.0)), vec3(1.0 / 2.2));
    color += dither(gl_FragCoord.xy) / 255.0;
    gl_FragColor = vec4(color, 1.0);
  }
`;

function fullscreenMaterial(fragmentShader, uniforms) {
  return new THREE.ShaderMaterial({ vertexShader: FULLSCREEN_VERTEX, fragmentShader, uniforms, depthTest: false, depthWrite: false });
}

// Layout --------------------------------------------------------------------------------------------

// Where the hero's words and controls are, so the chain can stand clear of them.
function textRects(text) {
  const rects = [];
  const range = document.createRange();
  const walker = document.createTreeWalker(text, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => (node.nodeValue.trim() ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT),
  });
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    range.selectNodeContents(node);
    for (const rect of range.getClientRects()) rects.push(rect);
  }
  for (const element of text.querySelectorAll('a, button, input, select, img, svg, .dw-command, .dw-badge')) rects.push(element.getBoundingClientRect());
  return rects.filter((rect) => rect.width > 0 && rect.height > 0);
}

// The largest stage-shaped box clear of the text: beside all of it, beside the title rows above the
// summary, or above it all where the stylesheet leaves room on a phone. Relative to the art.
function placement(root) {
  const hero = root.closest('.dw-hero') || root.parentElement;
  const box = root.getBoundingClientRect();
  const text = hero.querySelector('.dw-hero__text') || hero.querySelector('.dw-shell');
  const strip = hero.querySelector('.dw-strip');
  const shellElement = hero.querySelector('.dw-hero__grid') || hero.querySelector('.dw-shell') || hero;
  const shellStyle = getComputedStyle(shellElement);
  const shellBox = shellElement.getBoundingClientRect();
  const shell = strip ? strip.getBoundingClientRect() : { left: shellBox.left + parseFloat(shellStyle.paddingLeft), right: shellBox.right - parseFloat(shellStyle.paddingRight) };
  const summary = hero.querySelector('.dw-hero__summary');
  const floor = strip ? strip.getBoundingClientRect().top : box.bottom - 24;
  const rects = text ? textRects(text) : [];
  if (!rects.length) return { x: box.width * 0.72, y: box.height * 0.45, height: Math.min(box.width * 0.4 / STAGE_ASPECT, box.height * 0.7), above: false };
  const gap = 28;
  const right = Math.max(...rects.map((rect) => rect.right));
  const top = Math.min(...rects.map((rect) => rect.top));
  const summaryTop = summary ? summary.getBoundingClientRect().top : floor;
  const headRects = rects.filter((rect) => rect.bottom <= summaryTop + 1);
  const headRight = headRects.length ? Math.max(...headRects.map((rect) => rect.right)) : right;
  const candidates = [
    { x0: right + gap, x1: shell.right, y0: box.top + 14, y1: floor - 14, above: false },
    { x0: headRight + gap, x1: shell.right, y0: box.top + 10, y1: summaryTop - 10, above: false },
    { x0: shell.left, x1: shell.right, y0: box.top + 8, y1: top - 12, above: true },
  ].map((region) => ({ ...region, height: Math.max(0, Math.min(region.y1 - region.y0, (region.x1 - region.x0) / STAGE_ASPECT)) }));
  const best = candidates.reduce((a, b) => (b.height > a.height ? b : a));
  const height = Math.min(best.height * 0.96, 430);
  const stageWidth = height * STAGE_ASPECT;
  const x = best.above ? (best.x0 + best.x1) / 2 : Math.min(best.x1 - stageWidth / 2, (best.x0 + best.x1) / 2 + (best.x1 - best.x0 - stageWidth) * 0.3);
  return { x: x - box.left, y: (best.y0 + best.y1) / 2 - box.top, height, above: best.above };
}

// The scene -----------------------------------------------------------------------------------------

function mount(root) {
  const still = document.createElement('img');
  still.className = 'ocfg-hero__still';
  still.alt = '';
  still.decoding = 'async';
  still.src = new URL('../img/ocfg-chain.webp', import.meta.url).href;
  root.append(still);

  const canvas = document.createElement('canvas');
  canvas.className = 'ocfg-hero__canvas';
  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false, powerPreference: 'high-performance' });
  } catch {
    placeStill();
    return;
  }
  if (!renderer.capabilities.isWebGL2) {
    renderer.dispose();
    placeStill();
    return;
  }
  renderer.autoClear = false;
  renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
  root.append(canvas);

  const small = Math.min(innerWidth, innerHeight) < 700;
  const floatTargets = renderer.extensions.has('EXT_color_buffer_float') || renderer.extensions.has('EXT_color_buffer_half_float');
  const targetType = floatTargets ? THREE.HalfFloatType : THREE.UnsignedByteType;
  const makeTarget = () => new THREE.WebGLRenderTarget(1, 1, { type: targetType, depthBuffer: false });
  const sceneTarget = new THREE.WebGLRenderTarget(1, 1, { type: targetType, samples: small ? 2 : 4 });
  const bloomTargets = [makeTarget(), makeTarget(), makeTarget(), makeTarget()];
  const anisotropy = Math.min(8, renderer.capabilities.getMaxAnisotropy());

  const accent = cssColor('--dw-accent', '#b7d98c');
  const top = cssColor('--dw-bg-1', '#10150c');
  const bottom = cssColor('--dw-bg-0', '#090c07');
  const colors = { text: cssValue('--dw-text', '#e5e9e1'), key: cssValue('--dw-text-muted', '#b5bcad'), faint: cssValue('--dw-text-faint', '#8a9282') };
  const fonts = { mono: cssValue('--dw-font-mono', 'ui-monospace, monospace') };

  const camera = new THREE.PerspectiveCamera(30, 1, 0.1, 80);
  camera.position.set(0, 0.35, 10);
  camera.lookAt(0, 0, 0);

  const scene = new THREE.Scene();
  const envTarget = environment(renderer, accent);
  scene.environment = envTarget.texture;

  const quad = new THREE.PlaneGeometry(2, 2);
  const skyUniforms = {
    uTop: { value: top },
    uBottom: { value: bottom },
    uSunrise: { value: new THREE.Color(0.3, 0.4, 0.3) },
    uAccent: { value: accent },
    uCenter: { value: new THREE.Vector2(0.75, 0.5) },
    uResolution: { value: new THREE.Vector2(1, 1) },
    uRadius: { value: 0.3 },
    uTime: { value: 0 },
    uFlash: { value: 0 },
  };
  const sky = new THREE.Mesh(quad, fullscreenMaterial(SKY_FRAGMENT, skyUniforms));
  sky.frustumCulled = false;
  sky.renderOrder = -10;
  scene.add(sky);

  // The stage is placed beside the text; the deck turns within it, the load order faces the viewer.
  const stage = new THREE.Group();
  const deck = new THREE.Group();
  const tower = new THREE.Group();
  stage.add(deck, tower);
  scene.add(stage);

  // Lights: a cool key, a rim in the colour of the dawn, the pointer's lamp, the loader's light.
  const key = new THREE.DirectionalLight(new THREE.Color(0.9, 0.97, 1.0), 1.4);
  key.position.set(-4, 6, 7);
  const rim = new THREE.DirectionalLight(new THREE.Color(1, 0.8, 0.6), 2.2);
  rim.position.set(5, 1, -6);
  const lamp = new THREE.PointLight(new THREE.Color(1.0, 0.95, 0.85), 0, 0, 2);
  const loaderLight = new THREE.PointLight(new THREE.Color(1, 1, 1), 0, 0, 2);
  scene.add(key, rim, lamp, loaderLight);

  // The panes.
  const paneSize = small ? 512 : 1024;
  const glassGeometry = paneGeometry();
  const textGeometry = new THREE.PlaneGeometry(PANE.width * 0.97, PANE.height * 0.97);
  const panes = {};
  for (const config of Object.values(CONFIGS)) {
    const hue = new THREE.Color(config.hue);
    const glass = new THREE.MeshPhysicalMaterial({
      color: hue.clone().multiplyScalar(0.05).add(new THREE.Color(0.012, 0.016, 0.012)),
      metalness: 0,
      roughness: 0.2,
      clearcoat: 1,
      clearcoatRoughness: 0.16,
      iridescence: 0.55,
      iridescenceIOR: 1.35,
      iridescenceThicknessRange: [180, 520],
      specularIntensity: 0.55,
      envMapIntensity: 1.5,
      transparent: true,
      opacity: 0.84,
      depthWrite: false,
    });
    const edge = new THREE.MeshStandardMaterial({ color: hue.clone().multiplyScalar(0.3), emissive: hue, emissiveIntensity: 0.5, metalness: 0.4, roughness: 0.3 });
    const body = new THREE.Mesh(glassGeometry, [glass, edge]);
    const textMaterial = new THREE.MeshBasicMaterial({ transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, color: new THREE.Color(1.25, 1.25, 1.25) });
    const text = new THREE.Mesh(textGeometry, textMaterial);
    text.position.z = PANE.depth / 2 + 0.016;
    const group = new THREE.Group();
    const lift = new THREE.Group();
    lift.add(body, text);
    group.add(lift);
    if (config.nested) group.scale.setScalar(0.5);
    deck.add(group);
    body.userData.pane = config.id;
    panes[config.id] = {
      config, group, lift, body, glass, edge, text, textMaterial, hue,
      position: new THREE.Vector3(), velocity: new THREE.Vector3(), target: new THREE.Vector3(),
      read: 0, charge: 0, shake: 0, liftAmount: 0, bakedKey: '',
    };
  }

  // The config= links, from the root to each file it names and from mods/ to its patches/.
  const LINK_POINTS = 40;
  function makeLink(hue) {
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(LINK_POINTS * 3), 3));
    const t = new Float32Array(LINK_POINTS);
    for (let i = 0; i < LINK_POINTS; i++) t[i] = i / (LINK_POINTS - 1);
    geometry.setAttribute('aT', new THREE.BufferAttribute(t, 1));
    const material = new THREE.ShaderMaterial({
      vertexShader: LINK_VERTEX, fragmentShader: LINK_FRAGMENT,
      uniforms: { uColor: { value: new THREE.Color(hue).multiplyScalar(1.4) }, uTime: { value: 0 }, uBoost: { value: 0 }, uOpacity: { value: 1 } },
      transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
    });
    const line = new THREE.Line(geometry, material);
    line.frustumCulled = false;
    deck.add(line);
    return { line, geometry, material, boost: 0 };
  }
  const links = {};
  for (const id of [...DEFAULT_ORDER, 'patches']) links[id] = makeLink(CONFIGS[id].hue);
  const curve = new THREE.CubicBezierCurve3(new THREE.Vector3(), new THREE.Vector3(), new THREE.Vector3(), new THREE.Vector3());
  const linkPoint = new THREE.Vector3();

  // The load order: one cartridge per content= entry.
  const chipGeometryShared = chipGeometry();
  const chipFaceGeometry = new THREE.PlaneGeometry(CHIP.width * 0.96, CHIP.height * 0.8);
  const chips = new Map();
  const chipSize = small ? 256 : 512;
  function makeChip(entry) {
    const hue = new THREE.Color(entry.from.hue);
    const body = new THREE.Mesh(chipGeometryShared, [
      new THREE.MeshPhysicalMaterial({ color: new THREE.Color(0.05, 0.055, 0.05), metalness: 0.85, roughness: 0.28, clearcoat: 0.6, envMapIntensity: 1.2 }),
      new THREE.MeshStandardMaterial({ color: hue.clone().multiplyScalar(0.4), emissive: hue, emissiveIntensity: 0.9, metalness: 0.6, roughness: 0.25 }),
    ]);
    const faceMaterial = new THREE.MeshBasicMaterial({ transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, color: new THREE.Color(1.15, 1.15, 1.15) });
    const face = new THREE.Mesh(chipFaceGeometry, faceMaterial);
    face.position.z = CHIP.depth / 2 + 0.012;
    const group = new THREE.Group();
    group.add(body, face);
    tower.add(group);
    return {
      entry, group, body, face, faceMaterial, hue, bakedKey: '',
      position: new THREE.Vector3(), velocity: new THREE.Vector3(), target: new THREE.Vector3(),
      appear: 0, hop: 0, index: 0,
    };
  }
  function disposeChip(chip) {
    tower.remove(chip.group);
    for (const material of chip.body.material) material.dispose();
    if (chip.faceMaterial.map) chip.faceMaterial.map.dispose();
    chip.faceMaterial.dispose();
  }

  const header = new THREE.Mesh(new THREE.PlaneGeometry(CHIP.width, CHIP.width / 8), new THREE.MeshBasicMaterial({ transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }));
  header.position.set(TOWER.x, TOWER.top + 0.15, 0.2);
  tower.add(header);

  // The quill that marks the user's config directory.
  const quill = new THREE.Mesh(new THREE.PlaneGeometry(0.62, 0.186), new THREE.MeshBasicMaterial({ transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, color: new THREE.Color(1.3, 1.3, 1.3) }));
  const quillPosition = new THREE.Vector3(9, 9, 9);
  const quillQuaternion = new THREE.Quaternion();
  deck.add(quill);

  // Shards, for plugins a replace= discarded.
  const SHARDS = small ? 140 : 320;
  const shardMesh = new THREE.InstancedMesh(new THREE.TetrahedronGeometry(0.028), new THREE.MeshStandardMaterial({ color: 0x222222, emissive: 0xffffff, emissiveIntensity: 1.4, metalness: 0.3, roughness: 0.4 }), SHARDS);
  shardMesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
  shardMesh.setColorAt(0, new THREE.Color());
  shardMesh.frustumCulled = false;
  tower.add(shardMesh);
  const shards = Array.from({ length: SHARDS }, () => ({ life: 0, position: new THREE.Vector3(), velocity: new THREE.Vector3(), rotation: new THREE.Euler(), spin: new THREE.Vector3(), color: new THREE.Color() }));
  let shardCursor = 0;
  const shardDummy = new THREE.Object3D();
  const shardRandom = random(7);
  function burst(position, color, count, power) {
    if (reduceMotion) return;
    for (let i = 0; i < count; i++) {
      const shard = shards[shardCursor];
      shardCursor = (shardCursor + 1) % SHARDS;
      shard.life = 1;
      shard.position.copy(position).add(new THREE.Vector3((shardRandom() - 0.5) * CHIP.width, (shardRandom() - 0.5) * CHIP.height, 0));
      shard.velocity.set((shardRandom() - 0.5) * 1.6 * power, (shardRandom() * 1.4 + 0.2) * power, (shardRandom() - 0.2) * 1.2 * power);
      shard.rotation.set(shardRandom() * 6, shardRandom() * 6, shardRandom() * 6);
      shard.spin.set((shardRandom() - 0.5) * 14, (shardRandom() - 0.5) * 14, (shardRandom() - 0.5) * 14);
      shard.color.copy(color).multiplyScalar(0.6 + shardRandom() * 0.8);
    }
  }

  // Fargoth's ring, for the one order that leaves nothing but Fargoth.
  const ring = new THREE.Mesh(new THREE.TorusGeometry(0.15, 0.042, 32, 96), new THREE.MeshPhysicalMaterial({ color: new THREE.Color(1.0, 0.78, 0.34), metalness: 1, roughness: 0.16, clearcoat: 0.6, envMapIntensity: 2.2, emissive: new THREE.Color(0.35, 0.22, 0.05), emissiveIntensity: 0.4 }));
  ring.visible = false;
  tower.add(ring);
  const ringState = { active: false, position: new THREE.Vector3(), velocity: new THREE.Vector3(), tumble: 0, settle: 0, fade: 0, glint: 0, joy: 0 };
  // A shaft of light from above, onto the one plugin left standing.
  const shaft = new THREE.Mesh(new THREE.CylinderGeometry(0.2, 0.62, 3.2, 48, 1, true), new THREE.ShaderMaterial({
    vertexShader: /* glsl */ `
      varying vec2 vUv;
      varying vec3 vNormal;
      varying vec3 vView;
      void main() {
        vUv = uv;
        vec4 world = modelViewMatrix * vec4(position, 1.0);
        vNormal = normalize(normalMatrix * normal);
        vView = -world.xyz;
        gl_Position = projectionMatrix * world;
      }
    `,
    fragmentShader: /* glsl */ `
      uniform vec3 uColor;
      uniform float uOpacity;
      uniform float uTime;
      varying vec2 vUv;
      varying vec3 vNormal;
      varying vec3 vView;
      void main() {
        float viewLength = max(length(vView), 1e-4);
        float facing = abs(dot(vNormal, vView / viewLength));
        float edge = facing * facing;
        float fall = smoothstep(0.0, 0.25, vUv.y) * (1.0 - smoothstep(0.8, 1.0, vUv.y));
        float motes = 0.75 + 0.25 * sin(vUv.x * 40.0 + uTime * 1.3) * sin(vUv.y * 23.0 - uTime * 2.1);
        gl_FragColor = vec4(uColor * edge * fall * motes * uOpacity, 1.0);
      }
    `,
    uniforms: { uColor: { value: new THREE.Color(1.0, 0.8, 0.42).multiplyScalar(0.9) }, uOpacity: { value: 0 }, uTime: { value: 0 } },
    transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, side: THREE.DoubleSide,
  }));
  shaft.visible = false;
  tower.add(shaft);

  // Dust.
  const MOTES = small ? 120 : 260;
  const moteGeometry = new THREE.BufferGeometry();
  const moteSeeds = new Float32Array(MOTES * 4);
  const moteRandom = random(31);
  for (let i = 0; i < moteSeeds.length; i++) moteSeeds[i] = moteRandom();
  moteGeometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(MOTES * 3), 3));
  moteGeometry.setAttribute('aSeed', new THREE.BufferAttribute(moteSeeds, 4));
  const moteUniforms = { uTime: { value: 0 }, uPixel: { value: 400 }, uColor: { value: new THREE.Color(0.9, 1, 0.85).multiplyScalar(0.9) } };
  const motes = new THREE.Points(moteGeometry, new THREE.ShaderMaterial({ vertexShader: MOTE_VERTEX, fragmentShader: MOTE_FRAGMENT, uniforms: moteUniforms, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }));
  motes.frustumCulled = false;
  stage.add(motes);

  // The loader's light.
  const loaderBead = new THREE.Mesh(new THREE.SphereGeometry(0.022, 20, 12), new THREE.MeshBasicMaterial({ color: new THREE.Color(1.5, 1.5, 1.3) }));
  loaderBead.visible = false;
  deck.add(loaderBead);

  // Post-processing.
  const postScene = new THREE.Scene();
  const postCamera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
  const postQuad = new THREE.Mesh(quad);
  postQuad.frustumCulled = false;
  postScene.add(postQuad);
  const brightMaterial = fullscreenMaterial(BRIGHT_FRAGMENT, { tInput: { value: sceneTarget.texture }, uThreshold: { value: 1.0 } });
  const blurMaterial = fullscreenMaterial(BLUR_FRAGMENT, { tInput: { value: null }, uDirection: { value: new THREE.Vector2() } });
  const copyMaterial = fullscreenMaterial(/* glsl */ `
    uniform sampler2D tInput;
    varying vec2 vUv;
    void main() { gl_FragColor = texture2D(tInput, vUv); }
  `, { tInput: { value: null } });
  const compositeMaterial = fullscreenMaterial(COMPOSITE_FRAGMENT, {
    tScene: { value: sceneTarget.texture },
    tBloomNear: { value: bloomTargets[0].texture },
    tBloomFar: { value: bloomTargets[2].texture },
    uCenter: { value: new THREE.Vector2(0.7, 0.5) },
    uTime: { value: 0 },
  });
  function pass(material, target) {
    postQuad.material = material;
    renderer.setRenderTarget(target);
    renderer.render(postScene, postCamera);
  }
  function blur(target, scratch, radius) {
    blurMaterial.uniforms.tInput.value = target.texture;
    blurMaterial.uniforms.uDirection.value.set(radius / target.width, 0);
    pass(blurMaterial, scratch);
    blurMaterial.uniforms.tInput.value = scratch.texture;
    blurMaterial.uniforms.uDirection.value.set(0, radius / target.height);
    pass(blurMaterial, target);
  }

  // The chain's state ---------------------------------------------------------------------------------

  let order = [...DEFAULT_ORDER];
  let result = resolve(order);
  const sunrise = new THREE.Color();
  const sunriseTarget = new THREE.Color();
  const fallbackColor = (values) => new THREE.Color(values[0] / 255, values[1] / 255, values[2] / 255);
  sunrise.copy(fallbackColor(result.fallback.value));
  sunriseTarget.copy(sunrise);

  function bakePane(pane) {
    const state = { order: result.order, discarded: result.discarded, encoding: result.encoding.value, encodingFrom: result.encoding.from.id, fallbackFrom: result.fallback.from.id };
    const keyText = JSON.stringify([state.order, [...state.discarded], state.encoding, state.encodingFrom, state.fallbackFrom, fonts.mono]);
    if (keyText === pane.bakedKey) return;
    pane.bakedKey = keyText;
    const texture = canvasTexture(paneCanvas(pane.config, state, fonts, colors, pane.config.nested ? paneSize / 2 : paneSize), anisotropy);
    if (pane.textMaterial.map) pane.textMaterial.map.dispose();
    pane.textMaterial.map = texture;
    pane.textMaterial.needsUpdate = true;
  }
  function bakeChip(chip) {
    const keyText = `${chip.index}|${result.encoding.value}|${fonts.mono}`;
    if (keyText === chip.bakedKey) return;
    chip.bakedKey = keyText;
    const texture = canvasTexture(chipCanvas(chip.index, chip.entry.name, chip.entry.from.hue, result.encoding.value, fonts, colors, chipSize), anisotropy);
    if (chip.faceMaterial.map) chip.faceMaterial.map.dispose();
    chip.faceMaterial.map = texture;
    chip.faceMaterial.needsUpdate = true;
  }
  let bakedDecor = '';
  function bakeDecor() {
    const keyText = `${result.encoding.value}|${result.user.id}|${fonts.mono}`;
    if (keyText === bakedDecor) return;
    bakedDecor = keyText;
    for (const [mesh, canvas] of [[quill, quillCanvas(result.user.hue, result.encoding.value, fonts, colors)], [header, headerCanvas(result.encoding.value, fonts, colors)]]) {
      if (mesh.material.map) mesh.material.map.dispose();
      mesh.material.map = canvasTexture(canvas, anisotropy);
      mesh.material.needsUpdate = true;
    }
  }

  const tmp = new THREE.Vector3();
  const tmp2 = new THREE.Vector3();
  function deckToTower(point, target) {
    target.copy(point);
    deck.updateMatrixWorld();
    tower.updateMatrixWorld();
    deck.localToWorld(target);
    return tower.worldToLocal(target);
  }
  function chipSlot(index, target) {
    return target.set(TOWER.x, TOWER.top - index * (CHIP.height + CHIP.gap), 0.25);
  }

  // Apply a new resolution: rebake what changed, and send the cartridges where they now belong.
  function apply(next, animate) {
    const previousKeys = new Set(result.content.map((entry) => entry.key));
    result = next;
    for (const pane of Object.values(panes)) bakePane(pane);
    bakeDecor();
    sunriseTarget.copy(fallbackColor(result.fallback.value));
    const keys = new Set(result.content.map((entry) => entry.key));
    for (const [keyText, chip] of chips) {
      if (!keys.has(keyText)) {
        burst(chip.group.position, chip.hue, 22, 1);
        disposeChip(chip);
        chips.delete(keyText);
      }
    }
    result.content.forEach((entry, index) => {
      let chip = chips.get(entry.key);
      if (!chip) {
        chip = makeChip(entry);
        chips.set(entry.key, chip);
        const source = panes[entry.from.id];
        chip.position.copy(animate && !reduceMotion ? deckToTower(source.position, tmp) : chipSlot(index, tmp));
        chip.appear = animate && !reduceMotion ? 0 : 1;
      }
      chip.index = index;
      chipSlot(index, chip.target);
      bakeChip(chip);
      if (!previousKeys.has(entry.key)) chip.hop = 0;
    });
  }

  // The loader: a light that walks the chain in the order it loads.
  const loader = { active: false, time: 0, path: [] };
  function walk() {
    loader.active = true;
    loader.time = 0;
    loader.path = result.loaded.map((config) => config.id);
  }

  // Layout --------------------------------------------------------------------------------------------

  let width = 1;
  let height = 1;
  let scale = 1;
  let place = { x: 0, y: 0, height: 0, above: false };
  const quality = { level: 1, slow: 0 };
  const anchor = new THREE.Vector3();
  const raycaster = new THREE.Raycaster();
  const plane = new THREE.Plane(new THREE.Vector3(0, 0, 1), 0);
  const ndc = new THREE.Vector2();

  function placeStill() {
    const spot = placement(root);
    const stageWidth = spot.height * STAGE_ASPECT;
    Object.assign(still.style, {
      left: `${spot.x - stageWidth * (0.5 + STILL_PAD.left)}px`,
      top: `${spot.y - spot.height * (0.5 + STILL_PAD.top)}px`,
      width: `${stageWidth * (1 + STILL_PAD.left + STILL_PAD.right)}px`,
      height: `${spot.height * (1 + STILL_PAD.top + STILL_PAD.bottom)}px`,
    });
    root.classList.add('is-placed');
  }

  function layout() {
    const rect = root.getBoundingClientRect();
    width = Math.max(1, Math.round(rect.width));
    height = Math.max(1, Math.round(rect.height));
    const dpr = Math.min(window.devicePixelRatio || 1, small ? 1.5 : 1.75) * quality.level;
    renderer.setPixelRatio(dpr);
    renderer.setSize(width, height, false);
    const w = Math.max(1, Math.floor(width * dpr));
    const h = Math.max(1, Math.floor(height * dpr));
    sceneTarget.setSize(w, h);
    bloomTargets[0].setSize(Math.max(1, w >> 2), Math.max(1, h >> 2));
    bloomTargets[1].setSize(Math.max(1, w >> 2), Math.max(1, h >> 2));
    bloomTargets[2].setSize(Math.max(1, w >> 3), Math.max(1, h >> 3));
    bloomTargets[3].setSize(Math.max(1, w >> 3), Math.max(1, h >> 3));
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    camera.updateMatrixWorld();
    skyUniforms.uResolution.value.set(width, height);

    place = placement(root);
    placeStill();
    measureText();
    ndc.set(place.x / width * 2 - 1, -(place.y / height * 2 - 1));
    raycaster.setFromCamera(ndc, camera);
    raycaster.ray.intersectPlane(plane, anchor);
    const unitsPerPixel = 2 * camera.position.distanceTo(anchor) * Math.tan(THREE.MathUtils.degToRad(camera.fov / 2)) / height;
    scale = Math.max(0.05, place.height * unitsPerPixel / STAGE.height);
    stage.position.copy(anchor);
    stage.scale.setScalar(scale);
    moteUniforms.uPixel.value = height * dpr / (2 * Math.tan(THREE.MathUtils.degToRad(camera.fov / 2))) * scale;
    skyUniforms.uCenter.value.set(place.x / width, 1 - place.y / height);
    skyUniforms.uRadius.value = place.height / height * 0.62;
    compositeMaterial.uniforms.uCenter.value.copy(skyUniforms.uCenter.value);
  }

  // Pointer -------------------------------------------------------------------------------------------

  const hero = root.closest('.dw-hero') || root;
  const pointer = new THREE.Vector2(0, 0);
  let pointerActive = false;
  let lastPointer = 0;
  let presence = 0;
  const lampTarget = new THREE.Vector3();
  const lean = new THREE.Vector2();
  let press = null;
  let drag = null;

  function toNdc(event) {
    const rect = root.getBoundingClientRect();
    return new THREE.Vector2((event.clientX - rect.left) / rect.width * 2 - 1, -((event.clientY - rect.top) / rect.height * 2 - 1));
  }
  // Only a press clear of the hero's words and controls reaches the chain.
  const CONTROLS = 'a, button, input, select, textarea, summary, label, [contenteditable], .dw-command, .dw-strip';
  let textBoxes = [];
  function measureText() {
    const box = root.getBoundingClientRect();
    const text = hero.querySelector('.dw-hero__text') || hero.querySelector('.dw-shell');
    textBoxes = text ? textRects(text).map((rect) => ({ left: rect.left - box.left - 4, right: rect.right - box.left + 4, top: rect.top - box.top - 4, bottom: rect.bottom - box.top + 4 })) : [];
  }
  function clear(event) {
    if (event.target.closest && event.target.closest(CONTROLS)) return false;
    const box = root.getBoundingClientRect();
    const x = event.clientX - box.left;
    const y = event.clientY - box.top;
    return !textBoxes.some((rect) => x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom);
  }
  function pick(event) {
    raycaster.setFromCamera(toNdc(event), camera);
    const hits = raycaster.intersectObjects(Object.values(panes).map((pane) => pane.body), false);
    return hits.length ? panes[hits[0].object.userData.pane] : null;
  }

  function onMove(event) {
    pointer.copy(toNdc(event));
    pointerActive = true;
    lastPointer = performance.now();
    if (press && event.pointerId === press.id && !drag) {
      // Decided by when the move happened, not when it arrived: a busy frame can deliver a quick
      // swipe's first move after the hold has already timed out.
      const held = event.timeStamp - press.stamp >= press.hold;
      const moved = Math.hypot(event.clientX - press.x, event.clientY - press.y);
      if (!held && moved > press.slop) cancelPress();
      else if (held) beginDrag();
    }
    if (drag && event.pointerId === drag.id) moveDrag();
    wake();
  }
  function onDown(event) {
    if (event.button !== 0 || !clear(event)) return;
    pointer.copy(toNdc(event));
    pointerActive = true;
    lastPointer = performance.now();
    const pane = pick(event);
    if (!pane) return;
    if (event.pointerType === 'mouse') event.preventDefault();
    const kind = event.pointerType === 'mouse' ? 'mouse' : 'touch';
    press = { id: event.pointerId, pane, x: event.clientX, y: event.clientY, type: event.pointerType, start: performance.now(), stamp: event.timeStamp, hold: HOLD[kind], slop: SLOP[kind], ready: false, timer: 0 };
    press.timer = setTimeout(ready, press.hold);
    wake();
  }
  function cancelPress() {
    if (!press) return;
    clearTimeout(press.timer);
    press.pane.charge = 0;
    press = null;
  }
  // Held long enough: the pane lifts, and the next move carries it. The root and patches/ only
  // shake: the root always loads first, and patches/ goes wherever mods/ goes.
  function ready() {
    if (!press) return;
    press.ready = true;
    if (press.pane.config.fixed || press.pane.config.nested) {
      press.pane.shake = 1;
      cancelPress();
    }
    wake();
  }
  function beginDrag() {
    if (!press || press.pane.config.fixed || press.pane.config.nested) {
      cancelPress();
      return;
    }
    const { pane, id } = press;
    clearTimeout(press.timer);
    press = null;
    drag = { id, pane, slot: order.indexOf(pane.config.id) + 1, along: order.indexOf(pane.config.id) + 1 };
    try { hero.setPointerCapture(id); } catch { /* the pointer may already be gone */ }
    hero.classList.add('ocfg-dragging');
    const selection = window.getSelection && window.getSelection();
    if (selection) selection.removeAllRanges();
    burst(deckToTower(pane.position, new THREE.Vector3()), pane.hue, 10, 0.5);
    moveDrag();
    wake();
  }
  // The pointer, followed along the deck: its nearest point on the line of slots, in screen space.
  const screenSlots = [new THREE.Vector2(), new THREE.Vector2(), new THREE.Vector2(), new THREE.Vector2(), new THREE.Vector2()];
  function moveDrag() {
    if (!drag) return;
    for (let i = 1; i <= 4; i++) {
      slotPosition(i, tmp);
      deck.localToWorld(tmp);
      tmp.project(camera);
      screenSlots[i].set(tmp.x, tmp.y);
    }
    let best = { distance: Infinity, along: drag.along };
    for (let i = 1; i < 4; i++) {
      const a = screenSlots[i];
      const b = screenSlots[i + 1];
      const ab = new THREE.Vector2().subVectors(b, a);
      const length = Math.max(ab.lengthSq(), 1e-6);
      const t = THREE.MathUtils.clamp(new THREE.Vector2().subVectors(pointer, a).dot(ab) / length, -0.6, 1.6);
      const point = a.clone().addScaledVector(ab, THREE.MathUtils.clamp(t, 0, 1));
      const distance = point.distanceTo(pointer);
      if (distance < best.distance) best = { distance, along: THREE.MathUtils.clamp(i + t, 0.6, 4.4) };
    }
    drag.along = best.along;
    const slot = THREE.MathUtils.clamp(Math.round(best.along), 1, 4);
    if (slot !== drag.slot) {
      drag.slot = slot;
      const next = order.filter((id) => id !== drag.pane.config.id);
      next.splice(slot - 1, 0, drag.pane.config.id);
      order = next;
      apply(resolve(order), true);
    }
  }
  function onUp(event) {
    if (drag && event.pointerId === drag.id) {
      const dropped = drag.pane;
      drag = null;
      hero.classList.remove('ocfg-dragging');
      try { hero.releasePointerCapture(event.pointerId); } catch { /* already released */ }
      burst(deckToTower(dropped.position, new THREE.Vector3()), dropped.hue, 14, 0.6);
      walk();
      checkPayoff();
      wake();
      return;
    }
    if (press && event.pointerId === press.id) {
      // A click on a pane, not a hold: the loader walks the chain again.
      const click = event.timeStamp - press.stamp < press.hold;
      cancelPress();
      if (click) walk();
      wake();
    }
  }
  function onCancel(event) {
    if (press && event.pointerId === press.id) cancelPress();
    if (drag && event.pointerId === drag.id) onUp(event);
  }
  hero.addEventListener('pointermove', onMove, { passive: true });
  hero.addEventListener('pointerdown', onDown, { passive: false });
  hero.addEventListener('pointerup', onUp, { passive: true });
  hero.addEventListener('pointercancel', onCancel, { passive: true });
  hero.addEventListener('pointerleave', () => { pointerActive = false; }, { passive: true });
  // Once a pane is held, the page stops scrolling under the finger; before, it scrolls as ever.
  hero.addEventListener('touchmove', (event) => { if (drag) event.preventDefault(); }, { passive: false });
  hero.addEventListener('contextmenu', (event) => { if (press || drag) event.preventDefault(); });
  hero.addEventListener('dragstart', (event) => { if (press || drag) event.preventDefault(); });
  // A press on a pane is not the start of a text selection.
  hero.addEventListener('mousedown', (event) => { if (press || drag) event.preventDefault(); });
  document.addEventListener('selectstart', (event) => { if (press || drag) event.preventDefault(); });

  // The one order with a consequence of its own: purge/ last. Its replace=content discards every
  // plugin before it and leaves its own, Morrowind.esm and Fargoth.esp, and Fargoth gets his ring.
  function checkPayoff() {
    const fargoth = result.user.id === 'purge';
    if (fargoth && !ringState.active) {
      const chip = chips.get('purge:Fargoth.esp');
      if (!chip) return;
      ringState.active = true;
      ringState.fade = 1;
      ringState.settle = 0;
      ringState.tumble = 0;
      ringState.position.copy(chip.target).add(tmp.set(0.05, reduceMotion ? CHIP.height / 2 + 0.03 : 1.9, 0.04));
      ringState.velocity.set(0, 0, 0);
      ring.visible = true;
      skyUniforms.uFlash.value = 1;
    }
  }

  // The loop ------------------------------------------------------------------------------------------

  const clock = new THREE.Clock();
  let time = reduceMotion ? 3.4 : 0;
  let visible = false;
  let running = false;
  let first = true;
  let lost = false;
  let nextWalk = 2.0;
  let settling = 0;

  function frame() {
    running = false;
    if (lost) return;
    const rawDt = clock.getDelta();
    const dt = reduceMotion ? 1 : Math.min(rawDt, 0.05);
    if (!reduceMotion && rawDt < 0.5) {
      quality.slow = rawDt > 1 / 40 ? quality.slow + rawDt : Math.max(0, quality.slow - rawDt * 0.5);
      if (quality.slow > 1.5 && quality.level > 0.5) {
        quality.level = Math.max(0.5, quality.level - 0.2);
        quality.slow = 0;
        layout();
      }
    }
    if (!reduceMotion) time += dt;
    skyUniforms.uTime.value = time;
    moteUniforms.uTime.value = time;
    compositeMaterial.uniforms.uTime.value = time;
    skyUniforms.uFlash.value = damp(skyUniforms.uFlash.value, 0, 1.6, dt);
    sunrise.lerp(sunriseTarget, reduceMotion ? 1 : 1 - Math.exp(-dt * 2.2));
    skyUniforms.uSunrise.value.copy(sunrise);
    rim.color.copy(sunrise).lerp(new THREE.Color(1, 1, 1), 0.35);

    // The lamp and the lean: the pointer while it moves over the hero, a slow drift otherwise.
    const idle = !pointerActive || performance.now() - lastPointer > 4000;
    if (idle) {
      lampTarget.set(anchor.x + Math.sin(time * 0.35) * 1.4 * scale, anchor.y + Math.cos(time * 0.23) * 0.9 * scale, anchor.z + 3.0 * scale);
    } else {
      raycaster.setFromCamera(pointer, camera);
      plane.constant = -(anchor.z + 2.4 * scale);
      if (raycaster.ray.intersectPlane(plane, tmp)) lampTarget.copy(tmp);
      plane.constant = 0;
    }
    presence = damp(presence, idle ? 0.22 : 1, 3, dt);
    lamp.position.lerp(lampTarget, reduceMotion ? 1 : 1 - Math.exp(-dt * 6));
    lamp.intensity = presence * 2.4 * scale * scale;
    const dx = (lamp.position.x - anchor.x) / scale;
    const dy = (lamp.position.y - anchor.y) / scale;
    lean.x = damp(lean.x, THREE.MathUtils.clamp(-dy * 0.035, -0.08, 0.08), 2.5, dt);
    lean.y = damp(lean.y, THREE.MathUtils.clamp(dx * 0.04, -0.1, 0.1), 2.5, dt);
    deck.rotation.set(lean.x + Math.sin(time * 0.31) * 0.012, -0.3 + lean.y + Math.sin(time * 0.19) * 0.03, 0);
    tower.rotation.set(lean.x * 0.5, 0.1 + lean.y * 0.5 + Math.sin(time * 0.23) * 0.015, 0);

    // The loader's walk: a hop per file, in load order.
    if (!reduceMotion && !drag && time > nextWalk && !loader.active) {
      walk();
      nextWalk = time + 9;
    }
    if (loader.active) {
      loader.time += dt;
      const hop = 0.34;
      const step = loader.time / hop;
      const index = Math.floor(step);
      if (index >= loader.path.length - 1) {
        loader.active = false;
        loaderBead.visible = false;
        loaderLight.intensity = 0;
        panes[loader.path[loader.path.length - 1]].read = 1;
        nextWalk = Math.max(nextWalk, time + 9);
      } else {
        const from = panes[loader.path[index]];
        const to = panes[loader.path[index + 1]];
        from.read = 1;
        const t = THREE.MathUtils.smootherstep(step - index, 0, 1);
        tmp.copy(from.position).lerp(to.position, t);
        tmp.x -= Math.sin(Math.PI * t) * 0.35;
        tmp.z += Math.sin(Math.PI * t) * 0.2;
        loaderBead.position.copy(tmp);
        loaderBead.visible = true;
        loaderLight.position.copy(tmp);
        deck.localToWorld(loaderLight.position);
        loaderLight.intensity = 1.1 * scale * scale;
        const link = links[to.config.id];
        if (link) link.boost = 1;
      }
    }

    // The panes: each springs to its slot; the held one follows the pointer, lifted clear.
    for (const pane of Object.values(panes)) {
      const id = pane.config.id;
      if (id === 'root') slotPosition(0, pane.target);
      else if (pane.config.nested) {
        const mods = panes.mods;
        pane.target.copy(mods.position).add(tmp2.set(1.18, -0.3, -0.14));
      } else if (drag && drag.pane === pane) {
        const along = drag.along;
        const a = Math.floor(THREE.MathUtils.clamp(along, 1, 3.999));
        slotPosition(a, pane.target);
        slotPosition(a + 1, tmp2);
        pane.target.lerp(tmp2, THREE.MathUtils.clamp(along - a, 0, 1));
        if (along < 1) slotPosition(1, pane.target).addScaledVector(tmp2.set(-0.3, 0.33, -0.4), 1 - along);
        if (along > 4) slotPosition(4, pane.target).addScaledVector(tmp2.set(0.3, -0.33, 0.4), along - 4);
      } else {
        slotPosition(order.indexOf(id) + 1, pane.target);
      }
      if (reduceMotion) {
        pane.position.copy(pane.target);
        pane.velocity.set(0, 0, 0);
      } else {
        const stiffness = 90;
        const dampingRatio = 2 * Math.sqrt(stiffness) * 0.9;
        tmp.subVectors(pane.target, pane.position).multiplyScalar(stiffness).addScaledVector(pane.velocity, -dampingRatio);
        pane.velocity.addScaledVector(tmp, dt);
        pane.position.addScaledVector(pane.velocity, dt);
      }
      pane.group.position.copy(pane.position);
      const held = drag && drag.pane === pane || press && press.pane === pane && press.ready;
      pane.liftAmount = damp(pane.liftAmount, held ? 1 : 0, 10, dt);
      if (press && press.pane === pane) pane.charge = press.ready ? 1 : Math.min(1, (performance.now() - press.start) / press.hold);
      else if (!held) pane.charge = damp(pane.charge, 0, 6, dt);
      pane.shake = damp(pane.shake, 0, 3.5, dt);
      pane.lift.position.set(Math.sin(time * 38) * pane.shake * 0.03, pane.liftAmount * 0.05, pane.liftAmount * 0.55 + pane.charge * 0.05);
      pane.lift.rotation.set(-pane.liftAmount * 0.08, pane.liftAmount * 0.12, Math.sin(time * 41) * pane.shake * 0.04 + pane.liftAmount * -0.03);
      pane.read = damp(pane.read, 0, 1.4, dt);
      pane.edge.emissiveIntensity = 0.45 + pane.read * 3.2 + pane.charge * 1.6 + pane.liftAmount * 1.2;
      pane.textMaterial.color.setScalar(1.1 + pane.read * 1.1 + pane.liftAmount * 0.3);
      pane.glass.opacity = 0.84 + pane.liftAmount * 0.08;
      const depthOrder = pane.position.z + pane.liftAmount;
      pane.body.renderOrder = Math.round(depthOrder * 100);
      pane.text.renderOrder = Math.round(depthOrder * 100) + 1;
    }

    // The links: from the root's left edge to the named file's, bowing out to the left.
    const rootPane = panes.root;
    for (const [id, link] of Object.entries(links)) {
      const fromPane = id === 'patches' ? panes.mods : rootPane;
      const toPane = panes[id];
      const fromScale = fromPane.config.nested ? 0.5 : 1;
      const toScale = toPane.config.nested ? 0.5 : 1;
      if (id === 'patches') {
        curve.v0.copy(fromPane.position).add(tmp.set(PANE.width / 2 * fromScale, -0.1, 0));
        curve.v3.copy(toPane.position).add(tmp.set(-PANE.width / 2 * toScale, 0, 0));
        curve.v1.copy(curve.v0).add(tmp.set(0.18, -0.02, 0));
        curve.v2.copy(curve.v3).add(tmp.set(-0.18, 0.02, 0));
      } else {
        curve.v0.copy(fromPane.position).add(tmp.set(-PANE.width / 2 * fromScale, -0.22, 0.02));
        curve.v3.copy(toPane.position).add(tmp.set(-PANE.width / 2 * toScale, 0.12, 0.02));
        const bow = 0.16 + 0.07 * (order.indexOf(id) + 1);
        curve.v1.copy(curve.v0).add(tmp.set(-bow, -0.05, 0.1));
        curve.v2.copy(curve.v3).add(tmp.set(-bow, 0.1, -0.05));
      }
      const positions = link.geometry.attributes.position.array;
      for (let i = 0; i < LINK_POINTS; i++) {
        curve.getPoint(i / (LINK_POINTS - 1), linkPoint);
        positions[i * 3] = linkPoint.x;
        positions[i * 3 + 1] = linkPoint.y;
        positions[i * 3 + 2] = linkPoint.z;
      }
      link.geometry.attributes.position.needsUpdate = true;
      link.boost = damp(link.boost, 0, 2.5, dt);
      link.material.uniforms.uBoost.value = link.boost;
      link.material.uniforms.uTime.value = time;
    }

    // The cartridges: new ones fly from their file to their place in the load order.
    let moving = false;
    for (const chip of chips.values()) {
      if (reduceMotion) {
        chip.position.copy(chip.target);
        chip.appear = 1;
      } else {
        const stiffness = 60;
        tmp.subVectors(chip.target, chip.position).multiplyScalar(stiffness).addScaledVector(chip.velocity, -2 * Math.sqrt(stiffness) * 0.85);
        chip.velocity.addScaledVector(tmp, dt);
        chip.position.addScaledVector(chip.velocity, dt);
        chip.appear = Math.min(1, chip.appear + dt * 2.2);
        if (chip.velocity.lengthSq() > 1e-4) moving = true;
      }
      chip.hop = Math.min(1, chip.hop + dt * 1.5);
      const arc = Math.sin(Math.PI * Math.min(1, chip.appear)) * 0.25;
      chip.group.position.copy(chip.position).add(tmp.set(0, 0, arc));
      chip.group.scale.setScalar(0.35 + 0.65 * THREE.MathUtils.smootherstep(chip.appear, 0, 1));
      chip.group.rotation.set(0, Math.sin(time * 0.6 + chip.index) * 0.06 + (1 - chip.appear) * 1.6, 0);
    }

    // Fargoth's ring falls on his cartridge, bounces, spins like a coin and settles.
    const fargoth = chips.get('purge:Fargoth.esp');
    if (ringState.active) {
      if (result.user.id !== 'purge' || !fargoth) {
        ringState.fade = damp(ringState.fade, 0, 4, dt);
        if (ringState.fade < 0.02) {
          ringState.active = false;
          ring.visible = false;
        }
      } else {
        // It comes to rest hovering in front of Fargoth's label, as an item does on display.
        const restX = fargoth.group.position.x + CHIP.width * 0.33;
        const restZ = fargoth.group.position.z + CHIP.depth / 2 + 0.2;
        const rest = fargoth.group.position.y;
        if (reduceMotion) {
          ringState.position.set(restX, rest, restZ);
          ringState.velocity.set(0, 0, 0);
          ringState.settle = 1;
        } else {
          ringState.velocity.y -= 5.5 * dt;
          ringState.position.addScaledVector(ringState.velocity, dt);
          ringState.position.x = damp(ringState.position.x, restX, 3, dt);
          ringState.position.z = damp(ringState.position.z, restZ, 3, dt);
          if (ringState.position.y < rest) {
            ringState.position.y = rest;
            if (Math.abs(ringState.velocity.y) > 0.25) {
              burst(ringState.position, new THREE.Color(1.0, 0.8, 0.35), Math.round(Math.abs(ringState.velocity.y) * 6), 0.45);
              fargoth.hop = 0;
              skyUniforms.uFlash.value = Math.max(skyUniforms.uFlash.value, 0.4);
            }
            ringState.velocity.y = -ringState.velocity.y * 0.42;
            if (Math.abs(ringState.velocity.y) < 0.2) ringState.velocity.y = 0;
          }
          if (ringState.velocity.y === 0 && ringState.position.y <= rest + 1e-3) ringState.settle = Math.min(1, ringState.settle + dt * 0.8);
          ringState.tumble += dt * (1.3 + (1 - ringState.settle) * 7);
        }
        ring.position.copy(ringState.position);
        ring.position.y += ringState.settle * Math.sin(time * 1.4) * 0.02;
        ring.rotation.set(0.28 * ringState.settle + (1 - ringState.settle) * Math.sin(ringState.tumble * 0.7) * 1.1, ringState.tumble, 0);
      }
      ring.scale.setScalar(Math.max(ringState.fade, 0.001));
      if (fargoth) {
        shaft.visible = true;
        shaft.position.set(fargoth.group.position.x, fargoth.group.position.y + 1.45, fargoth.group.position.z);
        shaft.material.uniforms.uOpacity.value = ringState.fade * (0.55 + 0.25 * Math.sin(time * 0.9));
        shaft.material.uniforms.uTime.value = time;
      }
      // Once it has settled, the ring glints now and then, and Fargoth hops for joy.
      if (ringState.settle >= 1 && !reduceMotion) {
        ringState.glint += dt;
        if (ringState.glint > 3.2) {
          ringState.glint = 0;
          burst(tmp.copy(ring.position).add(tmp2.set(0.12, 0.05, 0.05)), new THREE.Color(1, 0.95, 0.7), 5, 0.25);
          if (fargoth) fargoth.hop = 0;
        }
      }
    } else {
      shaft.visible = false;
    }
    if (fargoth) {
      const hop = Math.sin(Math.PI * Math.min(1, fargoth.hop)) * (1 - fargoth.hop) * 0.12;
      fargoth.group.position.y += hop;
    }

    // The quill rides the last directory in the chain.
    const userPane = panes[result.user.id];
    const userScale = userPane.config.nested ? 0.5 : 1;
    tmp.copy(userPane.position).add(tmp2.set(PANE.width * 0.32 * userScale, PANE.height * 0.5 * userScale + 0.1, 0.06 + userPane.liftAmount * 0.55));
    if (reduceMotion || quillPosition.x > 8) quillPosition.copy(tmp);
    else quillPosition.lerp(tmp, 1 - Math.exp(-dt * 5));
    quill.position.copy(quillPosition);
    quill.position.y += Math.sin(time * 1.3) * 0.015;
    quill.quaternion.copy(deck.getWorldQuaternion(quillQuaternion)).invert().multiply(camera.quaternion);

    // Shards fall and fade.
    let shardAlive = false;
    for (let i = 0; i < SHARDS; i++) {
      const shard = shards[i];
      if (shard.life > 0) {
        shardAlive = true;
        shard.life -= dt * 0.8;
        shard.velocity.y -= 2.4 * dt;
        shard.velocity.multiplyScalar(Math.exp(-dt * 0.6));
        shard.position.addScaledVector(shard.velocity, dt);
        shard.rotation.x += shard.spin.x * dt;
        shard.rotation.y += shard.spin.y * dt;
        shard.rotation.z += shard.spin.z * dt;
      }
      const size = Math.max(0, shard.life);
      shardDummy.position.copy(shard.position);
      shardDummy.rotation.copy(shard.rotation);
      shardDummy.scale.setScalar(size > 0 ? size : 1e-4);
      shardDummy.updateMatrix();
      shardMesh.setMatrixAt(i, shardDummy.matrix);
      shardMesh.setColorAt(i, shard.color);
    }
    shardMesh.instanceMatrix.needsUpdate = true;
    if (shardMesh.instanceColor) shardMesh.instanceColor.needsUpdate = true;

    scene.updateMatrixWorld();
    renderer.setRenderTarget(sceneTarget);
    renderer.clear();
    renderer.render(scene, camera);
    pass(brightMaterial, bloomTargets[0]);
    blur(bloomTargets[0], bloomTargets[1], 1.0);
    blur(bloomTargets[0], bloomTargets[1], 2.0);
    copyMaterial.uniforms.tInput.value = bloomTargets[0].texture;
    pass(copyMaterial, bloomTargets[2]);
    blur(bloomTargets[2], bloomTargets[3], 1.5);
    blur(bloomTargets[2], bloomTargets[3], 3.0);
    pass(compositeMaterial, null);

    if (first) {
      first = false;
      root.classList.add('is-live');
    }
    const busy = drag || press || moving || shardAlive || loader.active || ringState.active && ringState.settle < 1;
    if (reduceMotion) settling = busy ? 2 : settling - 1;
    if (visible && !document.hidden && (!reduceMotion || busy || settling > 0)) requestFrame();
  }

  function requestFrame() {
    if (running || lost) return;
    running = true;
    requestAnimationFrame(frame);
  }
  function wake() {
    if (visible && !document.hidden) requestFrame();
  }

  canvas.addEventListener('webglcontextlost', (event) => {
    event.preventDefault();
    lost = true;
    root.classList.remove('is-live');
  });
  canvas.addEventListener('webglcontextrestored', () => {
    canvas.remove();
    still.remove();
    root.classList.remove('is-live', 'is-placed');
    mount(root);
  });

  apply(result, false);
  layout();
  new ResizeObserver(() => {
    layout();
    wake();
  }).observe(root);
  if (document.fonts) {
    document.fonts.ready.then(() => {
      for (const pane of Object.values(panes)) pane.bakedKey = '';
      for (const chip of chips.values()) chip.bakedKey = '';
      bakedDecor = '';
      apply(result, false);
      layout();
      wake();
    });
  }
  new IntersectionObserver((entries) => {
    visible = entries.some((entry) => entry.isIntersecting);
    if (visible) {
      clock.getDelta();
      wake();
    }
  }).observe(root);
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden && visible) {
      clock.getDelta();
      wake();
    }
  });
}

for (const root of document.querySelectorAll('[data-dw-hero-art]')) mount(root);
