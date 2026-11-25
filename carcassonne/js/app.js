import initWasm, { CarcassonneEngineWasm } from '../pkg/carcassonne_engine.js?v=2.5.0';
import { playSound, isSoundEnabled, setSoundEnabled } from './audio.js';
import {
  initFirebase,
  getSavedFirebaseConfig,
  saveFirebaseConfig,
  createRoom,
  joinRoom,
  listenToRoom,
  broadcastState,
  leaveRoom,
  isInOnlineRoom,
  getCurrentRoomId,
  checkIsHost
} from './firebase-sync.js';

const TILE_SIZE = 100;
const DRAG_THRESHOLD = 8;
const DIRS = [
  { dx: 0, dy: -1 }, // 0: North
  { dx: 1, dy: 0 },  // 1: East
  { dx: 0, dy: 1 },  // 2: South
  { dx: -1, dy: 0 }  // 3: West
];

const DEFAULT_PLAYERS = [
  { id: 'p_0', name: 'Player 1', color_key: 'red', hex: '#dc2626', score: 0, meeples: 7 },
  { id: 'p_1', name: 'Player 2', color_key: 'blue', hex: '#2563eb', score: 0, meeples: 7 }
];

const PLAYER_COLORS = [
  { key: 'red', hex: '#dc2626', label: 'Crimson' },
  { key: 'blue', hex: '#2563eb', label: 'Royal Blue' },
  { key: 'green', hex: '#16a34a', label: 'Emerald' },
  { key: 'yellow', hex: '#ca8a04', label: 'Gold' },
  { key: 'black', hex: '#374151', label: 'Iron' },
  { key: 'pink', hex: '#ec4899', label: 'Pink' }
];

let tileCatalog = {};
let tileCatalogList = [];
let wasmEngine = null;
let currentGameState = null;

const uiState = {
  panX: 0,
  panY: 0,
  zoom: 1,
  pendingPlacement: null, // null | { x, y }
  pendingMeepleFeatureId: null, // null | string
  pendingMeepleKind: 'normal', // 'normal' | 'big' | 'builder' | 'pig'
  currentRotation: 0,
  isDraggingBoard: false,
  boardPointers: new Map(),
  dragStartX: 0,
  dragStartY: 0,
  panOriginX: 0,
  panOriginY: 0,
  setupPlayerCount: 2,
  tileDrag: {
    active: false,
    pointerId: null,
    source: null, // 'hand' | 'pending'
    startX: 0,
    startY: 0,
    moved: false,
    snappedSlot: null, // null | { x, y }
    justDragged: false
  },
  meepleDrag: {
    active: false,
    pointerId: null,
    source: null, // 'hand' | 'board'
    kind: 'normal', // 'normal' | 'big' | 'builder' | 'pig'
    startX: 0,
    startY: 0,
    moved: false,
    snappedSpot: null // null | spot
  }
};

const boardViewport = document.getElementById('board-viewport');
const boardStage = document.getElementById('board-stage');

function getBoardEntries(state) {
  if (!state || !state.board) return [];
  if (state.board instanceof Map) {
    return Array.from(state.board.entries());
  }
  return Object.entries(state.board);
}

function getBoardTile(state, key) {
  if (!state || !state.board) return null;
  if (state.board instanceof Map) {
    return state.board.get(key) || null;
  }
  return state.board[key] || null;
}

const meepleSvgs = {
  normal: '',
  big: '',
  builder: '',
  pig: ''
};

function sanitizeSvgTemplate(raw) {
  return raw ? raw.replace(/style="color:[^"]*"/, '') : '';
}

function renderMeepleSvg(colorHex = '#dc2626', extraClasses = '', isFarmer = false, kind = 'normal') {
  const raw = meepleSvgs[kind] || meepleSvgs.normal;
  if (!raw) {
    return `<div class="${extraClasses}" style="background-color:${colorHex}; width:100%; height:100%; border-radius:50%;"></div>`;
  }
  const shouldRotate = isFarmer && (kind === 'normal' || kind === 'big');
  const farmerTransform = shouldRotate ? 'transform: rotate(85deg) scale(0.92);' : '';
  return raw.replace(
    '<svg ',
    `<svg class="${extraClasses}" style="color: ${colorHex}; ${farmerTransform}" `
  );
}

function screenToWorld(clientX, clientY) {
  const rect = boardViewport.getBoundingClientRect();
  return {
    x: (clientX - rect.left - uiState.panX) / uiState.zoom,
    y: (clientY - rect.top - uiState.panY) / uiState.zoom
  };
}

function getGridCellFromPointer(clientX, clientY) {
  const world = screenToWorld(clientX, clientY);
  return {
    x: Math.floor(world.x / TILE_SIZE),
    y: Math.floor(world.y / TILE_SIZE)
  };
}

function updateDragGhostPosition(clientX, clientY) {
  const dragGhost = document.getElementById('drag-ghost');
  if (!dragGhost) return;
  const ghostSize = Math.max(76, Math.min(130, TILE_SIZE * uiState.zoom));
  dragGhost.style.width = `${ghostSize}px`;
  dragGhost.style.height = `${ghostSize}px`;
  dragGhost.style.transform = `translate(${clientX - ghostSize / 2}px, ${clientY - ghostSize / 2}px)`;
}

function startTilePointerInteraction(e, source) {
  if (!currentGameState || currentGameState.phase !== 'PLACE_TILE' || !currentGameState.current_tile_type_id) return;
  if (e.button !== undefined && e.button !== 0) return;
  e.stopPropagation();
  e.preventDefault();

  uiState.tileDrag = {
    active: true,
    pointerId: e.pointerId,
    source,
    startX: e.clientX,
    startY: e.clientY,
    moved: false,
    snappedSlot: null,
    justDragged: false
  };
}

function updateMeepleDragGhostPosition(clientX, clientY) {
  const dragGhost = document.getElementById('drag-ghost');
  if (!dragGhost) return;
  const ghostSize = Math.max(28, Math.min(44, 34 * uiState.zoom));
  dragGhost.style.width = `${ghostSize}px`;
  dragGhost.style.height = `${ghostSize}px`;
  dragGhost.style.transform = `translate(${clientX - ghostSize / 2}px, ${clientY - ghostSize / 2}px)`;
}

function findNearestMeepleSpot(clientX, clientY, kind = 'normal') {
  if (!currentGameState || currentGameState.phase !== 'PLACE_MEEPLE' || !currentGameState.last_placed_coord) {
    return null;
  }
  const [lx, ly] = currentGameState.last_placed_coord.split(',').map(Number);
  const allSpots = wasmEngine.get_valid_meeple_spots(lx, ly) || [];
  const spots = allSpots.filter((s) => !s.allowed_kinds || s.allowed_kinds.includes(kind));
  if (spots.length === 0) return null;

  const world = screenToWorld(clientX, clientY);
  const tileMinX = lx * TILE_SIZE;
  const tileMinY = ly * TILE_SIZE;

  // Must be within or near the just-placed tile to snap
  const margin = 25;
  if (
    world.x < tileMinX - margin ||
    world.x > tileMinX + TILE_SIZE + margin ||
    world.y < tileMinY - margin ||
    world.y > tileMinY + TILE_SIZE + margin
  ) {
    return null;
  }

  let bestSpot = null;
  let bestDist = Infinity;
  for (const spot of spots) {
    const sx = tileMinX + (spot.hotspot.x / 100) * TILE_SIZE;
    const sy = tileMinY + (spot.hotspot.y / 100) * TILE_SIZE;
    const dist = Math.hypot(world.x - sx, world.y - sy);
    if (dist < bestDist) {
      bestDist = dist;
      bestSpot = spot;
    }
  }

  return bestDist <= 65 ? bestSpot : null;
}

