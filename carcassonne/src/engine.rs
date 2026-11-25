use std::collections::{HashMap, HashSet};
use crate::geometry::{coord_key, opposite_dir, rotate_point, world_to_edge, DIRS};
use crate::graph::{analyze_field, analyze_road_or_city};
use crate::models::{
    BoardTile, FeatureType, GamePhase, GameState, LogEntry, Meeple, MeepleKind, MeepleSpot,
    Player, ScoreEvent, TileDefinition, TradeGoods,
};
use crate::scoring::{evaluate_end_game_scoring, evaluate_turn_completions};

pub struct CarcassonneEngine {
    pub catalog: HashMap<String, TileDefinition>,
    pub state: GameState,
}

impl CarcassonneEngine {
    pub fn new(
        tiles: Vec<TileDefinition>,
        mut players: Vec<Player>,
        farmers_enabled: bool,
        river_enabled: bool,
        inns_cathedrals_enabled: bool,
        traders_builders_enabled: bool,
    ) -> Self {
        let mut catalog = HashMap::new();
        let mut deck = Vec::new();
        let mut river_intermediate = Vec::new();
        let mut start_type_id = String::from("start_tile");
        let mut spring_type_id: Option<String> = None;
        let mut lake_type_id: Option<String> = None;

        for tile in tiles {
            let include_in_deck = match tile.expansion.as_deref() {
                None | Some("base") => true,
                Some("inns_cathedrals") => inns_cathedrals_enabled,
                Some("traders_builders") => traders_builders_enabled,
                _ => false,
            };

            if tile.is_spring {
                spring_type_id = Some(tile.id.clone());
            } else if tile.is_lake {
                lake_type_id = Some(tile.id.clone());
            } else if tile.is_river {
                for _ in 0..tile.count {
                    river_intermediate.push(tile.id.clone());
                }
            } else if tile.is_start {
                start_type_id = tile.id.clone();
            } else if include_in_deck {
                for _ in 0..tile.count {
                    deck.push(tile.id.clone());
                }
            }
            catalog.insert(tile.id.clone(), tile);
        }

        for p in &mut players {
            p.has_big_meeple = inns_cathedrals_enabled;
            p.has_builder = traders_builders_enabled;
            p.has_pig = traders_builders_enabled;
            p.goods = TradeGoods::default();
        }

        // Shuffle decks
        fast_shuffle(&mut deck);
        fast_shuffle(&mut river_intermediate);

        let actually_use_river = river_enabled && spring_type_id.is_some() && lake_type_id.is_some();
        let mut river_deck = Vec::new();
        let initial_start_id = if actually_use_river {
            // Lake goes at the bottom of the river stack (index 0)
            river_deck.push(lake_type_id.unwrap());
            // Shuffled intermediate tiles go on top
            river_deck.extend(river_intermediate);
            spring_type_id.unwrap()
        } else {
            start_type_id
        };

        let mut board = HashMap::new();
        board.insert(
            coord_key(0, 0),
            BoardTile {
                type_id: initial_start_id.clone(),
                rotation: 0,
                x: 0,
                y: 0,
                meeple: None,
            },
        );

        let initial_log_text = if actually_use_river {
            format!(
                "The River expansion active: River Spring placed at (0, 0). {} river tiles to place before main deck.",
                river_deck.len()
            )
        } else {
            "Game started with starting tile placed at (0, 0).".to_string()
        };

        let mut engine = Self {
            catalog,
            state: GameState {
                phase: GamePhase::PlaceTile,
                players,
                current_player_index: 0,
                board,
                deck,
                river_deck,
                is_river_phase: actually_use_river,
                river_enabled: actually_use_river,
                last_river_turn: None,
                current_tile_type_id: None,
                current_rotation: 0,
                last_placed_coord: Some(coord_key(0, 0)),
                turn_number: 1,
                log: vec![LogEntry {
                    id: 1.0,
                    text: initial_log_text,
                    tone: "info".to_string(),
                }],
                farmers_enabled,
                inns_cathedrals_enabled,
                traders_builders_enabled,
                builder_bonus_pending: false,
                is_builder_bonus_turn: false,
            },
        };

        engine.draw_next_playable_tile();
        engine
    }

