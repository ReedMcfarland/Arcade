/**
 * Firebase Realtime Database Synchronization Module
 * Handles online multiplayer room creation, lobby, joining, action broadcasting, and presence.
 */

let firebaseApp = null;
let firebaseDb = null;
let currentRoomId = null;
let isHost = false;
let myPlayerId = null;
let roomStateListener = null;

const FIREBASE_CONFIG_STORAGE_KEY = 'carcassonne_firebase_config';
const STALE_ROOM_TTL_MS = 24 * 60 * 60 * 1000;

export function getSavedFirebaseConfig() {
  try {
    const raw = localStorage.getItem(FIREBASE_CONFIG_STORAGE_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function saveFirebaseConfig(config) {
  try {
    localStorage.setItem(FIREBASE_CONFIG_STORAGE_KEY, JSON.stringify(config));
  } catch (err) {
    console.error('Failed to save Firebase config:', err);
  }
}

export function initFirebase(config) {
  if (!config || !config.databaseURL) {
    return false;
  }
  try {
    if (window.firebase) {
      if (!window.firebase.apps.length) {
        firebaseApp = window.firebase.initializeApp(config);
      } else {
        firebaseApp = window.firebase.app();
      }
      firebaseDb = window.firebase.database();
      return true;
    }
  } catch (err) {
    console.warn('Firebase initialization error:', err);
  }
  return false;
}

export function isInOnlineRoom() {
  return currentRoomId !== null;
}

export function getCurrentRoomId() {
  return currentRoomId;
}

export function getMyPlayerId() {
  return myPlayerId;
}

export function setIsHost(host) {
  isHost = host;
}

export function checkIsHost() {
  return isHost;
}

async function pruneStaleRooms() {
  if (!firebaseDb) return;
  try {
    const snap = await firebaseDb.ref('carcassonne_rooms').once('value');
    if (!snap.exists()) return;
    const now = Date.now();
    const removals = [];
    snap.forEach((child) => {
      const val = child.val() || {};
      const lastActive = val.last_updated || val.created_at || 0;
      if (now - lastActive > STALE_ROOM_TTL_MS) {
        removals.push(child.ref.remove());
      }
    });
    if (removals.length > 0) {
      await Promise.all(removals);
    }
  } catch (err) {
    console.warn('Stale room cleanup skipped:', err);
  }
}

export async function createRoom(roomId, hostPlayer, expansions) {
  if (!firebaseDb) {
    throw new Error('Firebase Database not initialized. Please configure Firebase settings.');
  }
  await pruneStaleRooms();

  currentRoomId = roomId.toUpperCase();
  isHost = true;
  myPlayerId = hostPlayer.id;

  const now = Date.now();
  const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);
  await roomRef.set({
    created_at: now,
    last_updated: now,
    status: 'lobby',
    host_id: myPlayerId,
    expansions: expansions || {
      farmers: true,
      river: true,
      innsCathedrals: true,
      tradersBuilders: true
    },
    state_json: null,
    players: {
      [myPlayerId]: {
        id: myPlayerId,
        name: hostPlayer.name,
        color_key: hostPlayer.color_key,
        hex: hostPlayer.hex,
        joined_at: now,
        connected: true
      }
    }
  });

  setupPresence(currentRoomId, myPlayerId, true);
  return currentRoomId;
}

export async function joinRoom(roomId, playerName, playerColors) {
  if (!firebaseDb) {
    throw new Error('Firebase Database not initialized. Please configure Firebase settings.');
  }
  const normalizedCode = roomId.toUpperCase();
  const roomRef = firebaseDb.ref(`carcassonne_rooms/${normalizedCode}`);
  const snapshot = await roomRef.once('value');
  if (!snapshot.exists()) {
    throw new Error(`Room "${normalizedCode}" not found. Verify the room code.`);
  }

  const roomData = snapshot.val();
  if (roomData.status === 'playing') {
    throw new Error(`Room "${normalizedCode}" has already started.`);
  }

  const existingPlayers = Object.values(roomData.players || {});
  if (existingPlayers.length >= 6) {
    throw new Error(`Room "${normalizedCode}" is full (max 6 players).`);
  }

  const usedKeys = new Set(existingPlayers.map((p) => p.color_key));
  const assignedColor = playerColors.find((c) => !usedKeys.has(c.key)) || playerColors[existingPlayers.length % playerColors.length];

  const newPlayerId = `p_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
  const now = Date.now();

  currentRoomId = normalizedCode;
  myPlayerId = newPlayerId;
  isHost = false;

  await roomRef.child(`players/${myPlayerId}`).set({
    id: myPlayerId,
    name: playerName || `Player ${existingPlayers.length + 1}`,
    color_key: assignedColor.key,
    hex: assignedColor.hex,
    joined_at: now,
    connected: true
  });

  await roomRef.child('last_updated').set(now);
  setupPresence(currentRoomId, myPlayerId, false);
  return {
    roomData,
    playerId: myPlayerId,
    color: assignedColor
  };
}

export async function updatePlayerName(newName) {
  if (!firebaseDb || !currentRoomId || !myPlayerId) return;
  const trimmed = (newName || '').trim();
  if (!trimmed) return;
  const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);
  await roomRef.child(`players/${myPlayerId}/name`).set(trimmed);
  await roomRef.child('last_updated').set(Date.now());
}

export async function startRoomGame(stateJson) {
  if (!firebaseDb || !currentRoomId || !isHost) return;
  const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);
  await roomRef.update({
    status: 'playing',
    state_json: typeof stateJson === 'string' ? stateJson : JSON.stringify(stateJson),
    last_updated: Date.now()
  });
}

export function listenToRoom(onRoomUpdate) {
  if (!firebaseDb || !currentRoomId) return;

  const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);

  if (roomStateListener) {
    roomRef.off('value', roomStateListener);
  }

  roomStateListener = (snapshot) => {
    if (!snapshot.exists()) {
      if (onRoomUpdate) onRoomUpdate(null);
      return;
    }
    const data = snapshot.val();
    if (onRoomUpdate) {
      onRoomUpdate(data);
    }
  };

  roomRef.on('value', roomStateListener);
}

export async function broadcastState(gameStateOrJson) {
  if (!firebaseDb || !currentRoomId) return;
  try {
    const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);
    const stateJson = typeof gameStateOrJson === 'string'
      ? gameStateOrJson
      : JSON.stringify(gameStateOrJson);
    await roomRef.update({
      state_json: stateJson,
      last_updated: Date.now()
    });
  } catch (err) {
    console.error('Failed to broadcast game state:', err);
  }
}

export async function leaveRoom() {
  const db = firebaseDb;
  const roomId = currentRoomId;
  const playerId = myPlayerId;
  const wasHost = isHost;

  if (db && roomId) {
    const roomRef = db.ref(`carcassonne_rooms/${roomId}`);
    if (roomStateListener) {
      roomRef.off('value', roomStateListener);
      roomStateListener = null;
    }

    currentRoomId = null;
    myPlayerId = null;
    isHost = false;

    try {
      if (wasHost) {
        await roomRef.remove();
      } else if (playerId) {
        await roomRef.child(`players/${playerId}`).remove();
        const playersSnap = await roomRef.child('players').once('value');
        const players = playersSnap.val() || {};
        const anyConnected = Object.values(players).some((p) => p && p.connected === true);
        if (!anyConnected) {
          await roomRef.remove();
        }
      }
    } catch (err) {
      console.warn('Error closing/leaving room:', err);
    }
    return;
  }

  currentRoomId = null;
  myPlayerId = null;
  isHost = false;
}

function setupPresence(roomId, playerId, host) {
  if (!firebaseDb) return;
  const connectedRef = firebaseDb.ref('.info/connected');
  const roomRef = firebaseDb.ref(`carcassonne_rooms/${roomId}`);
  const myPresenceRef = roomRef.child(`players/${playerId}/connected`);

  connectedRef.on('value', (snap) => {
    if (snap.val() === true) {
      if (host) {
        roomRef.onDisconnect().remove();
      } else {
        myPresenceRef.onDisconnect().set(false);
      }
      myPresenceRef.set(true);
    }
  });
}