function startMeeplePointerInteraction(e, source, kind = 'normal') {
  if (!currentGameState || currentGameState.phase !== 'PLACE_MEEPLE') return;
  if (e.button !== undefined && e.button !== 0) return;
  e.stopPropagation();
  e.preventDefault();

  uiState.meepleDrag = {
    active: true,
    pointerId: e.pointerId,
    source,
    kind,
    startX: e.clientX,
    startY: e.clientY,
    moved: false,
    snappedSpot: null
  };
}

async function resolveTileAssets(catalogList) {
  const extensions = ['.png', '.webp', '.jpg'];
  await Promise.all(
    catalogList.map(async (tile) => {
      if (!tile.asset) return;
      const dotIndex = tile.asset.lastIndexOf('.');
      if (dotIndex === -1) return;
      const basePath = tile.asset.substring(0, dotIndex);
      const currentExt = tile.asset.substring(dotIndex).toLowerCase();

      // If current asset is .svg, check if a replacement raster exists (PNG, WebP, JPG)
      if (currentExt === '.svg') {
        for (const ext of extensions) {
          try {
            const res = await fetch(`${basePath}${ext}`, { method: 'HEAD' });
            if (res.ok && res.status === 200) {
              tile.asset = `${basePath}${ext}`;
              break;
            }
          } catch (_) {}
        }
      } else {
        // If current asset is raster (e.g. .png), verify it exists, otherwise fall back to .svg
        try {
          const res = await fetch(tile.asset, { method: 'HEAD' });
          if (!res.ok || res.status !== 200) {
            tile.asset = `${basePath}.svg`;
          }
        } catch (_) {
          tile.asset = `${basePath}.svg`;
        }
      }
    })
  );
}

async function fetchOptionalJson(url) {
  try {
    const res = await fetch(url);
    if (res.ok) return await res.json();
  } catch (_) {}
  return [];
}

async function fetchOptionalText(url) {
  try {
    const res = await fetch(url);
    if (res.ok) return await res.text();
  } catch (_) {}
  return '';
}

async function main() {
  await initWasm({ module_or_path: new URL('../pkg/carcassonne_engine_bg.wasm?v=2.5.0', import.meta.url) });

  // Load base + expansion tile catalogs and meeple SVG asset templates
  const [bagTiles, icTiles, tbTiles, meepleRaw, bigRaw, builderRaw, pigRaw] = await Promise.all([
    fetch('assets/tile_bag.json?v=2.5.0').then((r) => r.json()),
    fetchOptionalJson('assets/tiles_ic.json?v=2.5.0'),
    fetchOptionalJson('assets/tiles_tb.json?v=2.5.0'),
    fetchOptionalText('assets/ui/meeple.svg?v=2.5.0'),
    fetchOptionalText('assets/ui/meeple_big.svg?v=2.5.0'),
    fetchOptionalText('assets/ui/builder.svg?v=2.5.0'),
    fetchOptionalText('assets/ui/pig.svg?v=2.5.0')
  ]);
  tileCatalogList = [...bagTiles, ...icTiles, ...tbTiles];
  await resolveTileAssets(tileCatalogList);
  tileCatalog = {};
  for (const t of tileCatalogList) {
    tileCatalog[t.id] = t;
  }

  meepleSvgs.normal = sanitizeSvgTemplate(meepleRaw);
  meepleSvgs.big = sanitizeSvgTemplate(bigRaw) || meepleSvgs.normal;
  meepleSvgs.builder = sanitizeSvgTemplate(builderRaw) || meepleSvgs.normal;
  meepleSvgs.pig = sanitizeSvgTemplate(pigRaw) || meepleSvgs.normal;

  // Check saved Firebase settings, falling back to default Arcade Firebase project
  const DEFAULT_FIREBASE_CONFIG = {
    apiKey: 'AIzaSyB5QzoOs7MIpNC7GEPLVuu346P8d_23HcE',
    authDomain: 'a-little-wordy-7f579.firebaseapp.com',
    databaseURL: 'https://a-little-wordy-7f579-default-rtdb.firebaseio.com',
    projectId: 'a-little-wordy-7f579',
    storageBucket: 'a-little-wordy-7f579.firebasestorage.app',
    messagingSenderId: '195141822673',
    appId: '1:195141822673:web:d2051810f6865d0f513df0'
  };
  const fbConfig = getSavedFirebaseConfig() || DEFAULT_FIREBASE_CONFIG;
  if (fbConfig) {
    initFirebase(fbConfig);
    const fbInput = document.getElementById('firebase-config-input');
    if (fbInput) fbInput.value = JSON.stringify(fbConfig, null, 2);
  }

  setupEventListeners();
  initSetupModal();

  // Check for ?room=XYZ in URL
  const urlParams = new URLSearchParams(window.location.search);
  const roomParam = urlParams.get('room');
  if (roomParam) {
    promptJoinRoomFromUrl(roomParam.toUpperCase());
  } else {
    // Start local campaign with all enabled expansions by default
    startNewLocalGame(DEFAULT_PLAYERS, true, true, true, true);
  }
}

function startNewLocalGame(
  players,
  farmersEnabled,
  riverEnabled = true,
  innsCathedralsEnabled = true,
  tradersBuildersEnabled = true
) {
  leaveRoom();
  updateMultiplayerButtonLabel();

  wasmEngine = new CarcassonneEngineWasm(
    JSON.stringify(tileCatalogList),
    JSON.stringify(players),
    farmersEnabled,
    riverEnabled,
    innsCathedralsEnabled,
    tradersBuildersEnabled
  );

  uiState.pendingPlacement = null;
  uiState.pendingMeepleFeatureId = null;
  uiState.pendingMeepleKind = 'normal';
  uiState.currentRotation = 0;

  currentGameState = wasmEngine.get_state();
  recenterBoard();
  renderAll();

  if (currentGameState.current_tile_type_id) {
    animateDrawFromBag(currentGameState.current_tile_type_id);
  }
}

function renderAll() {
  if (!currentGameState) return;

  renderBoard();
  renderActivePlayerHUD();
  renderHandSlot();
  renderMenuDropdown();
  renderTileBagBadge();
}