    pub fn can_place_tile_at(&self, x: i32, y: i32, type_id: &str, rotation: u8) -> bool {
        let key = coord_key(x, y);
        if self.state.board.contains_key(&key) {
            return false;
        }

        let def = match self.catalog.get(type_id) {
            Some(d) => d,
            None => return false,
        };

        let mut has_neighbor = false;
        let mut river_connection: Option<usize> = None; // world direction of river connection
        let mut river_edge_count = 0;

        for dir in 0..4 {
            let my_unrot_edge = world_to_edge(dir, rotation);
            let my_edge = def.edges[my_unrot_edge];
            if my_edge == crate::models::EdgeType::River {
                river_edge_count += 1;
            }

            let (dx, dy) = DIRS[dir];
            let nx = x + dx;
            let ny = y + dy;
            let n_key = coord_key(nx, ny);

            if let Some(neighbor) = self.state.board.get(&n_key) {
                has_neighbor = true;

                let opp_dir = opposite_dir(dir);
                let n_unrot_edge = world_to_edge(opp_dir, neighbor.rotation);
                let n_def = match self.catalog.get(&neighbor.type_id) {
                    Some(nd) => nd,
                    None => return false,
                };
                let n_edge = n_def.edges[n_unrot_edge];

                if my_edge != n_edge {
                    return false;
                }

                if my_edge == crate::models::EdgeType::River && n_edge == crate::models::EdgeType::River {
                    river_connection = Some(dir);
                }
            }
        }

        if !has_neighbor {
            return false;
        }

        // River-specific rules:
        if def.is_river {
            // A river tile with river edges must connect to an existing river edge
            if river_edge_count > 0 && river_connection.is_none() {
                return false;
            }

            // U-Turn Prevention Rule:
            // The river cannot turn in the same direction twice in a row.
            if let Some(in_dir) = river_connection {
                let other_river_edge = (0..4).find(|&d| {
                    if d == in_dir {
                        return false;
                    }
                    let unrot = world_to_edge(d, rotation);
                    def.edges[unrot] == crate::models::EdgeType::River
                });

                if let Some(out_dir) = other_river_edge {
                    let flow_forward_dir = (in_dir + 2) % 4;
                    let turn: Option<i8> = if out_dir == flow_forward_dir {
                        None // Straight
                    } else if out_dir == (flow_forward_dir + 1) % 4 {
                        Some(1) // Clockwise / Right turn
                    } else if out_dir == (flow_forward_dir + 3) % 4 {
                        Some(-1) // Counter-clockwise / Left turn
                    } else {
                        None
                    };

                    if let Some(t) = turn {
                        if self.state.last_river_turn == Some(t) {
                            return false; // Immediate U-turn forbidden!
                        }
                    }
                }
            }
        }

        true
    }

    pub fn get_valid_placements(&self, type_id: &str, rotation: u8) -> Vec<(i32, i32)> {
        let mut candidates = HashSet::new();

        for tile in self.state.board.values() {
            for dir in 0..4 {
                let (dx, dy) = DIRS[dir];
                let nx = tile.x + dx;
                let ny = tile.y + dy;
                let n_key = coord_key(nx, ny);
                if !self.state.board.contains_key(&n_key) {
                    candidates.insert((nx, ny));
                }
            }
        }

        let mut valid = Vec::new();
        for (x, y) in candidates {
            if self.can_place_tile_at(x, y, type_id, rotation) {
                valid.push((x, y));
            }
        }
        valid
    }

    pub fn can_place_tile_anywhere(&self, type_id: &str) -> bool {
        for r in 0..4 {
            if !self.get_valid_placements(type_id, r).is_empty() {
                return true;
            }
        }
        false
    }

