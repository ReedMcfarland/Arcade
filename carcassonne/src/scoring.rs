use std::collections::{HashMap, HashSet};
use crate::geometry::coord_key;
use crate::graph::{analyze_field, analyze_road_or_city, count_monastery_coverage, MeepleOnBoard};
use crate::models::{BoardTile, FeatureType, GameState, MeepleKind, ScoreEvent, TileDefinition, TradeGoods};

fn meeple_weight(kind: MeepleKind) -> usize {
    match kind {
        MeepleKind::Normal => 1,
        MeepleKind::Big => 2,
        MeepleKind::Builder | MeepleKind::Pig => 0,
    }
}

fn majority_winner_ids(meeples: &[MeepleOnBoard]) -> Vec<String> {
    let mut weights: HashMap<String, usize> = HashMap::new();
    let mut max_weight = 0;
    for m in meeples {
        let w = meeple_weight(m.kind);
        if w > 0 {
            let total = weights.entry(m.player_id.clone()).or_insert(0);
            *total += w;
            if *total > max_weight {
                max_weight = *total;
            }
        }
    }
    if max_weight == 0 {
        return Vec::new();
    }
    weights
        .into_iter()
        .filter(|&(_, w)| w == max_weight)
        .map(|(id, _)| id)
        .collect()
}

fn return_feature_meeples(meeples: &[MeepleOnBoard], state: &mut GameState) {
    for m in meeples {
        if let Some(tile) = state.board.get_mut(&m.coord_key) {
            if let Some(current_meeple) = &tile.meeple {
                if current_meeple.feature_id == m.feature_id {
                    if let Some(player) = state.players.iter_mut().find(|p| p.id == m.player_id) {
                        match current_meeple.kind {
                            MeepleKind::Normal => player.meeples += 1,
                            MeepleKind::Big => player.has_big_meeple = true,
                            MeepleKind::Builder => player.has_builder = true,
                            MeepleKind::Pig => player.has_pig = true,
                        }
                    }
                    tile.meeple = None;
                }
            }
        }
    }
}

/// Helper to award points to players with the maximum meeple weight on a feature
pub fn award_points(
    meeples: &[MeepleOnBoard],
    points: i32,
    description: &str,
    anchor_x: i32,
    anchor_y: i32,
    state: &mut GameState,
    return_meeples: bool,
) -> Option<ScoreEvent> {
    if meeples.is_empty() {
        return None;
    }

    if return_meeples {
        return_feature_meeples(meeples, state);
    }

    if points <= 0 {
        return None;
    }

    let winner_ids = majority_winner_ids(meeples);
    if winner_ids.is_empty() {
        return None;
    }

    let mut winner_names = Vec::new();
    let mut winner_hex = String::from("#c99738");

    for pid in &winner_ids {
        if let Some(player) = state.players.iter_mut().find(|p| &p.id == pid) {
            player.score += points;
            winner_names.push(player.name.clone());
            winner_hex = player.hex.clone();
        }
    }

    Some(ScoreEvent {
        player_ids: winner_ids,
        player_names: winner_names,
        points,
        description: description.to_string(),
        x: anchor_x,
        y: anchor_y,
        hex: winner_hex,
    })
}