function renderBoard() {
  boardStage.innerHTML = '';

  const isDragging = uiState.tileDrag.active && uiState.tileDrag.moved;
  const snap = uiState.tileDrag.snappedSlot;

  // 1. Placed Tiles
  const boardEntries = getBoardEntries(currentGameState);
  for (const [key, tile] of boardEntries) {
    const def = tileCatalog[tile.type_id];
    if (!def) continue;

    const el = document.createElement('div');
    el.className = 'board-tile';
    if (currentGameState.last_placed_coord === key) {
      el.classList.add('just-confirmed');
    }
    el.style.left = `${tile.x * TILE_SIZE}px`;
    el.style.top = `${tile.y * TILE_SIZE}px`;

    const img = document.createElement('img');
    img.src = def.asset;
    img.draggable = false;
    img.className = 'w-full h-full block rounded-[5px] pointer-events-none select-none object-cover';
    img.style.transform = `rotate(${tile.rotation * 90}deg)`;
    img.alt = def.name;
    img.onerror = () => {
      if (def.asset.endsWith('.svg')) img.src = def.asset.replace(/\.svg$/, '.png');
      else if (def.asset.endsWith('.png')) img.src = def.asset.replace(/\.png$/, '.svg');
    };
    el.appendChild(img);

    // If tile has a meeple
    if (tile.meeple) {
      const player = currentGameState.players.find(p => p.id === tile.meeple.player_id);
      const feat = def.features.find(f => f.id === tile.meeple.feature_id);
      const field = def.fields.find(f => f.id === tile.meeple.feature_id);
      const hotspot = feat ? feat.hotspot : (field ? field.hotspot : { x: 50, y: 50 });

      // Rotate hotspot
      const rot = tile.rotation % 4;
      let rx = hotspot.x, ry = hotspot.y;
      if (rot === 1) { rx = 100 - hotspot.y; ry = hotspot.x; }
      else if (rot === 2) { rx = 100 - hotspot.x; ry = 100 - hotspot.y; }
      else if (rot === 3) { rx = hotspot.y; ry = 100 - hotspot.x; }

      const kind = tile.meeple.kind || 'normal';
      const isFarmer = !!(tile.meeple.is_farmer || field);
      const meepleEl = document.createElement('div');
      meepleEl.className = `placed-meeple meeple-kind-${kind}`;
      if (isFarmer && (kind === 'normal' || kind === 'big')) {
        meepleEl.classList.add('meeple-farmer');
      }
      meepleEl.style.left = `${rx}%`;
      meepleEl.style.top = `${ry}%`;
      const playerColor = player ? player.hex : '#dc2626';
      meepleEl.innerHTML = renderMeepleSvg(playerColor, 'w-full h-full pointer-events-none select-none', isFarmer, kind);
      el.appendChild(meepleEl);
    }

    boardStage.appendChild(el);
  }

  // 2. Drag Hover Indicator (only shown while actively dragging a tile over an empty cell)
  if (isDragging && snap && currentGameState.phase === 'PLACE_TILE') {
    if (!getBoardTile(currentGameState, `${snap.x},${snap.y}`)) {
      const hoverEl = document.createElement('div');
      hoverEl.className = 'drag-hover-cell';
      hoverEl.style.left = `${snap.x * TILE_SIZE}px`;
      hoverEl.style.top = `${snap.y * TILE_SIZE}px`;
      boardStage.appendChild(hoverEl);
    }
  }

  // 3. Staged Pending Tile (can be placed anywhere on an empty grid cell)
  if (
    uiState.pendingPlacement &&
    currentGameState.phase === 'PLACE_TILE' &&
    !(isDragging && uiState.tileDrag.source === 'pending')
  ) {
    const { x, y } = uiState.pendingPlacement;
    const def = tileCatalog[currentGameState.current_tile_type_id];
    if (def) {
      const pendingEl = document.createElement('div');
      pendingEl.className = 'pending-tile';
      pendingEl.style.left = `${x * TILE_SIZE}px`;
      pendingEl.style.top = `${y * TILE_SIZE}px`;
      pendingEl.title = 'Drag to reposition • Tap to rotate';

      const img = document.createElement('img');
      img.src = def.asset;
      img.draggable = false;
      img.className = 'w-full h-full block rounded-[6px] pointer-events-none select-none object-cover';
      img.style.transform = `rotate(${uiState.currentRotation * 90}deg)`;
      img.onerror = () => {
        if (def.asset.endsWith('.svg')) img.src = def.asset.replace(/\.svg$/, '.png');
        else if (def.asset.endsWith('.png')) img.src = def.asset.replace(/\.png$/, '.svg');
      };
      pendingEl.appendChild(img);

      pendingEl.addEventListener('pointerdown', (e) => startTilePointerInteraction(e, 'pending'));

      boardStage.appendChild(pendingEl);
    }
  }

  // 4. Staged Meeple (if in PLACE_MEEPLE phase and snapped to a feature)
  if (
    currentGameState.phase === 'PLACE_MEEPLE' &&
    currentGameState.last_placed_coord &&
    uiState.pendingMeepleFeatureId
  ) {
    const [lx, ly] = currentGameState.last_placed_coord.split(',').map(Number);
    const spots = wasmEngine.get_valid_meeple_spots(lx, ly) || [];
    const spot = spots.find((s) => s.id === uiState.pendingMeepleFeatureId);
    if (spot) {
      const activePlayer = currentGameState.players[currentGameState.current_player_index];
      const color = activePlayer ? activePlayer.hex : '#dc2626';
      const kind = uiState.pendingMeepleKind || 'normal';
      const shouldRotate = spot.is_farmer && (kind === 'normal' || kind === 'big');
      const stagedEl = document.createElement('div');
      stagedEl.className = `staged-meeple meeple-kind-${kind} ${shouldRotate ? 'meeple-farmer' : ''}`;
      stagedEl.innerHTML = renderMeepleSvg(color, 'w-full h-full pointer-events-none select-none', spot.is_farmer, kind);
      stagedEl.style.left = `${lx * TILE_SIZE + (spot.hotspot.x / 100) * TILE_SIZE}px`;
      stagedEl.style.top = `${ly * TILE_SIZE + (spot.hotspot.y / 100) * TILE_SIZE}px`;
      stagedEl.title = 'Drag to reposition or remove';
      stagedEl.addEventListener('pointerdown', (e) => startMeeplePointerInteraction(e, 'board', kind));
      boardStage.appendChild(stagedEl);
    }
  }
}

function stagePendingTile(x, y) {
  if (getBoardTile(currentGameState, `${x},${y}`)) {
    showToast('This position is already occupied', 'warn');
    return;
  }
  uiState.pendingPlacement = { x, y };
  playSound('stage');
  renderAll();
}

function rotateTile(direction) {
  playSound('rotate');
  if (direction === 'cw') {
    uiState.currentRotation = (uiState.currentRotation + 1) % 4;
  } else {
    uiState.currentRotation = (uiState.currentRotation + 3) % 4;
  }

  // Keep pending tile on board so player can test if rotation makes it valid
  renderAll();
}

function confirmTurnAction() {
  if (!currentGameState) return;

  // In online room, check if it is our turn
  if (isInOnlineRoom()) {
    const currentPlayer = currentGameState.players[currentGameState.current_player_index];
    const myId = (window.myCurrentPlayerId || currentPlayer.id);
    if (myId !== currentPlayer.id) {
      showToast(`It is ${currentPlayer.name}'s turn!`, 'warn');
      return;
    }
  }

  if (currentGameState.phase === 'PLACE_TILE') {
    if (!uiState.pendingPlacement) {
      showToast('Place the tile on the board first', 'warn');
      return;
    }
    const { x, y } = uiState.pendingPlacement;
    if (!wasmEngine.can_place_tile(x, y, uiState.currentRotation)) {
      showToast('Cannot confirm: tile placement is invalid here (must connect to an existing tile with matching edges)', 'warn');
      return;
    }
    try {
      const result = wasmEngine.place_tile(x, y, uiState.currentRotation);
      currentGameState = result.state;
      uiState.pendingPlacement = null;
      uiState.pendingMeepleFeatureId = null;
      uiState.pendingMeepleKind = 'normal';
      playSound('confirm');

      if (!result.needs_meeple) {
        if (currentGameState.is_builder_bonus_turn) {
          const activePlayer = currentGameState.players[currentGameState.current_player_index];
          if (activePlayer) {
            showToast(`${activePlayer.name} gets a Builder Bonus Turn!`, 'info');
          }
        }
        if (currentGameState.current_tile_type_id) {
          animateDrawFromBag(currentGameState.current_tile_type_id);
        }
      }

      broadcastIfOnline();
      renderAll();
    } catch (err) {
      showToast(err.toString(), 'warn');
    }
  } else if (currentGameState.phase === 'PLACE_MEEPLE') {
    try {
      const featureId = uiState.pendingMeepleFeatureId || null;
      const meepleKind = featureId ? (uiState.pendingMeepleKind || 'normal') : null;
      const result = wasmEngine.place_meeple(featureId, meepleKind);
      currentGameState = result.state;
      uiState.pendingMeepleFeatureId = null;
      uiState.pendingMeepleKind = 'normal';

      // Spawn floating score popups
      if (result.score_events && result.score_events.length > 0) {
        playSound('score');
        for (const ev of result.score_events) {
          spawnFloatingScore(ev.x, ev.y, `+${ev.points}`, ev.hex);
          showToast(`${ev.player_names.join(' & ')} scored +${ev.points} pts (${ev.description})`, 'score');
        }
      }

      if (currentGameState.phase === 'GAME_OVER') {
        showGameOverModal();
      } else {
        if (currentGameState.is_builder_bonus_turn) {
          const activePlayer = currentGameState.players[currentGameState.current_player_index];
          if (activePlayer) {
            showToast(`${activePlayer.name} gets a Builder Bonus Turn!`, 'info');
          }
        }
        if (currentGameState.current_tile_type_id) {
          animateDrawFromBag(currentGameState.current_tile_type_id);
        }
      }

      broadcastIfOnline();
      renderAll();
    } catch (err) {
      showToast(err.toString(), 'warn');
    }
  }
}