    pub fn draw_next_playable_tile(&mut self) -> Option<String> {
        if self.state.is_river_phase {
            while let Some(next_id) = self.state.river_deck.pop() {
                if self.can_place_tile_anywhere(&next_id) {
                    self.state.current_tile_type_id = Some(next_id.clone());
                    self.state.current_rotation = 0;
                    self.state.phase = GamePhase::PlaceTile;
                    return Some(next_id);
                } else {
                    let name = self
                        .catalog
                        .get(&next_id)
                        .map(|d| d.name.as_str())
                        .unwrap_or("River tile");
                    self.add_log(
                        format!("Discarded \"{}\" — no valid river placement on board.", name),
                        "warn",
                    );
                }
            }

            // River deck is exhausted -> transition to regular deck!
            self.state.is_river_phase = false;
            self.add_log(
                "--- River completed! Beginning standard tile draw from the bag. ---".to_string(),
                "info",
            );
        }

        while let Some(next_id) = self.state.deck.pop() {
            if self.can_place_tile_anywhere(&next_id) {
                self.state.current_tile_type_id = Some(next_id.clone());
                self.state.current_rotation = 0;
                self.state.phase = GamePhase::PlaceTile;
                return Some(next_id);
            } else {
                let name = self
                    .catalog
                    .get(&next_id)
                    .map(|d| d.name.as_str())
                    .unwrap_or("Tile");
                self.add_log(
                    format!("Discarded \"{}\" — no valid placement on board.", name),
                    "warn",
                );
            }
        }

        // Bag is empty -> trigger game over
        self.state.current_tile_type_id = None;
        self.trigger_game_over();
        None
    }

    pub fn trigger_game_over(&mut self) -> Vec<ScoreEvent> {
        self.state.phase = GamePhase::GameOver;
        self.add_log("--- Final End-Game Scoring ---".to_string(), "info");
        let events = evaluate_end_game_scoring(&mut self.state, &self.catalog);
        for ev in &events {
            self.add_log(
                format!("{} scored +{} pts ({})", ev.player_names.join(" & "), ev.points, ev.description),
                "score",
            );
        }
        events
    }

    pub fn place_tile(&mut self, x: i32, y: i32, rotation: u8) -> Result<bool, String> {
        if self.state.phase != GamePhase::PlaceTile {
            return Err("Not in tile placement phase".to_string());
        }

        let type_id = self
            .state
            .current_tile_type_id
            .clone()
            .ok_or_else(|| "No current tile to place".to_string())?;

        if !self.can_place_tile_at(x, y, &type_id, rotation) {
            return Err(format!("Illegal placement at ({}, {}) with rotation {}", x, y, rotation));
        }

        // Update River U-turn state if this is a river tile
        if let Some(def) = self.catalog.get(&type_id) {
            if def.is_river {
                let mut river_conn_dir = None;
                for dir in 0..4 {
                    let (dx, dy) = DIRS[dir];
                    let n_key = coord_key(x + dx, y + dy);
                    if let Some(neighbor) = self.state.board.get(&n_key) {
                        let my_unrot = world_to_edge(dir, rotation);
                        let n_unrot = world_to_edge(opposite_dir(dir), neighbor.rotation);
                        if let Some(n_def) = self.catalog.get(&neighbor.type_id) {
                            if def.edges[my_unrot] == crate::models::EdgeType::River
                                && n_def.edges[n_unrot] == crate::models::EdgeType::River
                            {
                                river_conn_dir = Some(dir);
                                break;
                            }
                        }
                    }
                }

                if let Some(in_dir) = river_conn_dir {
                    let other_river_edge = (0..4).find(|&d| {
                        if d == in_dir {
                            return false;
                        }
                        let unrot = world_to_edge(d, rotation);
                        def.edges[unrot] == crate::models::EdgeType::River
                    });

                    if let Some(out_dir) = other_river_edge {
                        let flow_forward_dir = (in_dir + 2) % 4;
                        if out_dir == (flow_forward_dir + 1) % 4 {
                            self.state.last_river_turn = Some(1);
                        } else if out_dir == (flow_forward_dir + 3) % 4 {
                            self.state.last_river_turn = Some(-1);
                        } else {
                            self.state.last_river_turn = None;
                        }
                    } else {
                        // Lake (no outflow)
                        self.state.last_river_turn = None;
                    }
                }
            }
        }

        let key = coord_key(x, y);
        self.state.board.insert(
            key.clone(),
            BoardTile {
                type_id: type_id.clone(),
                rotation,
                x,
                y,
                meeple: None,
            },
        );
        self.state.last_placed_coord = Some(key.clone());

        let player_id = self.current_player().id.clone();
        let player_name = self.current_player().name.clone();
        let tile_name = self.catalog.get(&type_id).map(|d| d.name.clone()).unwrap_or_else(|| type_id.clone());
        self.add_log(
            format!("{} placed {} at ({}, {}).", player_name, tile_name, x, y),
            "action",
        );

        // Builder bonus turn check: if Traders & Builders is enabled and this is NOT already a bonus turn,
        // check whether the just-placed tile extends any road or city that already contains the current player's Builder.
        if self.state.traders_builders_enabled && !self.state.is_builder_bonus_turn {
            if let Some(def) = self.catalog.get(&type_id) {
                let mut triggers_builder = false;
                for feat in &def.features {
                    if feat.feature_type == FeatureType::Road || feat.feature_type == FeatureType::City {
                        if let Some(analysis) = analyze_road_or_city(x, y, &feat.id, &self.state.board, &self.catalog) {
                            if analysis.meeples.iter().any(|m| m.player_id == player_id && m.kind == MeepleKind::Builder) {
                                triggers_builder = true;
                                break;
                            }
                        }
                    }
                }
                if triggers_builder {
                    self.state.builder_bonus_pending = true;
                }
            }
        }

        let valid_spots = self.get_valid_meeple_spots(x, y);

        if !valid_spots.is_empty() {
            self.state.phase = GamePhase::PlaceMeeple;
            Ok(true) // Needs meeple decision
        } else {
            self.finish_turn(x, y);
            Ok(false) // Turn finished immediately
        }
    }

