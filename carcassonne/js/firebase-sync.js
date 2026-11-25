/**
 * Firebase Realtime Database Synchronization Module
 * Handles online multiplayer room creation, joining, action broadcasting, and presence.
 */

let firebaseApp = null;
let firebaseDb = null;
let currentRoomId = null;
let isHost = false;
let myPlayerId = null;
let roomStateListener = null;

const FIREBASE_CONFIG_STORAGE_KEY = 'carcassonne_firebase_config';

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

export async function createRoom(roomId, initialGameState, playerConfig) {
  if (!firebaseDb) {
    throw new Error('Firebase Database not initialized. Please configure Firebase settings.');
  }
  currentRoomId = roomId.toUpperCase();
  isHost = true;
  myPlayerId = playerConfig.id;

  const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);
  await roomRef.set({
    created_at: Date.now(),
    host_id: myPlayerId,
    state_json: JSON.stringify(initialGameState),
    players: {
      [myPlayerId]: {
        name: playerConfig.name,
        color_key: playerConfig.color_key,
        hex: playerConfig.hex,
        connected: true,
        last_seen: Date.now()
      }
    }
  });

  setupPresence(currentRoomId, myPlayerId);
  return currentRoomId;
}

export async function joinRoom(roomId, playerConfig) {
  if (!firebaseDb) {
    throw new Error('Firebase Database not initialized. Please configure Firebase settings.');
  }
  currentRoomId = roomId.toUpperCase();
  myPlayerId = playerConfig.id;
  isHost = false;

  const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);
  const snapshot = await roomRef.once('value');
  if (!snapshot.exists()) {
    throw new Error(`Room "${currentRoomId}" not found. Verify the room code.`);
  }

  const roomData = snapshot.val();
  await roomRef.child(`players/${myPlayerId}`).set({
    name: playerConfig.name,
    color_key: playerConfig.color_key,
    hex: playerConfig.hex,
    connected: true,
    last_seen: Date.now()
  });

  setupPresence(currentRoomId, myPlayerId);
  return roomData;
}

export function listenToRoom(onStateUpdate, onPlayersUpdate) {
  if (!firebaseDb || !currentRoomId) return;

  const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);

  if (roomStateListener) {
    roomRef.off('value', roomStateListener);
  }

  roomStateListener = (snapshot) => {
    if (!snapshot.exists()) return;
    const data = snapshot.val();
    if (data.state_json && onStateUpdate) {
      try {
        const state = JSON.parse(data.state_json);
        onStateUpdate(state);
      } catch (err) {
        console.error('Failed to parse remote game state:', err);
      }
    }
    if (data.players && onPlayersUpdate) {
      onPlayersUpdate(data.players);
    }
  };

  roomRef.on('value', roomStateListener);
}

export async function broadcastState(gameState) {
  if (!firebaseDb || !currentRoomId) return;
  try {
    const roomRef = firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`);
    await roomRef.update({
      state_json: JSON.stringify(gameState),
      last_updated: Date.now()
    });
  } catch (err) {
    console.error('Failed to broadcast game state:', err);
  }
}

export function leaveRoom() {
  if (firebaseDb && currentRoomId) {
    if (roomStateListener) {
      firebaseDb.ref(`carcassonne_rooms/${currentRoomId}`).off('value', roomStateListener);
      roomStateListener = null;
    }
    if (myPlayerId) {
      firebaseDb.ref(`carcassonne_rooms/${currentRoomId}/players/${myPlayerId}/connected`).set(false);
    }
  }
  currentRoomId = null;
  myPlayerId = null;
  isHost = false;
}

function setupPresence(roomId, playerId) {
  if (!firebaseDb) return;
  const connectedRef = firebaseDb.ref('.info/connected');
  const myPresenceRef = firebaseDb.ref(`carcassonne_rooms/${roomId}/players/${playerId}/connected`);

  connectedRef.on('value', (snap) => {
    if (snap.val() === true) {
      myPresenceRef.onDisconnect().set(false);
      myPresenceRef.set(true);
    }
  });
}