function broadcastIfOnline() {
  if (isInOnlineRoom()) {
    broadcastState(currentGameState);
  }
}

function renderHandSlot() {
  const tileHandContainer = document.getElementById('tile-hand-container');
  const meepleTray = document.getElementById('meeple-tray');
  const floatingTile = document.getElementById('floating-tile');
  const actionTitle = document.getElementById('action-bar-title');
  const actionSub = document.getElementById('action-bar-sub');
  const btnPrimary = document.getElementById('btn-action-primary');
  const btnSecondary = document.getElementById('btn-action-secondary');

  if (currentGameState.phase === 'PLACE_TILE' && currentGameState.current_tile_type_id) {
    if (tileHandContainer) tileHandContainer.classList.remove('hidden');
    if (meepleTray) {
      meepleTray.classList.add('hidden');
      meepleTray.innerHTML = '';
    }

    const def = tileCatalog[currentGameState.current_tile_type_id];
    floatingTile.innerHTML = `<img src="${def.asset}" draggable="false" class="w-full h-full pointer-events-none select-none object-cover" style="transform: rotate(${uiState.currentRotation * 90}deg)" alt="${def.name}" />`;
    floatingTile.style.cursor = 'grab';
    floatingTile.title = 'Drag tile to board or tap to rotate';

    if (uiState.tileDrag.active && uiState.tileDrag.moved && uiState.tileDrag.source === 'hand') {
      floatingTile.style.opacity = '0.35';
    } else {
      floatingTile.style.opacity = '1';
    }

    floatingTile.onpointerdown = (e) => startTilePointerInteraction(e, 'hand');

    const isRiver = currentGameState.is_river_phase;
    actionTitle.textContent = isRiver ? 'River Tile' : 'Stage Tile';
    actionTitle.className = isRiver ? 'font-medieval font-bold text-xs text-blue-900 uppercase tracking-wider' : 'font-medieval font-bold text-xs text-[#2a1b10] uppercase tracking-wider';
    actionSub.textContent = uiState.pendingPlacement ? 'Tile staged — Tap Confirm' : (isRiver ? 'Place river tile to extend stream' : 'Drag tile to grid or tap slot');
    btnPrimary.textContent = 'Confirm';
    btnSecondary.textContent = 'Reset';

    if (uiState.pendingPlacement) {
      const isValid = wasmEngine.can_place_tile(uiState.pendingPlacement.x, uiState.pendingPlacement.y, uiState.currentRotation);
      btnPrimary.disabled = !isValid;
      btnSecondary.disabled = false;
    } else {
      btnPrimary.disabled = true;
      btnSecondary.disabled = true;
    }
  } else {
    floatingTile.style.cursor = 'default';
    floatingTile.style.opacity = '1';
    floatingTile.onpointerdown = null;

    if (currentGameState.phase === 'PLACE_MEEPLE') {
      if (tileHandContainer) tileHandContainer.classList.add('hidden');
      if (meepleTray) {
        meepleTray.classList.remove('hidden');
        meepleTray.innerHTML = '';

        const activePlayer = currentGameState.players[currentGameState.current_player_index];
        const playerColor = activePlayer ? activePlayer.hex : '#dc2626';

        const pieces = [
          {
            kind: 'normal',
            label: 'Meeple',
            count: activePlayer ? activePlayer.meeples : 0,
            available: activePlayer ? activePlayer.meeples > 0 : false,
            iconSize: 'w-9 h-9'
          }
        ];
        if (currentGameState.inns_cathedrals_enabled && activePlayer && activePlayer.has_big_meeple) {
          pieces.push({ kind: 'big', label: 'Big 2x', available: true, iconSize: 'w-10 h-10' });
        }
        if (currentGameState.traders_builders_enabled && activePlayer && activePlayer.has_builder) {
          pieces.push({ kind: 'builder', label: 'Builder', available: true, iconSize: 'w-9 h-9' });
        }
        if (currentGameState.traders_builders_enabled && activePlayer && activePlayer.has_pig) {
          pieces.push({ kind: 'pig', label: 'Pig', available: true, iconSize: 'w-9 h-9' });
        }

        for (const piece of pieces) {
          const item = document.createElement('div');
          const isStaged = uiState.pendingMeepleFeatureId && uiState.pendingMeepleKind === piece.kind;
          const isDraggingThis = uiState.meepleDrag.active && uiState.meepleDrag.moved && uiState.meepleDrag.kind === piece.kind;
          item.className = `meeple-tray-item relative flex flex-col items-center justify-center w-14 h-16 rounded-xl bg-black/5 border border-[#c9b38f]/70 ${piece.available ? 'cursor-grab active:cursor-grabbing hover:bg-black/10' : 'disabled opacity-35 cursor-not-allowed'}`;
          item.style.opacity = (isStaged || isDraggingThis) ? '0.35' : (piece.available ? '1' : '0.35');
          item.title = piece.available ? `Drag ${piece.label} onto tile` : `No ${piece.label} remaining`;

          const countBadge = piece.count !== undefined
            ? `<span class="absolute -top-1.5 -right-1.5 bg-[#2a1b10] text-amber-200 text-[10px] font-bold px-1.5 py-0.5 rounded-full leading-none shadow">${piece.count}</span>`
            : '';

          item.innerHTML = `
            ${countBadge}
            <div class="${piece.iconSize} flex items-center justify-center pointer-events-none select-none">
              ${renderMeepleSvg(playerColor, 'w-full h-full pointer-events-none select-none', false, piece.kind)}
            </div>
            <span class="text-[10px] font-bold text-[#2a1b10] leading-tight mt-0.5 pointer-events-none select-none">${piece.label}</span>
          `;

          if (piece.available) {
            item.addEventListener('pointerdown', (e) => startMeeplePointerInteraction(e, 'hand', piece.kind));
          }
          meepleTray.appendChild(item);
        }
      }

      actionTitle.textContent = 'Deploy Meeple';
      actionTitle.className = 'font-medieval font-bold text-xs text-[#2a1b10] uppercase tracking-wider';
      actionSub.textContent = uiState.pendingMeepleFeatureId ? 'Piece staged — Confirm or drag' : 'Drag a piece onto tile or Skip';
      btnPrimary.textContent = uiState.pendingMeepleFeatureId ? 'Place Meeple' : 'Skip Meeple';
      btnPrimary.disabled = false;
      btnSecondary.textContent = 'Clear';
      btnSecondary.disabled = !uiState.pendingMeepleFeatureId;
    } else if (currentGameState.phase === 'GAME_OVER') {
      if (tileHandContainer) tileHandContainer.classList.remove('hidden');
      if (meepleTray) {
        meepleTray.classList.add('hidden');
        meepleTray.innerHTML = '';
      }
      actionTitle.textContent = 'Campaign Ended';
      actionTitle.className = 'font-medieval font-bold text-xs text-[#2a1b10] uppercase tracking-wider';
      actionSub.textContent = 'Scores finalized';
      btnPrimary.textContent = 'Game Over';
      btnPrimary.disabled = true;
      btnSecondary.disabled = true;
    }
  }
}