    pub fn get_valid_meeple_spots(&self, x: i32, y: i32) -> Vec<MeepleSpot> {
        let key = coord_key(x, y);
        let tile = match self.state.board.get(&key) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let def = match self.catalog.get(&tile.type_id) {
            Some(d) => d,
            None => return Vec::new(),
        };

        let mut spots = Vec::new();
        let player = self.current_player();

        // Features (Cities, Roads, Monasteries)
        for feat in &def.features {
            let mut allowed_kinds = Vec::new();
            match feat.feature_type {
                FeatureType::Monastery => {
                    if player.meeples > 0 {
                        allowed_kinds.push(MeepleKind::Normal);
                    }
                    if self.state.inns_cathedrals_enabled && player.has_big_meeple {
                        allowed_kinds.push(MeepleKind::Big);
                    }
                }
                FeatureType::Road | FeatureType::City => {
                    let meeples = analyze_road_or_city(x, y, &feat.id, &self.state.board, &self.catalog)
                        .map(|a| a.meeples)
                        .unwrap_or_default();

                    if meeples.is_empty() {
                        if player.meeples > 0 {
                            allowed_kinds.push(MeepleKind::Normal);
                        }
                        if self.state.inns_cathedrals_enabled && player.has_big_meeple {
                            allowed_kinds.push(MeepleKind::Big);
                        }
                    } else if self.state.traders_builders_enabled && player.has_builder {
                        let has_own_follower = meeples.iter().any(|m| {
                            m.player_id == player.id
                                && (m.kind == MeepleKind::Normal || m.kind == MeepleKind::Big)
                        });
                        if has_own_follower {
                            allowed_kinds.push(MeepleKind::Builder);
                        }
                    }
                }
            }

            if !allowed_kinds.is_empty() {
                let rot_hotspot = rotate_point(&feat.hotspot, tile.rotation);
                let type_name = match feat.feature_type {
                    FeatureType::City => "city",
                    FeatureType::Road => "road",
                    FeatureType::Monastery => "monastery",
                };
                spots.push(MeepleSpot {
                    id: feat.id.clone(),
                    spot_type: type_name.to_string(),
                    hotspot: rot_hotspot,
                    is_farmer: false,
                    allowed_kinds,
                });
            }
        }

        // Fields (Farms)
        if self.state.farmers_enabled {
            for field in &def.fields {
                let mut allowed_kinds = Vec::new();
                let meeples = analyze_field(x, y, &field.id, &self.state.board, &self.catalog)
                    .map(|a| a.meeples)
                    .unwrap_or_default();

                if meeples.is_empty() {
                    if player.meeples > 0 {
                        allowed_kinds.push(MeepleKind::Normal);
                    }
                    if self.state.inns_cathedrals_enabled && player.has_big_meeple {
                        allowed_kinds.push(MeepleKind::Big);
                    }
                } else if self.state.traders_builders_enabled && player.has_pig {
                    let has_own_farmer = meeples.iter().any(|m| {
                        m.player_id == player.id
                            && (m.kind == MeepleKind::Normal || m.kind == MeepleKind::Big)
                    });
                    if has_own_farmer {
                        allowed_kinds.push(MeepleKind::Pig);
                    }
                }

                if !allowed_kinds.is_empty() {
                    let rot_hotspot = rotate_point(&field.hotspot, tile.rotation);
                    spots.push(MeepleSpot {
                        id: field.id.clone(),
                        spot_type: "field".to_string(),
                        hotspot: rot_hotspot,
                        is_farmer: true,
                        allowed_kinds,
                    });
                }
            }
        }

        spots
    }