/// Evaluate completions directly following a tile/meeple placement turn
pub fn evaluate_turn_completions(
    placed_x: i32,
    placed_y: i32,
    state: &mut GameState,
    catalog: &HashMap<String, TileDefinition>,
) -> Vec<ScoreEvent> {
    let mut events = Vec::new();
    let placed_key = coord_key(placed_x, placed_y);

    let (type_id, _) = match state.board.get(&placed_key) {
        Some(t) => (t.type_id.clone(), t.rotation),
        None => return events,
    };
    let def = match catalog.get(&type_id) {
        Some(d) => d.clone(),
        None => return events,
    };

    let mut checked_features: HashSet<String> = HashSet::new();

    // 1. Check roads and cities touching the placed tile
    for feat in &def.features {
        if feat.feature_type == FeatureType::Road || feat.feature_type == FeatureType::City {
            let node_key = format!("{}|{}", placed_key, feat.id);
            if checked_features.contains(&node_key) {
                continue;
            }

            if let Some(analysis) = analyze_road_or_city(placed_x, placed_y, &feat.id, &state.board, catalog) {
                for v in &analysis.visited_nodes {
                    checked_features.insert(v.clone());
                }

                if analysis.is_complete() && !analysis.meeples.is_empty() {
                    let (pts, desc) = match analysis.feature_type {
                        FeatureType::Road => {
                            let per_tile = if analysis.has_inn { 2 } else { 1 };
                            let pts = (analysis.tiles.len() * per_tile) as i32;
                            let inn_desc = if analysis.has_inn { " (Inn)" } else { "" };
                            (pts, format!("Road{}, {} tiles", inn_desc, analysis.tiles.len()))
                        }
                        FeatureType::City => {
                            let per_unit = if analysis.has_cathedral { 3 } else { 2 };
                            let pts = ((analysis.tiles.len() + analysis.shields.len()) * per_unit) as i32;
                            let cathedral_desc = if analysis.has_cathedral { " (Cathedral)" } else { "" };
                            let shield_desc = if analysis.shields.is_empty() {
                                String::new()
                            } else {
                                format!(", {} shield(s)", analysis.shields.len())
                            };
                            (pts, format!("City{}, {} tiles{}", cathedral_desc, analysis.tiles.len(), shield_desc))
                        }
                        FeatureType::Monastery => (0, String::new()),
                    };

                    if let Some(ev) = award_points(
                        &analysis.meeples,
                        pts,
                        &desc,
                        placed_x,
                        placed_y,
                        state,
                        true,
                    ) {
                        events.push(ev);
                    }
                }
            }
        }
    }

    // 2. Check Monasteries on placed tile and surrounding 8 neighbors
    for dx in -1..=1 {
        for dy in -1..=1 {
            let mx = placed_x + dx;
            let my = placed_y + dy;
            let m_key = coord_key(mx, my);

            if let Some(m_tile) = state.board.get(&m_key).cloned() {
                if let Some(meeple) = &m_tile.meeple {
                    if let Some(m_def) = catalog.get(&m_tile.type_id) {
                        if let Some(m_feat) = m_def.features.iter().find(|f| f.id == meeple.feature_id) {
                            if m_feat.feature_type == FeatureType::Monastery {
                                let coverage = count_monastery_coverage(mx, my, &state.board);
                                if coverage == 9 {
                                    let meeple_list = vec![MeepleOnBoard {
                                        coord_key: m_key.clone(),
                                        x: mx,
                                        y: my,
                                        player_id: meeple.player_id.clone(),
                                        feature_id: meeple.feature_id.clone(),
                                        is_farmer: false,
                                        kind: meeple.kind,
                                    }];
                                    if let Some(ev) = award_points(
                                        &meeple_list,
                                        9,
                                        "Completed Monastery",
                                        mx,
                                        my,
                                        state,
                                        true,
                                    ) {
                                        events.push(ev);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    events
}

/// End-game scoring for incomplete roads, cities, monasteries, and farms
pub fn evaluate_end_game_scoring(
    state: &mut GameState,
    catalog: &HashMap<String, TileDefinition>,
) -> Vec<ScoreEvent> {
    let mut events = Vec::new();
    let mut scored_feature_nodes = HashSet::new();
    let mut scored_field_nodes = HashSet::new();

    let board_snapshot: Vec<(String, BoardTile)> = state
        .board
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // 1. Incomplete roads, cities, and monasteries
    for (key, tile) in &board_snapshot {
        if let Some(meeple) = &tile.meeple {
            let def = match catalog.get(&tile.type_id) {
                Some(d) => d,
                None => continue,
            };

            if let Some(feat) = def.features.iter().find(|f| f.id == meeple.feature_id) {
                if feat.feature_type == FeatureType::Monastery {
                    let pts = count_monastery_coverage(tile.x, tile.y, &state.board) as i32;
                    let meeple_list = vec![MeepleOnBoard {
                        coord_key: key.clone(),
                        x: tile.x,
                        y: tile.y,
                        player_id: meeple.player_id.clone(),
                        feature_id: meeple.feature_id.clone(),
                        is_farmer: false,
                        kind: meeple.kind,
                    }];
                    if let Some(ev) = award_points(
                        &meeple_list,
                        pts,
                        &format!("Incomplete Monastery ({}/9)", pts),
                        tile.x,
                        tile.y,
                        state,
                        false,
                    ) {
                        events.push(ev);
                    }
                } else if feat.feature_type == FeatureType::Road || feat.feature_type == FeatureType::City {
                    let node_key = format!("{}|{}", key, feat.id);
                    if scored_feature_nodes.contains(&node_key) {
                        continue;
                    }

                    if let Some(analysis) = analyze_road_or_city(tile.x, tile.y, &feat.id, &state.board, catalog) {
                        for v in &analysis.visited_nodes {
                            scored_feature_nodes.insert(v.clone());
                        }

                        let (pts, desc) = match analysis.feature_type {
                            FeatureType::Road => {
                                let pts = if analysis.has_inn { 0 } else { analysis.tiles.len() as i32 };
                                (pts, format!("Incomplete Road ({} tiles)", analysis.tiles.len()))
                            }
                            FeatureType::City => {
                                let pts = if analysis.has_cathedral {
                                    0
                                } else {
                                    (analysis.tiles.len() + analysis.shields.len()) as i32
                                };
                                (pts, format!("Incomplete City ({} tiles)", analysis.tiles.len()))
                            }
                            _ => (0, String::new()),
                        };

                        if let Some(ev) = award_points(
                            &analysis.meeples,
                            pts,
                            &desc,
                            tile.x,
                            tile.y,
                            state,
                            false,
                        ) {
                            events.push(ev);
                        }
                    }
                }
            }
        }
    }

    // 2. Field / Farm scoring
    if state.farmers_enabled {
        for (key, tile) in &board_snapshot {
            if let Some(meeple) = &tile.meeple {
                let def = match catalog.get(&tile.type_id) {
                    Some(d) => d,
                    None => continue,
                };

                if let Some(field) = def.fields.iter().find(|f| f.id == meeple.feature_id) {
                    let node_key = format!("{}|{}", key, field.id);
                    if scored_field_nodes.contains(&node_key) {
                        continue;
                    }

                    if let Some(analysis) = analyze_field(tile.x, tile.y, &field.id, &state.board, catalog) {
                        for v in &analysis.visited_nodes {
                            scored_field_nodes.insert(v.clone());
                        }

                        let completed_cities_count = analysis.adjacent_completed_city_keys.len() as i32;
                        if completed_cities_count > 0 {
                            let winner_ids = majority_winner_ids(&analysis.meeples);
                            let mut pig_winners = Vec::new();
                            let mut normal_winners = Vec::new();
                            for pid in winner_ids {
                                if analysis.pig_players.contains(&pid) {
                                    pig_winners.push(pid);
                                } else {
                                    normal_winners.push(pid);
                                }
                            }

                            for (group_ids, pts_per_city, pig_note) in [
                                (pig_winners, 4, " + Pig"),
                                (normal_winners, 3, ""),
                            ] {
                                if group_ids.is_empty() {
                                    continue;
                                }
                                let pts = completed_cities_count * pts_per_city;
                                let mut names = Vec::new();
                                let mut hex = String::from("#c99738");
                                for pid in &group_ids {
                                    if let Some(player) = state.players.iter_mut().find(|p| &p.id == pid) {
                                        player.score += pts;
                                        names.push(player.name.clone());
                                        hex = player.hex.clone();
                                    }
                                }
                                events.push(ScoreEvent {
                                    player_ids: group_ids,
                                    player_names: names,
                                    points: pts,
                                    description: format!(
                                        "Farm ({} adjacent completed cities{})",
                                        completed_cities_count, pig_note
                                    ),
                                    x: tile.x,
                                    y: tile.y,
                                    hex,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Trader goods majority scoring (end game)
    let goods_categories: [(&str, fn(&TradeGoods) -> u32); 3] = [
        ("Wine", |g| g.wine),
        ("Grain", |g| g.grain),
        ("Cloth", |g| g.cloth),
    ];
    for (label, getter) in goods_categories {
        let max_val = state.players.iter().map(|p| getter(&p.goods)).max().unwrap_or(0);
        if max_val > 0 {
            let mut winner_ids = Vec::new();
            let mut winner_names = Vec::new();
            let mut winner_hex = String::from("#c99738");
            for player in state.players.iter_mut() {
                if getter(&player.goods) == max_val {
                    player.score += 10;
                    winner_ids.push(player.id.clone());
                    winner_names.push(player.name.clone());
                    winner_hex = player.hex.clone();
                }
            }
            events.push(ScoreEvent {
                player_ids: winner_ids,
                player_names: winner_names,
                points: 10,
                description: format!("Trader Majority: {} ({})", label, max_val),
                x: 0,
                y: 0,
                hex: winner_hex,
            });
        }
    }

    events
}