function renderActivePlayerHUD() {
  const player = currentGameState.players[currentGameState.current_player_index];
  if (!player) return;

  const dot = document.getElementById('pill-player-dot');
  const name = document.getElementById('pill-player-name');
  const score = document.getElementById('pill-player-score');
  const meeples = document.getElementById('pill-player-meeples');
  const builderBadge = document.getElementById('builder-bonus-badge');

  dot.style.backgroundColor = player.hex;
  name.textContent = player.name;
  score.textContent = `${player.score} pts`;
  meeples.innerHTML = `<span class="w-3.5 h-3.5 inline-block opacity-90">${renderMeepleSvg(player.hex, 'w-full h-full')}</span> ${player.meeples}`;

  if (builderBadge) {
    builderBadge.classList.toggle('hidden', !currentGameState.is_builder_bonus_turn);
  }
}

function renderTradeGoodsHtml(goods) {
  if (!currentGameState || !currentGameState.traders_builders_enabled) return '';
  const g = goods || { wine: 0, grain: 0, cloth: 0 };
  return `
    <span class="inline-flex items-center gap-1.5 text-[10px] font-mono text-[#594636]">
      <span class="inline-flex items-center gap-0.5" title="Wine"><img src="assets/ui/good_wine.svg" class="w-3 h-3 inline-block" alt="W" onerror="this.replaceWith('🍷')" />${g.wine || 0}</span>
      <span class="inline-flex items-center gap-0.5" title="Grain"><img src="assets/ui/good_grain.svg" class="w-3 h-3 inline-block" alt="G" onerror="this.replaceWith('🌾')" />${g.grain || 0}</span>
      <span class="inline-flex items-center gap-0.5" title="Cloth"><img src="assets/ui/good_cloth.svg" class="w-3 h-3 inline-block" alt="C" onerror="this.replaceWith('🧵')" />${g.cloth || 0}</span>
    </span>
  `;
}

function renderMenuDropdown() {
  document.getElementById('menu-turn-num').textContent = `Turn ${currentGameState.turn_number}`;
  document.getElementById('menu-tiles-left').textContent = `${currentGameState.deck.length}`;
  document.getElementById('menu-zoom-label').textContent = `${Math.round(uiState.zoom * 100)}%`;

  // Players scoreboard
  const playersContainer = document.getElementById('menu-players-list');
  playersContainer.innerHTML = '';
  for (const p of currentGameState.players) {
    const row = document.createElement('div');
    row.className = 'flex items-center justify-between gap-2 text-xs py-1 px-2 rounded-lg bg-black/5';
    row.innerHTML = `
      <div class="flex items-center gap-2 min-w-0">
        <span class="w-3 h-3 rounded-full border border-black/20 flex-shrink-0" style="background-color: ${p.hex}"></span>
        <span class="font-bold text-[#2a1b10] truncate">${p.name}</span>
      </div>
      <div class="flex items-center gap-2 font-mono flex-shrink-0">
        ${renderTradeGoodsHtml(p.goods)}
        <span class="text-amber-800 font-semibold flex items-center gap-1">
          <span class="w-3 h-3 inline-block opacity-90">${renderMeepleSvg(p.hex, 'w-full h-full')}</span> ${p.meeples}
        </span>
        <span class="font-bold text-[#2a1b10]">${p.score} pts</span>
      </div>
    `;
    playersContainer.appendChild(row);
  }

  // Log entries
  const logContainer = document.getElementById('menu-game-log');
  logContainer.innerHTML = '';
  for (const item of (currentGameState.log || []).slice(0, 15)) {
    const line = document.createElement('div');
    line.className = item.tone === 'score' ? 'text-green-800 font-semibold' : (item.tone === 'warn' ? 'text-amber-800' : 'text-[#423223]');
    line.textContent = `• ${item.text}`;
    logContainer.appendChild(line);
  }
}

function renderTileBagBadge() {
  const badge = document.getElementById('bag-count-badge');
  const riverBadge = document.getElementById('river-phase-badge');
  const riverCount = document.getElementById('river-remaining-count');

  if (currentGameState.is_river_phase) {
    const remaining = (currentGameState.river_deck ? currentGameState.river_deck.length : 0) + (currentGameState.current_tile_type_id ? 1 : 0);
    if (badge) {
      badge.textContent = `${remaining}`;
      badge.classList.remove('bg-red-600');
      badge.classList.add('bg-blue-600');
      badge.title = `${remaining} river tiles remaining`;
    }
    if (riverBadge) {
      riverBadge.classList.remove('hidden');
      if (riverCount) riverCount.textContent = `${remaining}`;
    }
  } else {
    if (badge) {
      badge.textContent = `${currentGameState.deck.length}`;
      badge.classList.remove('bg-blue-600');
      badge.classList.add('bg-red-600');
      badge.title = `${currentGameState.deck.length} tiles in bag`;
    }
    if (riverBadge) {
      riverBadge.classList.add('hidden');
    }
  }
}

function spawnFloatingScore(tileX, tileY, text, colorHex) {
  const el = document.createElement('div');
  el.className = 'floating-score';
  el.style.left = `${tileX * TILE_SIZE + 50}px`;
  el.style.top = `${tileY * TILE_SIZE + 35}px`;
  el.style.color = colorHex || '#facc15';
  el.textContent = text;
  boardStage.appendChild(el);
  setTimeout(() => el.remove(), 1400);
}

function showToast(message, tone = 'info') {
  const container = document.getElementById('toast-container');
  const el = document.createElement('div');
  const bgClass = tone === 'warn'
    ? 'bg-[#7c2d12]/95 border-[#fb923c] text-[#ffedd5]'
    : tone === 'score'
      ? 'bg-[#14532d]/95 border-[#4ade80] text-[#dcfce7]'
      : 'bg-[#1f150f]/95 border-[#c99738] text-[#f5ebd6]';
  el.className = `px-4 py-2 rounded-full border text-xs sm:text-sm font-semibold shadow-2xl backdrop-blur-sm transition-all duration-300 opacity-0 -translate-y-2 ${bgClass}`;
  el.textContent = message;
  container.appendChild(el);
  requestAnimationFrame(() => el.classList.remove('opacity-0', '-translate-y-2'));
  setTimeout(() => {
    el.classList.add('opacity-0', '-translate-y-2');
    setTimeout(() => el.remove(), 320);
  }, 2800);
}

function animateDrawFromBag(typeId) {
  const bagBtn = document.getElementById('tile-bag-btn');
  const targetTile = document.getElementById('floating-tile');
  if (!bagBtn || !targetTile) return;

  bagBtn.classList.remove('bag-draw-bounce');
  void bagBtn.offsetWidth;
  bagBtn.classList.add('bag-draw-bounce');

  const bagRect = bagBtn.getBoundingClientRect();
  const targetRect = targetTile.getBoundingClientRect();
  const def = tileCatalog[typeId];
  if (!def) return;

  const flyer = document.createElement('div');
  flyer.style.position = 'fixed';
  flyer.style.zIndex = '60';
  flyer.style.pointerEvents = 'none';
  flyer.style.width = `${targetRect.width}px`;
  flyer.style.height = `${targetRect.height}px`;
  flyer.style.borderRadius = '12px';
  flyer.style.boxShadow = '0 14px 30px rgba(0,0,0,0.65), 0 0 0 2px #fde047';
  flyer.style.left = `${bagRect.left + bagRect.width / 2 - targetRect.width / 2}px`;
  flyer.style.top = `${bagRect.top + bagRect.height / 2 - targetRect.height / 2}px`;
  flyer.innerHTML = `<img src="${def.asset}" class="w-full h-full rounded-xl object-cover" alt="Drawn Tile" />`;
  document.body.appendChild(flyer);

  const dx = targetRect.left - (bagRect.left + bagRect.width / 2 - targetRect.width / 2);
  const dy = targetRect.top - (bagRect.top + bagRect.height / 2 - targetRect.height / 2);

  const anim = flyer.animate([
    { transform: 'translate(0px, 0px) scale(0.25) rotate(24deg)', opacity: 0.3 },
    { transform: `translate(${dx * 0.5}px, ${dy * 0.5 - 40}px) scale(0.9) rotate(-8deg)`, opacity: 1, offset: 0.55 },
    { transform: `translate(${dx}px, ${dy}px) scale(1) rotate(0deg)`, opacity: 1 }
  ], {
    duration: 440,
    easing: 'cubic-bezier(0.22, 1, 0.36, 1)'
  });

  anim.onfinish = () => flyer.remove();
}

