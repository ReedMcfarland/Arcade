use std::collections::{HashMap, HashSet, VecDeque};
use crate::geometry::{coord_key, matching_neighbor_field_nodes, opposite_dir, rotate_field_node, DIRS};
use crate::models::{BoardTile, FeatureType, MeepleKind, TileDefinition, TradeGoods};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeepleOnBoard {
    pub coord_key: String,
    pub x: i32,
    pub y: i32,
    pub player_id: String,
    pub feature_id: String,
    pub is_farmer: bool,
    pub kind: MeepleKind,
}

#[derive(Debug, Clone)]
pub struct FeatureAnalysis {
    pub feature_type: FeatureType,
    pub tiles: HashSet<String>,
    pub shields: HashSet<String>,
    pub meeples: Vec<MeepleOnBoard>,
    pub open_edges: usize,
    pub visited_nodes: HashSet<String>,
    pub has_inn: bool,
    pub has_cathedral: bool,
    pub goods: TradeGoods,
}

impl FeatureAnalysis {
    pub fn is_complete(&self) -> bool {
        self.open_edges == 0
    }
}

/// Analyze a road or city feature starting at (start_x, start_y, start_feature_id)
pub fn analyze_road_or_city(
    start_x: i32,
    start_y: i32,
    start_feature_id: &str,
    board: &HashMap<String, BoardTile>,
    catalog: &HashMap<String, TileDefinition>,
) -> Option<FeatureAnalysis> {
    let start_key = coord_key(start_x, start_y);
    let start_tile = board.get(&start_key)?;
    let start_def = catalog.get(&start_tile.type_id)?;
    let start_feat = start_def.features.iter().find(|f| f.id == start_feature_id)?;

    let mut visited_nodes = HashSet::new();
    let mut tiles = HashSet::new();
    let mut shields = HashSet::new();
    let mut meeples = Vec::new();
    let mut open_edges = 0;
    let mut has_inn = false;
    let mut has_cathedral = false;
    let mut goods = TradeGoods::default();

    let mut queue = VecDeque::new();
    let start_node_key = format!("{}|{}", start_key, start_feature_id);
    visited_nodes.insert(start_node_key);
    queue.push_back((start_x, start_y, start_feature_id.to_string()));

    while let Some((curr_x, curr_y, curr_feat_id)) = queue.pop_front() {
        let c_key = coord_key(curr_x, curr_y);
        let tile = match board.get(&c_key) {
            Some(t) => t,
            None => continue,
        };
        let def = match catalog.get(&tile.type_id) {
            Some(d) => d,
            None => continue,
        };
        let feat = match def.features.iter().find(|f| f.id == curr_feat_id) {
            Some(f) => f,
            None => continue,
        };

        tiles.insert(c_key.clone());
        if feat.shield {
            shields.insert(format!("{}|{}", c_key, feat.id));
        }
        if feat.has_inn {
            has_inn = true;
        }
        if feat.has_cathedral {
            has_cathedral = true;
        }
        goods.wine += feat.goods.wine;
        goods.grain += feat.goods.grain;
        goods.cloth += feat.goods.cloth;

        if let Some(m) = &tile.meeple {
            if m.feature_id == feat.id {
                meeples.push(MeepleOnBoard {
                    coord_key: c_key.clone(),
                    x: curr_x,
                    y: curr_y,
                    player_id: m.player_id.clone(),
                    feature_id: feat.id.clone(),
                    is_farmer: false,
                    kind: m.kind,
                });
            }
        }

        for &unrot_edge in &feat.edges {
            let world_dir = (unrot_edge + tile.rotation as usize) % 4;
            let (dx, dy) = DIRS[world_dir];
            let nx = curr_x + dx;
            let ny = curr_y + dy;
            let n_key = coord_key(nx, ny);

            match board.get(&n_key) {
                None => {
                    open_edges += 1;
                }
                Some(neighbor) => {
                    let opp_dir = opposite_dir(world_dir);
                    let n_unrot_edge = (opp_dir + 4 - (neighbor.rotation as usize % 4)) % 4;
                    if let Some(n_def) = catalog.get(&neighbor.type_id) {
                        if let Some(n_feat) = n_def
                            .features
                            .iter()
                            .find(|f| f.feature_type == feat.feature_type && f.edges.contains(&n_unrot_edge))
                        {
                            let node_key = format!("{}|{}", n_key, n_feat.id);
                            if !visited_nodes.contains(&node_key) {
                                visited_nodes.insert(node_key);
                                queue.push_back((nx, ny, n_feat.id.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    Some(FeatureAnalysis {
        feature_type: start_feat.feature_type,
        tiles,
        shields,
        meeples,
        open_edges,
        visited_nodes,
        has_inn,
        has_cathedral,
        goods,
    })
}

/// Count tiles surrounding a monastery (including center tile, max 9)
pub fn count_monastery_coverage(x: i32, y: i32, board: &HashMap<String, BoardTile>) -> usize {
    let mut count = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            if board.contains_key(&coord_key(x + dx, y + dy)) {
                count += 1;
            }
        }
    }
    count
}

#[derive(Debug, Clone)]
pub struct FieldAnalysis {
    pub tiles: HashSet<String>,
    pub meeples: Vec<MeepleOnBoard>,
    pub adjacent_completed_city_keys: HashSet<String>,
    pub visited_nodes: HashSet<String>,
    pub pig_players: HashSet<String>,
}

/// Analyze a field / grass farm starting at (start_x, start_y, start_field_id)
pub fn analyze_field(
    start_x: i32,
    start_y: i32,
    start_field_id: &str,
    board: &HashMap<String, BoardTile>,
    catalog: &HashMap<String, TileDefinition>,
) -> Option<FieldAnalysis> {
    let start_key = coord_key(start_x, start_y);
    let start_tile = board.get(&start_key)?;
    let start_def = catalog.get(&start_tile.type_id)?;
    let _ = start_def.fields.iter().find(|f| f.id == start_field_id)?;

    let mut visited_nodes = HashSet::new();
    let mut tiles = HashSet::new();
    let mut meeples = Vec::new();
    let mut adjacent_completed_city_keys = HashSet::new();
    let mut pig_players = HashSet::new();

    let mut queue = VecDeque::new();
    let start_node_key = format!("{}|{}", start_key, start_field_id);
    visited_nodes.insert(start_node_key);
    queue.push_back((start_x, start_y, start_field_id.to_string()));

    while let Some((curr_x, curr_y, curr_field_id)) = queue.pop_front() {
        let c_key = coord_key(curr_x, curr_y);
        let tile = match board.get(&c_key) {
            Some(t) => t,
            None => continue,
        };
        let def = match catalog.get(&tile.type_id) {
            Some(d) => d,
            None => continue,
        };
        let field = match def.fields.iter().find(|f| f.id == curr_field_id) {
            Some(f) => f,
            None => continue,
        };

        tiles.insert(c_key.clone());

        if let Some(m) = &tile.meeple {
            if m.feature_id == field.id {
                if m.kind == MeepleKind::Pig {
                    pig_players.insert(m.player_id.clone());
                }
                meeples.push(MeepleOnBoard {
                    coord_key: c_key.clone(),
                    x: curr_x,
                    y: curr_y,
                    player_id: m.player_id.clone(),
                    feature_id: field.id.clone(),
                    is_farmer: true,
                    kind: m.kind,
                });
            }
        }

        // Check bordering cities for completed status
        for city_id in &field.adjacent_cities {
            if let Some(city_analysis) = analyze_road_or_city(curr_x, curr_y, city_id, board, catalog) {
                if city_analysis.is_complete() {
                    let mut sorted_city_tiles: Vec<_> = city_analysis.tiles.into_iter().collect();
                    sorted_city_tiles.sort();
                    let city_key = sorted_city_tiles.join(";");
                    adjacent_completed_city_keys.insert(city_key);
                }
            }
        }

        // Cross tile edges to connected neighbor fields
        for world_dir in 0..4 {
            let neighbor_pairs = matching_neighbor_field_nodes(world_dir);
            let (dx, dy) = DIRS[world_dir];
            let nx = curr_x + dx;
            let ny = curr_y + dy;
            let n_key = coord_key(nx, ny);

            let neighbor = match board.get(&n_key) {
                Some(n) => n,
                None => continue,
            };
            let n_def = match catalog.get(&neighbor.type_id) {
                Some(nd) => nd,
                None => continue,
            };

            for &(my_node, n_expected_node) in neighbor_pairs {
                // Check if this field segment on current tile contains my_node in world orientation
                let has_my_node = field.nodes.iter().any(|&node| {
                    rotate_field_node(node, tile.rotation) == my_node
                });

                if has_my_node {
                    // Check which field segment on neighbor tile contains n_expected_node in world orientation
                    for n_field in &n_def.fields {
                        let n_has_node = n_field.nodes.iter().any(|&node| {
                            rotate_field_node(node, neighbor.rotation) == n_expected_node
                        });

                        if n_has_node {
                            let n_node_key = format!("{}|{}", n_key, n_field.id);
                            if !visited_nodes.contains(&n_node_key) {
                                visited_nodes.insert(n_node_key);
                                queue.push_back((nx, ny, n_field.id.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    Some(FieldAnalysis {
        tiles,
        meeples,
        adjacent_completed_city_keys,
        visited_nodes,
        pig_players,
    })
}