    pub fn place_meeple(
        &mut self,
        feature_id: Option<String>,
        meeple_kind: MeepleKind,
    ) -> Result<Vec<ScoreEvent>, String> {
        if self.state.phase != GamePhase::PlaceMeeple {
            return Err("Not in meeple placement phase".to_string());
        }

        let coord = self
            .state
            .last_placed_coord
            .clone()
            .ok_or_else(|| "No last placed coordinate".to_string())?;

        let (x, y) = crate::geometry::parse_coord_key(&coord)
            .ok_or_else(|| "Invalid coordinate key".to_string())?;

        if let Some(feat_id) = feature_id {
            let valid_spots = self.get_valid_meeple_spots(x, y);
            let spot = valid_spots
                .iter()
                .find(|s| s.id == feat_id)
                .ok_or_else(|| format!("Invalid meeple target feature: {}", feat_id))?;

            if !spot.allowed_kinds.contains(&meeple_kind) {
                return Err(format!(
                    "Meeple kind {:?} is not allowed on feature {}",
                    meeple_kind, feat_id
                ));
            }

            let player_id = self.current_player().id.clone();
            let player_name = self.current_player().name.clone();

            match meeple_kind {
                MeepleKind::Normal => {
                    if self.current_player().meeples == 0 {
                        return Err("Player has no meeples remaining".to_string());
                    }
                    self.current_player_mut().meeples -= 1;
                }
                MeepleKind::Big => {
                    if !self.current_player().has_big_meeple {
                        return Err("Player has no big meeple remaining".to_string());
                    }
                    self.current_player_mut().has_big_meeple = false;
                }
                MeepleKind::Builder => {
                    if !self.current_player().has_builder {
                        return Err("Player has no builder remaining".to_string());
                    }
                    self.current_player_mut().has_builder = false;
                }
                MeepleKind::Pig => {
                    if !self.current_player().has_pig {
                        return Err("Player has no pig remaining".to_string());
                    }
                    self.current_player_mut().has_pig = false;
                }
            }

            if let Some(tile) = self.state.board.get_mut(&coord) {
                tile.meeple = Some(Meeple {
                    player_id,
                    feature_id: feat_id.clone(),
                    is_farmer: spot.is_farmer,
                    kind: meeple_kind,
                });
            }

            let role_label = match (meeple_kind, spot.is_farmer) {
                (MeepleKind::Normal, true) => "farmer in a meadow".to_string(),
                (MeepleKind::Normal, false) => format!("meeple on a {}", spot.spot_type),
                (MeepleKind::Big, true) => "big farmer in a meadow".to_string(),
                (MeepleKind::Big, false) => format!("big meeple on a {}", spot.spot_type),
                (MeepleKind::Builder, _) => format!("builder on a {}", spot.spot_type),
                (MeepleKind::Pig, _) => "pig in a meadow".to_string(),
            };
            self.add_log(
                format!("{} placed a {}.", player_name, role_label),
                "action",
            );
        } else {
            let player_name = self.current_player().name.clone();
            self.add_log(format!("{} skipped placing a meeple.", player_name), "neutral");
        }

        Ok(self.finish_turn(x, y))
    }