// Camera Pan & Zoom
function applyCameraTransform() {
  boardStage.style.transform = `translate(${uiState.panX}px, ${uiState.panY}px) scale(${uiState.zoom})`;
  const label = document.getElementById('menu-zoom-label');
  if (label) label.textContent = `${Math.round(uiState.zoom * 100)}%`;
}

function recenterBoard() {
  const rect = boardViewport.getBoundingClientRect();
  const entries = getBoardEntries(currentGameState);
  if (entries.length === 0) {
    uiState.panX = rect.width / 2 - TILE_SIZE / 2;
    uiState.panY = rect.height / 2 - TILE_SIZE / 2;
  } else {
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const [, t] of entries) {
      if (t.x < minX) minX = t.x;
      if (t.x > maxX) maxX = t.x;
      if (t.y < minY) minY = t.y;
      if (t.y > maxY) maxY = t.y;
    }
    const centerWorldX = ((minX + maxX + 1) * TILE_SIZE) / 2;
    const centerWorldY = ((minY + maxY + 1) * TILE_SIZE) / 2;
    uiState.zoom = Math.min(1.1, Math.max(0.65, uiState.zoom));
    uiState.panX = rect.width / 2 - centerWorldX * uiState.zoom;
    uiState.panY = rect.height / 2 - centerWorldY * uiState.zoom;
  }
  applyCameraTransform();
}

function zoomAtPoint(factor, clientX, clientY) {
  const rect = boardViewport.getBoundingClientRect();
  const mx = clientX - rect.left;
  const my = clientY - rect.top;
  const prevZoom = uiState.zoom;
  const nextZoom = Math.min(2.2, Math.max(0.35, prevZoom * factor));
  const ratio = nextZoom / prevZoom;
  uiState.panX = mx - (mx - uiState.panX) * ratio;
  uiState.panY = my - (my - uiState.panY) * ratio;
  uiState.zoom = nextZoom;
  applyCameraTransform();
}

