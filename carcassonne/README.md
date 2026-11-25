# Carcassonne Web Tabletop (Rust + WebAssembly & Firebase)

A modern, web-deployable implementation of **Carcassonne** built in **Rust** (compiled to WebAssembly), deployable to **GitHub Pages**, with real-time multiplayer room synchronization via **Firebase Realtime Database**.

---

## Highlights

- **Rust Game Engine (`carcassonne-engine`)**:
  - Full game state, rotation geometry, and placement legality checks.
  - Complete graph topology engine: Breadth-First Search (BFS) traversals for Roads, Cities (with coat-of-arms shield scoring), Monasteries ($3 \times 3$ grid coverage), and **Grass Fields / Farms** (supporting 8 perimeter contact nodes + central tile node `CENTER`).
  - Unit-tested with `cargo test`.
  - Compiled to WebAssembly using `wasm-pack` with zero runtime dependencies.
- **Dynamic Tile Bag Catalog (`assets/tile_bag.json`)**:
  - Central specification defining all 15 initial tile variants (42 tiles).
  - Explicit edge graphs (`[North, East, South, West]`), internal features, and grass connection topology.
  - Easily extensible to the full classic 24 base variants and future expansions.
- **100% Modular Vector Graphics (`/assets`)**:
  - All visual assets (15 tile designs, leather tile bag, wooden meeple, and heraldic shield) are standalone SVG vector graphics in `assets/tiles/` and `assets/ui/`.
  - Drop in any custom vector art to instantly reskin the game without touching any code!
- **Tactile Tabletop UI**:
  - Medieval parchment & wood aesthetic styled with Tailwind CSS, Cinzel & Inter typography.
  - Full-screen pan and pinch-to-zoom interactive camera.
  - Animated tile draw from the bottom-right leather pouch.
  - Web Audio synthesizer generating procedural sound effects for tile rotations, placement clicks, meeple drops, and fanfare chords.
- **Online Multiplayer (Firebase) & Offline Pass-and-Play**:
  - Works out-of-the-box in local pass-and-play mode with zero keys or configuration.
  - Optional real-time online room hosting (`?room=XYZ`) and live action synchronization via Firebase Realtime Database.
- **GitHub Pages CI/CD**:
  - Automated deployment workflow in `.github/workflows/deploy.yml`.

---

## Directory Structure

```
carcassonne/
├── Cargo.toml                  # Rust crate configuration (cdylib + rlib)
├── src/                        # Rust Core Engine & WASM Bridge
│   ├── lib.rs                  # Module declarations and re-exports
│   ├── models.rs               # Tile, Edge, Feature, Grass/Field, Player, GameState structs
│   ├── geometry.rs             # 2D grid coordinates, 90° rotations, edge/sub-edge mappings
│   ├── graph.rs                # BFS graph traversals for roads, cities, monasteries, and farms
│   ├── scoring.rs              # In-turn completion scoring & end-game farming evaluation
│   ├── engine.rs               # Game turn state machine and placement rules
│   └── wasm_api.rs             # wasm-bindgen JavaScript bindings
├── tests/
│   └── engine_tests.rs         # Automated test suite for game rules & tile bag parsing
├── pkg/                        # Generated WebAssembly binary & JS glue code (wasm-pack)
├── assets/
│   ├── tile_bag.json           # Master tile bag specification & topology
│   ├── ui/
│   │   ├── tile_bag.svg        # Medieval leather drawstring pouch
│   │   ├── meeple.svg          # Classic wooden meeple silhouette
│   │   └── shield.svg          # Heraldic coat-of-arms shield
│   └── tiles/                  # 15 standalone SVG vector tile graphics
│       ├── start_tile.svg
│       ├── road_straight.svg
│       ├── road_curve.svg
│       ├── road_t.svg
│       ├── road_cross.svg
│       ├── monastery.svg
│       ├── monastery_road.svg
│       ├── city_cap.svg
│       ├── city_corner.svg
│       ├── city_corner_shield.svg
│       ├── city_bridge.svg
│       ├── city_bridge_shield.svg
│       ├── city_full.svg
│       ├── city_cap_road_straight.svg
│       └── city_cap_road_curve.svg
├── css/
│   └── style.css               # Parchment tabletop styling & animations
├── js/
│   ├── app.js                  # Tabletop orchestrator (viewport, drag/drop, WASM bridge)
│   ├── audio.js                # Tactile Web Audio synthesizer
│   └── firebase-sync.js        # Firebase Realtime Database room lobby & sync
├── index.html                  # Main responsive web layout
└── .github/workflows/
    └── deploy.yml              # Automated GitHub Pages CI/CD
```

