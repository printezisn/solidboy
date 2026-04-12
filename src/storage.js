const DATABASE_NAME = 'game-data';
const DATABASE_VERSION = 1;
const GAME_DATA_STORE = 'game-data';

const openDatabase = () => {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DATABASE_NAME, DATABASE_VERSION);

    request.onupgradeneeded = (event) => {
      const db = event.target.result;
      if (!db.objectStoreNames.contains(GAME_DATA_STORE)) {
        db.createObjectStore(GAME_DATA_STORE);
      }
    };

    request.onsuccess = (event) => resolve(event.target.result);
    request.onerror = (event) => reject(event.target.error);
  });
};

export const saveGameData = async (gameName, data) => {
  const db = await openDatabase();

  return new Promise((resolve, reject) => {
    const tx = db.transaction(GAME_DATA_STORE, 'readwrite');
    const store = tx.objectStore(GAME_DATA_STORE);
    const request = store.put(data, gameName);

    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
};

export const fetchGameData = async (gameName) => {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(GAME_DATA_STORE, 'readonly');
    const store = tx.objectStore(GAME_DATA_STORE);
    const request = store.get(gameName);

    request.onsuccess = () =>
      resolve(request.result || new Uint8ClampedArray());
    request.onerror = () => reject(request.error);
  });
};