function setupEventListeners() {
  // Rotate buttons
  document.getElementById('btn-rotate-cw').addEventListener('click', () => rotateTile('cw'));
  document.getElementById('btn-rotate-ccw').addEventListener('click', () => rotateTile('ccw'));

  // Action buttons
  document.getElementById('btn-action-primary').addEventListener('click', confirmTurnAction);
  document.getElementById('btn-action-secondary').addEventListener('click', () => {
    uiState.pendingPlacement = null;
    uiState.pendingMeepleFeatureId = null;
    uiState.pendingMeepleKind = 'normal';
    renderAll();
  });

  // Sound toggle
  document.getElementById('btn-sound').addEventListener('click', () => {
    setSoundEnabled(!isSoundEnabled());
    document.getElementById('icon-sound-label').textContent = isSoundEnabled() ? 'ON' : 'OFF';
    document.getElementById('icon-sound-label').className = isSoundEnabled() ? 'font-bold text-green-700' : 'font-bold text-red-700';
  });

  // Menu toggle
  const menuBtn = document.getElementById('btn-menu-toggle');
  const menuDropdown = document.getElementById('menu-dropdown');
  menuBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    menuDropdown.classList.toggle('hidden');
  });
  document.addEventListener('click', (e) => {
    if (!menuDropdown.contains(e.target) && !menuBtn.contains(e.target)) {
      menuDropdown.classList.add('hidden');
    }
  });

  // Camera buttons
  document.getElementById('btn-zoom-in').addEventListener('click', () => zoomAtPoint(1.2, window.innerWidth / 2, window.innerHeight / 2));
  document.getElementById('btn-zoom-out').addEventListener('click', () => zoomAtPoint(0.8, window.innerWidth / 2, window.innerHeight / 2));
  document.getElementById('btn-recenter').addEventListener('click', recenterBoard);

  // Viewport Pan & Pinch + Tile Drag + Meeple Drag
  boardViewport.addEventListener('pointerdown', (e) => {
    if (e.target.closest('.staged-meeple') || e.target.closest('.pending-tile')) return;
    if (uiState.tileDrag.active || uiState.meepleDrag.active) return;
    uiState.isDraggingBoard = true;
    uiState.dragStartX = e.clientX;
    uiState.dragStartY = e.clientY;
    uiState.panOriginX = uiState.panX;
    uiState.panOriginY = uiState.panY;
    boardViewport.classList.add('dragging');
  });

  // Prevent browser native drag-and-drop on images/links
  window.addEventListener('dragstart', (e) => e.preventDefault());

  window.addEventListener('pointermove', (e) => {
    if (uiState.tileDrag.active) {
      const dist = Math.hypot(e.clientX - uiState.tileDrag.startX, e.clientY - uiState.tileDrag.startY);
      if (!uiState.tileDrag.moved && dist >= DRAG_THRESHOLD) {
        uiState.tileDrag.moved = true;
        const dragGhost = document.getElementById('drag-ghost');
        const def = tileCatalog[currentGameState.current_tile_type_id];
        if (dragGhost && def) {
          dragGhost.classList.remove('hidden', 'meeple-ghost');
          dragGhost.innerHTML = `<img src="${def.asset}" draggable="false" class="w-full h-full rounded-[6px] select-none pointer-events-none object-cover" style="transform: rotate(${uiState.currentRotation * 90}deg)" alt="Dragging" />`;
        }
        const floatingTile = document.getElementById('floating-tile');
        if (floatingTile && uiState.tileDrag.source === 'hand') {
          floatingTile.style.opacity = '0.35';
        }
      }

      if (uiState.tileDrag.moved) {
        updateDragGhostPosition(e.clientX, e.clientY);
        const cell = getGridCellFromPointer(e.clientX, e.clientY);
        const isOccupied = !!getBoardTile(currentGameState, `${cell.x},${cell.y}`);
        const snapped = isOccupied ? null : cell;
        const prevSnap = uiState.tileDrag.snappedSlot;
        const changed = (!prevSnap && snapped) || (prevSnap && !snapped) || (prevSnap && snapped && (prevSnap.x !== snapped.x || prevSnap.y !== snapped.y));
        if (changed) {
          uiState.tileDrag.snappedSlot = snapped;
          renderBoard();
        }
      }
      return;
    }

    if (uiState.meepleDrag.active) {
      const dist = Math.hypot(e.clientX - uiState.meepleDrag.startX, e.clientY - uiState.meepleDrag.startY);
      if (!uiState.meepleDrag.moved && dist >= 4) {
        uiState.meepleDrag.moved = true;
      }

      if (uiState.meepleDrag.moved) {
        const kind = uiState.meepleDrag.kind || 'normal';
        const spot = findNearestMeepleSpot(e.clientX, e.clientY, kind);
        const dragGhost = document.getElementById('drag-ghost');
        const activePlayer = currentGameState.players[currentGameState.current_player_index];
        const color = activePlayer ? activePlayer.hex : '#dc2626';

        if (spot) {
          // Snapped to a feature on the board: hide cursor ghost and show staged meeple in place
          if (dragGhost) {
            dragGhost.classList.add('hidden');
            dragGhost.classList.remove('meeple-ghost');
            dragGhost.innerHTML = '';
          }
          if (uiState.pendingMeepleFeatureId !== spot.id || uiState.pendingMeepleKind !== kind) {
            uiState.pendingMeepleFeatureId = spot.id;
            uiState.pendingMeepleKind = kind;
            uiState.meepleDrag.snappedSpot = spot;
            playSound('meeple');
            renderBoard();
            renderHandSlot();
          }
        } else {
          // Not near a valid spot: show meeple following the pointer
          if (dragGhost) {
            dragGhost.classList.remove('hidden');
            dragGhost.classList.add('meeple-ghost');
            dragGhost.innerHTML = renderMeepleSvg(color, 'w-full h-full pointer-events-none select-none', false, kind);
            updateMeepleDragGhostPosition(e.clientX, e.clientY);
          }
          if (uiState.pendingMeepleFeatureId !== null) {
            uiState.pendingMeepleFeatureId = null;
            uiState.pendingMeepleKind = 'normal';
            uiState.meepleDrag.snappedSpot = null;
            renderBoard();
            renderHandSlot();
          }
        }
      }
      return;
    }

    if (!uiState.isDraggingBoard) return;
    uiState.panX = uiState.panOriginX + (e.clientX - uiState.dragStartX);
    uiState.panY = uiState.panOriginY + (e.clientY - uiState.dragStartY);
    applyCameraTransform();
  });

  const handlePointerEnd = (e) => {
    if (uiState.tileDrag.active) {
      const wasMoved = uiState.tileDrag.moved;
      const snap = uiState.tileDrag.snappedSlot;

      const dragGhost = document.getElementById('drag-ghost');
      if (dragGhost) {
        dragGhost.classList.add('hidden');
        dragGhost.innerHTML = '';
      }

      const floatingTile = document.getElementById('floating-tile');
      if (floatingTile) {
        floatingTile.style.opacity = '1';
      }

      uiState.tileDrag.active = false;
      uiState.tileDrag.snappedSlot = null;
      if (wasMoved) {
        uiState.tileDrag.justDragged = true;
        setTimeout(() => { uiState.tileDrag.justDragged = false; }, 120);
      }
      uiState.tileDrag.moved = false;

      if (!wasMoved) {
        // Tap / Click without dragging: rotate tile 90 deg CW
        rotateTile('cw');
      } else {
        if (snap && !getBoardTile(currentGameState, `${snap.x},${snap.y}`)) {
          stagePendingTile(snap.x, snap.y);
        } else {
          renderAll();
        }
      }
      return;
    }

    if (uiState.meepleDrag.active) {
      const dragGhost = document.getElementById('drag-ghost');
      if (dragGhost) {
        dragGhost.classList.add('hidden');
        dragGhost.classList.remove('meeple-ghost');
        dragGhost.innerHTML = '';
      }

      uiState.meepleDrag.active = false;
      uiState.meepleDrag.moved = false;
      uiState.meepleDrag.snappedSpot = null;
      renderAll();
      return;
    }

    if (uiState.isDraggingBoard) {
      const clientX = e ? e.clientX : uiState.dragStartX;
      const clientY = e ? e.clientY : uiState.dragStartY;
      const dist = Math.hypot(clientX - uiState.dragStartX, clientY - uiState.dragStartY);
      uiState.isDraggingBoard = false;
      boardViewport.classList.remove('dragging');

      if (dist < 6 && currentGameState) {
        if (currentGameState.phase === 'PLACE_TILE' && currentGameState.current_tile_type_id) {
          const cell = getGridCellFromPointer(clientX, clientY);
          if (!getBoardTile(currentGameState, `${cell.x},${cell.y}`)) {
            stagePendingTile(cell.x, cell.y);
          }
        } else if (currentGameState.phase === 'PLACE_MEEPLE') {
          const activePlayer = currentGameState.players[currentGameState.current_player_index];
          const tryKinds = [];
          if (activePlayer && activePlayer.meeples > 0) tryKinds.push('normal');
          if (currentGameState.inns_cathedrals_enabled && activePlayer && activePlayer.has_big_meeple) tryKinds.push('big');
          if (currentGameState.traders_builders_enabled && activePlayer && activePlayer.has_builder) tryKinds.push('builder');
          if (currentGameState.traders_builders_enabled && activePlayer && activePlayer.has_pig) tryKinds.push('pig');

          for (const tryKind of tryKinds) {
            const spot = findNearestMeepleSpot(clientX, clientY, tryKind);
            if (spot) {
              uiState.pendingMeepleFeatureId = spot.id;
              uiState.pendingMeepleKind = tryKind;
              playSound('meeple');
              renderAll();
              break;
            }
          }
        }
      }
    }
  };

  window.addEventListener('pointerup', handlePointerEnd);
  window.addEventListener('pointercancel', handlePointerEnd);

  boardViewport.addEventListener('wheel', (e) => {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.1 : 0.9;
    zoomAtPoint(factor, e.clientX, e.clientY);
  }, { passive: false });

  // Modals & Menu Buttons
  document.getElementById('btn-new-game').addEventListener('click', () => {
    menuDropdown.classList.add('hidden');
    document.getElementById('modal-setup').classList.remove('hidden');
  });

  document.getElementById('btn-finish-now').addEventListener('click', () => {
    menuDropdown.classList.add('hidden');
    if (wasmEngine) {
      wasmEngine.trigger_game_over();
      currentGameState = wasmEngine.get_state();
      showGameOverModal();
    }
  });

  document.getElementById('tile-bag-btn').addEventListener('click', openBagModal);
  document.getElementById('btn-close-bag').addEventListener('click', () => {
    document.getElementById('modal-bag').classList.add('hidden');
  });

  document.getElementById('btn-open-multiplayer').addEventListener('click', () => {
    menuDropdown.classList.add('hidden');
    document.getElementById('modal-multiplayer').classList.remove('hidden');
  });
  document.getElementById('btn-close-multiplayer').addEventListener('click', () => {
    document.getElementById('modal-multiplayer').classList.add('hidden');
  });

  document.getElementById('btn-save-firebase-config').addEventListener('click', () => {
    const raw = document.getElementById('firebase-config-input').value;
    try {
      const parsed = JSON.parse(raw);
      saveFirebaseConfig(parsed);
      initFirebase(parsed);
      showToast('Firebase config saved successfully!', 'score');
    } catch {
      showToast('Invalid JSON in Firebase config', 'warn');
    }
  });

  document.getElementById('btn-host-room').addEventListener('click', handleHostRoom);
  document.getElementById('btn-join-room').addEventListener('click', handleJoinRoom);

  document.getElementById('btn-gameover-inspect').addEventListener('click', () => {
    document.getElementById('modal-gameover').classList.add('hidden');
  });
  document.getElementById('btn-play-again').addEventListener('click', () => {
    document.getElementById('modal-gameover').classList.add('hidden');
    document.getElementById('modal-setup').classList.remove('hidden');
  });
}

function openBagModal() {
  const grid = document.getElementById('bag-modal-grid');
  grid.innerHTML = '';

  // Count remaining tiles in deck
  const counts = {};
  for (const id of currentGameState.deck) {
    counts[id] = (counts[id] || 0) + 1;
  }

  for (const def of tileCatalogList) {
    if (def.is_start) continue;
    if (def.expansion === 'inns_cathedrals' && !currentGameState.inns_cathedrals_enabled) continue;
    if (def.expansion === 'traders_builders' && !currentGameState.traders_builders_enabled) continue;
    if (def.is_river && !currentGameState.river_enabled) continue;
    const remaining = counts[def.id] || 0;
    const item = document.createElement('div');
    item.className = 'parchment-panel rounded-xl p-2.5 flex flex-col items-center text-center gap-1.5 shadow-md';
    item.innerHTML = `
      <div class="w-16 h-16 rounded-lg overflow-hidden border border-[#c9b38f] shadow-inner bg-[#77a851]">
        <img src="${def.asset}" class="w-full h-full object-cover" alt="${def.name}" />
      </div>
      <span class="font-bold text-[11px] text-[#2a1b10] line-clamp-1">${def.name}</span>
      <span class="px-2 py-0.5 rounded-full text-[10px] font-bold ${remaining > 0 ? 'bg-amber-100 text-amber-900 border border-amber-300' : 'bg-gray-200 text-gray-500'}">
        ${remaining} / ${def.count} left
      </span>
    `;
    grid.appendChild(item);
  }

  document.getElementById('modal-bag').classList.remove('hidden');
}