---

## Quickstart & Local Development

### 1. Prerequisites
- **Rust** ($1.70+$) with `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **wasm-pack** or **Node.js**:
  ```bash
  npm install -g wasm-pack
  # OR run directly via: npx -y wasm-pack
  ```

### 2. Run Automated Tests
```bash
cargo test
```

### 3. Build WebAssembly Bundle
```bash
wasm-pack build --target web --release
# OR via npx:
npx -y wasm-pack build --target web --release
```

### 4. Serve Locally
Because ES Modules and WebAssembly require a local web server (cannot be opened via `file://`), run any static HTTP server:
```bash
# Using Python:
python3 -m http.server 8080

# OR using Node / npx:
npx serve .
```
Open [http://localhost:8080](http://localhost:8080) in your browser.

---

## Tile Bag Specification (`assets/tile_bag.json`)

The tile bag is defined in a structured JSON schema:

```json
{
  "id": "start_tile",
  "name": "Starting Tile (City & Road)",
  "count": 1,
  "is_start": true,
  "asset": "assets/tiles/start_tile.svg",
  "edges": ["C", "R", "F", "R"],
  "features": [
    { "id": "c0", "type": "city", "edges": [0], "shield": false, "hotspot": { "x": 50, "y": 16 } },
    { "id": "r0", "type": "road", "edges": [1, 3], "shield": false, "hotspot": { "x": 50, "y": 54 } }
  ],
  "fields": [
    {
      "id": "f0",
      "nodes": ["WN", "EN"],
      "adjacent_cities": ["c0"],
      "hotspot": { "x": 50, "y": 38 }
    },
    {
      "id": "f1",
      "nodes": ["WS", "SW", "SE", "ES", "CENTER"],
      "adjacent_cities": [],
      "hotspot": { "x": 50, "y": 78 }
    }
  ]
}
```

### Grass Topology & Center Node
Each square tile has 4 edges ($0: \text{North}, 1: \text{East}, 2: \text{South}, 3: \text{West}$).
Fields connect across tile borders via 8 boundary contact points:
- North: `NW`, `NE`
- East: `EN`, `ES`
- South: `SE`, `SW`
- West: `WS`, `WN`
- Center: `CENTER` (represents the interior meadow body, allowing fields to connect through tiles or anchor special structures)

At game end, each connected farm scores **3 points per unique adjacent completed city** for the player(s) with the most farmers.

---

## Customizing Vector Graphics

All graphics are standard standalone SVG files (`viewBox="0 0 100 100"`). To replace any tile or UI graphic:
1. Open or export your SVG vector design.
2. Ensure edge connections match (roads 14px outer/10px inner centered at 50 on respective edges).
3. Save the file over the corresponding SVG in `assets/tiles/<name>.svg` or `assets/ui/<name>.svg`.
4. Refresh your browser! No code compilation needed.

---

## Firebase Multiplayer Setup

1. In the game, click the top-left **Carcassonne** menu and select **Online Multiplayer (Firebase)**.
2. Expand **Firebase Connection Settings**.
3. Paste your Firebase project configuration JSON:
   ```json
   {
     "apiKey": "your-api-key",
     "authDomain": "your-project.firebaseapp.com",
     "databaseURL": "https://your-project-default-rtdb.firebaseio.com",
     "projectId": "your-project-id"
   }
   ```
4. Click **Save Firebase Config**. Config is saved in your browser's `localStorage`.
5. Click **Create Room** to generate a room code (or share URL `?room=XYZ`) and play with friends in real time!