    fn finish_turn(&mut self, x: i32, y: i32) -> Vec<ScoreEvent> {
        // Award Trader goods for any city completed by this tile placement to the tile-placer
        if self.state.traders_builders_enabled {
            let placed_key = coord_key(x, y);
            let def_opt = self
                .state
                .board
                .get(&placed_key)
                .and_then(|t| self.catalog.get(&t.type_id))
                .cloned();

            if let Some(def) = def_opt {
                let mut checked_cities: HashSet<String> = HashSet::new();
                for feat in &def.features {
                    if feat.feature_type != FeatureType::City {
                        continue;
                    }
                    let node_key = format!("{}|{}", placed_key, feat.id);
                    if checked_cities.contains(&node_key) {
                        continue;
                    }
                    if let Some(analysis) =
                        analyze_road_or_city(x, y, &feat.id, &self.state.board, &self.catalog)
                    {
                        for v in &analysis.visited_nodes {
                            checked_cities.insert(v.clone());
                        }
                        if analysis.is_complete() {
                            let g = analysis.goods;
                            if g.wine > 0 || g.grain > 0 || g.cloth > 0 {
                                let player_name = {
                                    let p = self.current_player_mut();
                                    p.goods.wine += g.wine;
                                    p.goods.grain += g.grain;
                                    p.goods.cloth += g.cloth;
                                    p.name.clone()
                                };
                                let mut parts = Vec::new();
                                if g.wine > 0 {
                                    parts.push(format!("{} wine", g.wine));
                                }
                                if g.grain > 0 {
                                    parts.push(format!("{} grain", g.grain));
                                }
                                if g.cloth > 0 {
                                    parts.push(format!("{} cloth", g.cloth));
                                }
                                self.add_log(
                                    format!(
                                        "{} collected trade goods: {}.",
                                        player_name,
                                        parts.join(", ")
                                    ),
                                    "score",
                                );
                            }
                        }
                    }
                }
            }
        }

        let score_events = evaluate_turn_completions(x, y, &mut self.state, &self.catalog);

        for ev in &score_events {
            self.add_log(
                format!("{} scored +{} pts ({})", ev.player_names.join(" & "), ev.points, ev.description),
                "score",
            );
        }

        let has_remaining_tiles = !self.state.deck.is_empty()
            || (self.state.is_river_phase && !self.state.river_deck.is_empty());

        if self.state.builder_bonus_pending && has_remaining_tiles {
            self.state.builder_bonus_pending = false;
            self.state.is_builder_bonus_turn = true;
            let player_name = self.current_player().name.clone();
            self.add_log(format!("{} gets a Builder bonus turn!", player_name), "info");
        } else {
            self.state.builder_bonus_pending = false;
            self.state.is_builder_bonus_turn = false;
            self.state.current_player_index =
                (self.state.current_player_index + 1) % self.state.players.len();
        }
        self.state.turn_number += 1;

        self.draw_next_playable_tile();
        score_events
    }

    pub fn current_player(&self) -> &Player {
        &self.state.players[self.state.current_player_index]
    }

    pub fn current_player_mut(&mut self) -> &mut Player {
        &mut self.state.players[self.state.current_player_index]
    }

    pub fn add_log(&mut self, text: String, tone: &str) {
        let id = self.state.log.len() as f64 + 1.0;
        self.state.log.insert(
            0,
            LogEntry {
                id,
                text,
                tone: tone.to_string(),
            },
        );
        if self.state.log.len() > 100 {
            self.state.log.pop();
        }
    }
}

fn fast_shuffle<T>(vec: &mut [T]) {
    let mut rng_state: u64 = 0x853c49e6748fea9b;
    let mut seed = [0u8; 8];
    if getrandom::getrandom(&mut seed).is_ok() {
        rng_state = u64::from_le_bytes(seed);
    }
    for i in (1..vec.len()).rev() {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (rng_state >> 33) as usize % (i + 1);
        vec.swap(i, j);
    }
}