function initSetupModal() {
  const container = document.getElementById('setup-player-inputs');
  const countBtns = document.querySelectorAll('#setup-count-btns button');

  function renderPlayerInputs(count) {
    container.innerHTML = '';
    for (let i = 0; i < count; i++) {
      const color = PLAYER_COLORS[i % PLAYER_COLORS.length];
      const row = document.createElement('div');
      row.className = 'flex items-center gap-2 bg-black/5 p-2 rounded-xl';
      row.innerHTML = `
        <span class="w-6 h-6 rounded-full border-2 border-white shadow flex-shrink-0" style="background-color: ${color.hex}"></span>
        <input type="text" class="player-name-input flex-1 bg-white border border-[#c9b38f] rounded-lg px-2.5 py-1 text-xs font-bold text-[#2a1b10] focus:outline-none" value="Player ${i + 1}" />
        <span class="text-[10px] font-semibold uppercase text-[#594636]">${color.label}</span>
      `;
      container.appendChild(row);
    }
  }

  countBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      countBtns.forEach(b => {
        b.className = 'btn-secondary py-2 rounded-xl font-bold text-sm';
      });
      btn.className = 'btn-medieval py-2 rounded-xl font-bold text-sm';
      uiState.setupPlayerCount = parseInt(btn.dataset.count, 10);
      renderPlayerInputs(uiState.setupPlayerCount);
    });
  });

  renderPlayerInputs(2);

  document.getElementById('btn-start-game').addEventListener('click', () => {
    const nameInputs = container.querySelectorAll('.player-name-input');
    const players = [];
    nameInputs.forEach((input, idx) => {
      const color = PLAYER_COLORS[idx % PLAYER_COLORS.length];
      players.push({
        id: `p_${idx}`,
        name: input.value.trim() || `Player ${idx + 1}`,
        color_key: color.key,
        hex: color.hex,
        score: 0,
        meeples: 7
      });
    });

    const farmersEnabled = document.getElementById('setup-farmers-toggle').checked;
    const riverToggle = document.getElementById('setup-river-toggle');
    const riverEnabled = riverToggle ? riverToggle.checked : true;
    const icToggle = document.getElementById('setup-ic-toggle');
    const innsCathedralsEnabled = icToggle ? icToggle.checked : true;
    const tbToggle = document.getElementById('setup-tb-toggle');
    const tradersBuildersEnabled = tbToggle ? tbToggle.checked : true;
    document.getElementById('modal-setup').classList.add('hidden');
    startNewLocalGame(players, farmersEnabled, riverEnabled, innsCathedralsEnabled, tradersBuildersEnabled);
  });
}

async function handleHostRoom() {
  try {
    const roomId = Math.random().toString(36).substring(2, 6).toUpperCase();
    const farmersEnabled = document.getElementById('setup-farmers-toggle').checked;
    const riverToggle = document.getElementById('setup-river-toggle');
    const riverEnabled = riverToggle ? riverToggle.checked : true;
    const icToggle = document.getElementById('setup-ic-toggle');
    const innsCathedralsEnabled = icToggle ? icToggle.checked : true;
    const tbToggle = document.getElementById('setup-tb-toggle');
    const tradersBuildersEnabled = tbToggle ? tbToggle.checked : true;
    const playerConfig = { id: 'p_0', name: 'Host Player', color_key: 'red', hex: '#dc2626', score: 0, meeples: 7 };

    wasmEngine = new CarcassonneEngineWasm(
      JSON.stringify(tileCatalogList),
      JSON.stringify([playerConfig]),
      farmersEnabled,
      riverEnabled,
      innsCathedralsEnabled,
      tradersBuildersEnabled
    );
    currentGameState = wasmEngine.get_state();

    await createRoom(roomId, currentGameState, playerConfig);
    window.myCurrentPlayerId = playerConfig.id;

    listenToRoom((remoteState) => {
      wasmEngine.import_state(JSON.stringify(remoteState));
      currentGameState = wasmEngine.get_state();
      renderAll();
    }, (players) => {
      showToast(`Room players updated (${Object.keys(players).length})`, 'info');
    });

    document.getElementById('modal-multiplayer').classList.add('hidden');
    updateMultiplayerButtonLabel();
    showToast(`Hosting Room ${roomId}! Share code with friends.`, 'score');
  } catch (err) {
    showToast(err.message, 'warn');
  }
}

async function handleJoinRoom() {
  const code = document.getElementById('input-room-code').value.trim().toUpperCase();
  if (!code) {
    showToast('Please enter a room code', 'warn');
    return;
  }
  await connectToRoom(code);
}

async function promptJoinRoomFromUrl(roomId) {
  await connectToRoom(roomId);
}

async function connectToRoom(roomId) {
  try {
    const playerConfig = {
      id: `p_${Date.now().toString(36)}`,
      name: `Player ${Math.floor(Math.random() * 900 + 100)}`,
      color_key: 'blue',
      hex: '#2563eb',
      score: 0,
      meeples: 7
    };

    const roomData = await joinRoom(roomId, playerConfig);
    window.myCurrentPlayerId = playerConfig.id;

    const farmersEnabled = true;
    wasmEngine = new CarcassonneEngineWasm(
      JSON.stringify(tileCatalogList),
      JSON.stringify([playerConfig]),
      farmersEnabled,
      true,
      true,
      true
    );

    if (roomData.state_json) {
      wasmEngine.import_state(roomData.state_json);
      currentGameState = wasmEngine.get_state();
    }

    listenToRoom((remoteState) => {
      wasmEngine.import_state(JSON.stringify(remoteState));
      currentGameState = wasmEngine.get_state();
      renderAll();
    });

    document.getElementById('modal-multiplayer').classList.add('hidden');
    updateMultiplayerButtonLabel();
    recenterBoard();
    renderAll();
    showToast(`Connected to Room ${roomId}!`, 'score');
  } catch (err) {
    showToast(err.message, 'warn');
  }
}

function updateMultiplayerButtonLabel() {
  const label = document.getElementById('multiplayer-btn-label');
  if (!label) return;
  if (isInOnlineRoom()) {
    label.textContent = `Online: Room ${getCurrentRoomId()}`;
  } else {
    label.textContent = 'Online Multiplayer (Firebase)';
  }
}

function showGameOverModal() {
  const modal = document.getElementById('modal-gameover');
  const winnerText = document.getElementById('gameover-winner-text');
  const scoreboard = document.getElementById('gameover-scoreboard');

  // Sort players by score descending
  const sorted = [...currentGameState.players].sort((a, b) => b.score - a.score);
  const winner = sorted[0];

  winnerText.textContent = `${winner.name} Triumphs with ${winner.score} Points!`;
  scoreboard.innerHTML = '';

  sorted.forEach((p, rank) => {
    const row = document.createElement('div');
    row.className = 'parchment-panel rounded-xl p-3 flex items-center justify-between gap-2';
    row.innerHTML = `
      <div class="flex items-center gap-3 min-w-0">
        <span class="font-medieval font-bold text-base text-[#594636]">#${rank + 1}</span>
        <span class="w-4 h-4 rounded-full border border-black/20 flex-shrink-0" style="background-color: ${p.hex}"></span>
        <div class="flex flex-col min-w-0">
          <span class="font-bold text-sm text-[#2a1b10] truncate">${p.name}</span>
          ${renderTradeGoodsHtml(p.goods)}
        </div>
      </div>
      <span class="font-medieval font-extrabold text-base text-amber-800 flex-shrink-0">${p.score} pts</span>
    `;
    scoreboard.appendChild(row);
  });

  modal.classList.remove('hidden');
}

// Start application
main().catch(console.error);
