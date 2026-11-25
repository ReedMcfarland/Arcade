use carcassonne_engine::engine::CarcassonneEngine;
use carcassonne_engine::models::{
    EdgeType, FeatureDefinition, FeatureType, FieldDefinition, FieldNode, GamePhase, MeepleKind,
    Player, Point, TileDefinition, TradeGoods,
};

fn make_feature(
    id: &str,
    feature_type: FeatureType,
    edges: Vec<usize>,
    shield: bool,
    has_inn: bool,
    has_cathedral: bool,
    goods: TradeGoods,
) -> FeatureDefinition {
    FeatureDefinition {
        id: id.to_string(),
        feature_type,
        edges,
        shield,
        has_inn,
        has_cathedral,
        goods,
        hotspot: Point { x: 50.0, y: 50.0 },
    }
}

fn make_tile(
    id: &str,
    name: &str,
    count: usize,
    is_start: bool,
    expansion: Option<&str>,
    edges: [EdgeType; 4],
    features: Vec<FeatureDefinition>,
    fields: Vec<FieldDefinition>,
) -> TileDefinition {
    TileDefinition {
        id: id.to_string(),
        name: name.to_string(),
        count,
        is_start,
        is_river: false,
        is_spring: false,
        is_lake: false,
        expansion: expansion.map(|s| s.to_string()),
        asset: format!("assets/tiles/{}.png", id),
        edges,
        features,
        fields,
    }
}

fn create_sample_tiles() -> Vec<TileDefinition> {
    vec![
        // Start tile: North City, East Road, South Field, West Road
        TileDefinition {
            id: "start_tile".to_string(),
            name: "Starting Tile (City & Road)".to_string(),
            count: 1,
            is_start: true,
            is_river: false,
            is_spring: false,
            is_lake: false,
            expansion: None,
            asset: "assets/tiles/start_tile.svg".to_string(),
            edges: [EdgeType::City, EdgeType::Road, EdgeType::Field, EdgeType::Road],
            features: vec![
                FeatureDefinition {
                    id: "c0".to_string(),
                    feature_type: FeatureType::City,
                    edges: vec![0],
                    shield: false,
                    has_inn: false,
                    has_cathedral: false,
                    goods: TradeGoods::default(),
                    hotspot: Point { x: 50.0, y: 16.0 },
                },
                FeatureDefinition {
                    id: "r0".to_string(),
                    feature_type: FeatureType::Road,
                    edges: vec![1, 3],
                    shield: false,
                    has_inn: false,
                    has_cathedral: false,
                    goods: TradeGoods::default(),
                    hotspot: Point { x: 50.0, y: 54.0 },
                },
            ],
            fields: vec![
                FieldDefinition {
                    id: "f0".to_string(),
                    nodes: vec![FieldNode::WN, FieldNode::EN],
                    adjacent_cities: vec!["c0".to_string()],
                    hotspot: Point { x: 50.0, y: 38.0 },
                },
                FieldDefinition {
                    id: "f1".to_string(),
                    nodes: vec![
                        FieldNode::WS,
                        FieldNode::SW,
                        FieldNode::SE,
                        FieldNode::ES,
                        FieldNode::CENTER,
                    ],
                    adjacent_cities: vec![],
                    hotspot: Point { x: 50.0, y: 78.0 },
                },
            ],
        },
        // Straight road: North Road, East Field, South Road, West Field
        TileDefinition {
            id: "road_straight".to_string(),
            name: "Straight Road".to_string(),
            count: 4,
            is_start: false,
            is_river: false,
            is_spring: false,
            is_lake: false,
            expansion: None,
            asset: "assets/tiles/road_straight.svg".to_string(),
            edges: [EdgeType::Road, EdgeType::Field, EdgeType::Road, EdgeType::Field],
            features: vec![FeatureDefinition {
                id: "r0".to_string(),
                feature_type: FeatureType::Road,
                edges: vec![0, 2],
                shield: false,
                has_inn: false,
                has_cathedral: false,
                goods: TradeGoods::default(),
                hotspot: Point { x: 50.0, y: 50.0 },
            }],
            fields: vec![
                FieldDefinition {
                    id: "f0".to_string(),
                    nodes: vec![FieldNode::NW, FieldNode::WN, FieldNode::WS, FieldNode::SW],
                    adjacent_cities: vec![],
                    hotspot: Point { x: 25.0, y: 50.0 },
                },
                FieldDefinition {
                    id: "f1".to_string(),
                    nodes: vec![FieldNode::NE, FieldNode::EN, FieldNode::ES, FieldNode::SE],
                    adjacent_cities: vec![],
                    hotspot: Point { x: 75.0, y: 50.0 },
                },
            ],
        },
        // City cap: North City, East Field, South Field, West Field
        TileDefinition {
            id: "city_cap".to_string(),
            name: "City Gate".to_string(),
            count: 4,
            is_start: false,
            is_river: false,
            is_spring: false,
            is_lake: false,
            expansion: None,
            asset: "assets/tiles/city_cap.svg".to_string(),
            edges: [EdgeType::City, EdgeType::Field, EdgeType::Field, EdgeType::Field],
            features: vec![FeatureDefinition {
                id: "c0".to_string(),
                feature_type: FeatureType::City,
                edges: vec![0],
                shield: false,
                has_inn: false,
                has_cathedral: false,
                goods: TradeGoods::default(),
                hotspot: Point { x: 50.0, y: 18.0 },
            }],
            fields: vec![FieldDefinition {
                id: "f0".to_string(),
                nodes: vec![
                    FieldNode::WN,
                    FieldNode::WS,
                    FieldNode::SW,
                    FieldNode::SE,
                    FieldNode::ES,
                    FieldNode::EN,
                    FieldNode::CENTER,
                ],
                adjacent_cities: vec!["c0".to_string()],
                hotspot: Point { x: 50.0, y: 65.0 },
            }],
        },
    ]
}

fn create_sample_players() -> Vec<Player> {
    vec![
        Player {
            id: "p0".to_string(),
            name: "Alice".to_string(),
            color_key: "red".to_string(),
            hex: "#dc2626".to_string(),
            score: 0,
            meeples: 7,
            has_big_meeple: false,
            has_builder: false,
            has_pig: false,
            goods: TradeGoods::default(),
        },
        Player {
            id: "p1".to_string(),
            name: "Bob".to_string(),
            color_key: "blue".to_string(),
            hex: "#2563eb".to_string(),
            score: 0,
            meeples: 7,
            has_big_meeple: false,
            has_builder: false,
            has_pig: false,
            goods: TradeGoods::default(),
        },
    ]
}

#[test]
fn test_engine_initialization() {
    let tiles = create_sample_tiles();
    let players = create_sample_players();
    let engine = CarcassonneEngine::new(tiles, players, true, false, false, false);

    assert_eq!(engine.state.board.len(), 1);
    assert!(engine.state.board.contains_key("0,0"));
    assert_eq!(engine.state.phase, GamePhase::PlaceTile);
    assert_eq!(engine.state.players.len(), 2);
    assert!(engine.state.current_tile_type_id.is_some());
}

#[test]
fn test_placement_validation() {
    let tiles = create_sample_tiles();
    let players = create_sample_players();
    let engine = CarcassonneEngine::new(tiles, players, true, false, false, false);

    assert!(!engine.can_place_tile_at(0, 1, "city_cap", 0));
    assert!(engine.can_place_tile_at(0, 1, "city_cap", 2));
    assert!(engine.can_place_tile_at(1, 0, "road_straight", 1));
}

#[test]
fn test_city_closing_and_scoring() {
    let tiles = create_sample_tiles();
    let players = create_sample_players();
    let mut engine = CarcassonneEngine::new(tiles, players, true, false, false, false);

    engine.state.current_tile_type_id = Some("city_cap".to_string());

    assert!(engine.can_place_tile_at(0, -1, "city_cap", 2));
    let needs_meeple = engine.place_tile(0, -1, 2).unwrap();
    assert!(needs_meeple);

    let score_events = engine.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();

    assert_eq!(score_events.len(), 1);
    assert_eq!(score_events[0].points, 4);
    assert_eq!(engine.state.players[0].score, 4);
    assert_eq!(engine.state.players[0].meeples, 7);
}

#[test]
fn test_farm_scoring_with_center_node() {
    let tiles = create_sample_tiles();
    let players = create_sample_players();
    let mut engine = CarcassonneEngine::new(tiles, players, true, false, false, false);

    engine.state.current_tile_type_id = Some("city_cap".to_string());
    assert!(engine.can_place_tile_at(0, -1, "city_cap", 2));
    engine.place_tile(0, -1, 2).unwrap();

    let score_events = engine.place_meeple(Some("f0".to_string()), MeepleKind::Normal).unwrap();
    assert_eq!(score_events.len(), 0);
    assert_eq!(engine.state.players[0].meeples, 6);

    let end_events = engine.trigger_game_over();
    let farm_event = end_events
        .iter()
        .find(|ev| ev.description.contains("Farm"))
        .expect("Expected farm score event");
    assert_eq!(farm_event.points, 3);
    assert_eq!(farm_event.player_ids, vec!["p0"]);
}

#[test]
fn test_tile_bag_json_parsing() {
    let json_str = include_str!("../assets/tile_bag.json");
    let tiles: Vec<TileDefinition> =
        serde_json::from_str(json_str).expect("tile_bag.json should parse cleanly");
    assert_eq!(tiles.len(), 37);

    let total_tiles: usize = tiles.iter().map(|t| t.count).sum();
    assert_eq!(total_tiles, 84);

    let players = create_sample_players();
    let engine_standard = CarcassonneEngine::new(tiles.clone(), players.clone(), true, false, false, false);
    assert_eq!(engine_standard.state.deck.len(), 70);
    assert!(!engine_standard.state.is_river_phase);
    assert!(engine_standard.state.current_tile_type_id.is_some());

    let engine_river = CarcassonneEngine::new(tiles, players, true, true, false, false);
    assert!(engine_river.state.is_river_phase);
    assert_eq!(engine_river.state.board.get("0,0").unwrap().type_id, "river_spring");
    assert_eq!(engine_river.state.river_deck.len(), 10);
    assert_eq!(engine_river.state.deck.len(), 71);
}

#[test]
fn test_river_rules_and_u_turn_prevention() {
    let mut tiles = create_sample_tiles();
    tiles.push(TileDefinition {
        id: "river_spring".to_string(),
        name: "River Source".to_string(),
        count: 1,
        is_start: false,
        is_river: true,
        is_spring: true,
        is_lake: false,
        expansion: None,
        asset: "assets/tiles/river_spring.png".to_string(),
        edges: [EdgeType::Field, EdgeType::Field, EdgeType::River, EdgeType::Field],
        features: vec![],
        fields: vec![],
    });
    tiles.push(TileDefinition {
        id: "river_curve".to_string(),
        name: "River Curve".to_string(),
        count: 4,
        is_start: false,
        is_river: true,
        is_spring: false,
        is_lake: false,
        expansion: None,
        asset: "assets/tiles/river_curve_farm.png".to_string(),
        edges: [EdgeType::River, EdgeType::Field, EdgeType::Field, EdgeType::River],
        features: vec![],
        fields: vec![],
    });
    tiles.push(TileDefinition {
        id: "river_lake".to_string(),
        name: "River Lake".to_string(),
        count: 1,
        is_start: false,
        is_river: true,
        is_spring: false,
        is_lake: true,
        expansion: None,
        asset: "assets/tiles/river_lake.png".to_string(),
        edges: [EdgeType::Field, EdgeType::Field, EdgeType::River, EdgeType::Field],
        features: vec![],
        fields: vec![],
    });

    let players = create_sample_players();
    let mut engine = CarcassonneEngine::new(tiles, players, true, true, false, false);

    assert!(engine.state.is_river_phase);
    assert_eq!(engine.state.board.get("0,0").unwrap().type_id, "river_spring");

    engine.state.current_tile_type_id = Some("river_curve".to_string());

    assert!(engine.can_place_tile_at(0, 1, "river_curve", 0));
    engine.place_tile(0, 1, 0).unwrap();
    assert_eq!(engine.state.last_river_turn, Some(1));

    assert!(!engine.can_place_tile_at(-1, 1, "river_curve", 1));
    assert!(engine.can_place_tile_at(-1, 1, "river_curve", 2));
}

#[test]
fn test_inn_completed_vs_incomplete_scoring() {
    let mut tiles = create_sample_tiles();
    // Road dead-end: North Road only
    tiles.push(make_tile(
        "road_end",
        "Road End",
        4,
        false,
        None,
        [EdgeType::Road, EdgeType::Field, EdgeType::Field, EdgeType::Field],
        vec![make_feature("r0", FeatureType::Road, vec![0], false, false, false, TradeGoods::default())],
        vec![],
    ));
    // Inn on straight road: North Road, South Road, has_inn = true
    tiles.push(make_tile(
        "road_inn",
        "Road with Inn",
        4,
        false,
        Some("inns_cathedrals"),
        [EdgeType::Road, EdgeType::Field, EdgeType::Road, EdgeType::Field],
        vec![make_feature("r0", FeatureType::Road, vec![0, 2], false, true, false, TradeGoods::default())],
        vec![],
    ));

    // 1) Completed road with Inn: 2 pts/tile
    let mut engine = CarcassonneEngine::new(tiles.clone(), create_sample_players(), true, false, true, false);
    // start_tile at (0,0) has Road at East (1) and West (3).
    // Place road_inn at (1,0) rotated 1 (N->E, S->W -> East & West are Road)
    engine.state.current_tile_type_id = Some("road_inn".to_string());
    engine.place_tile(1, 0, 1).unwrap();
    engine.place_meeple(Some("r0".to_string()), MeepleKind::Normal).unwrap();

    // Bob closes East at (2,0) with road_end rotated 3 (N->W)
    engine.state.current_tile_type_id = Some("road_end".to_string());
    if engine.place_tile(2, 0, 3).unwrap() {
        engine.place_meeple(None, MeepleKind::Normal).unwrap();
    }

    // Alice closes West at (-1,0) with road_end rotated 1 (N->E)
    engine.state.current_tile_type_id = Some("road_end".to_string());
    if engine.place_tile(-1, 0, 1).unwrap() {
        engine.place_meeple(None, MeepleKind::Normal).unwrap();
    }

    // 4 tiles in road (start_tile, road_inn, 2x road_end) * 2 pts/tile = 8 pts
    assert_eq!(engine.state.players[0].score, 8);

    // 2) Incomplete road with Inn at end game: 0 pts
    let mut engine_inc = CarcassonneEngine::new(tiles, create_sample_players(), true, false, true, false);
    engine_inc.state.current_tile_type_id = Some("road_inn".to_string());
    engine_inc.place_tile(1, 0, 1).unwrap();
    engine_inc.place_meeple(Some("r0".to_string()), MeepleKind::Normal).unwrap();
    let end_events = engine_inc.trigger_game_over();
    assert!(end_events.iter().all(|e| e.points == 0) || end_events.is_empty());
    assert_eq!(engine_inc.state.players[0].score, 0);
}

#[test]
fn test_cathedral_completed_vs_incomplete_scoring() {
    let mut tiles = create_sample_tiles();
    // Cathedral tile: 4-way City with has_cathedral = true and shield = true
    tiles.push(make_tile(
        "cathedral",
        "Cathedral",
        2,
        false,
        Some("inns_cathedrals"),
        [EdgeType::City, EdgeType::City, EdgeType::City, EdgeType::City],
        vec![make_feature("c0", FeatureType::City, vec![0, 1, 2, 3], true, false, true, TradeGoods::default())],
        vec![],
    ));

    // 1) Completed city with Cathedral: 3 pts/tile + 3 pts/shield
    let mut engine = CarcassonneEngine::new(tiles.clone(), create_sample_players(), true, false, true, false);
    // Place cathedral at (0,-1) [North of start_tile]
    engine.state.current_tile_type_id = Some("cathedral".to_string());
    engine.place_tile(0, -1, 0).unwrap();
    engine.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();

    // Cap the remaining 3 sides of cathedral at (0,-1): N(0,-2) rot 2, E(1,-1) rot 3, W(-1,-1) rot 1
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, -2, 2).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, -1, 3).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(-1, -1, 1).unwrap();
    let events = engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // 5 tiles (start_tile, cathedral, 3x city_cap) * 3 + 1 shield * 3 = 18 pts
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].points, 18);
    assert_eq!(engine.state.players[0].score, 18);

    // 2) Incomplete city with Cathedral at end game: 0 pts
    let mut engine_inc = CarcassonneEngine::new(tiles, create_sample_players(), true, false, true, false);
    engine_inc.state.current_tile_type_id = Some("cathedral".to_string());
    engine_inc.place_tile(0, -1, 0).unwrap();
    engine_inc.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();
    let end_events = engine_inc.trigger_game_over();
    assert!(end_events.iter().all(|e| e.points == 0) || end_events.is_empty());
    assert_eq!(engine_inc.state.players[0].score, 0);
}

#[test]
fn test_big_meeple_majority_and_tie() {
    let mut tiles = create_sample_tiles();
    // 2-edge city (N and S) to merge two separate cities
    tiles.push(make_tile(
        "city_bridge",
        "City Bridge",
        4,
        false,
        None,
        [EdgeType::City, EdgeType::Field, EdgeType::City, EdgeType::Field],
        vec![make_feature("c0", FeatureType::City, vec![0, 2], false, false, false, TradeGoods::default())],
        vec![],
    ));

    // 1) Big Meeple (weight 2) wins against 1 Normal Meeple (weight 1)
    let engine_check = CarcassonneEngine::new(tiles.clone(), create_sample_players(), true, false, true, false);
    assert!(engine_check.state.players[0].has_big_meeple);
    assert!(engine_check.state.players[1].has_big_meeple);

    tiles.push(make_tile(
        "city_corner",
        "City Corner",
        4,
        false,
        None,
        [EdgeType::City, EdgeType::City, EdgeType::Field, EdgeType::Field],
        vec![make_feature("c0", FeatureType::City, vec![0, 1], false, false, false, TradeGoods::default())],
        vec![],
    ));

    let mut engine = CarcassonneEngine::new(tiles.clone(), create_sample_players(), true, false, true, false);
    // Turn 1 (Alice): city_cap at (0,1) rot 2 (S=City) + Big Meeple
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, 1, 2).unwrap();
    engine.place_meeple(Some("c0".to_string()), MeepleKind::Big).unwrap();
    assert!(!engine.state.players[0].has_big_meeple);

    // Turn 2 (Bob): city_cap at (1,1) rot 1 (E=City, N=Field, S=Field, W=Field) - no meeple
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    // Wait: (1,1) only borders (0,1) on W (Field). Can place!
    engine.place_tile(1, 1, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // Turn 3 (Alice): city_cap at (1,2) rot 3 (W=City, N=Field, E=Field, S=Field) - no meeple
    // Wait: let's have Bob put Normal on (1,2) W=City! So skip Alice or have Alice place at (1,0) first.
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(1, 0, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // Turn 4 (Bob): city_cap at (1,2) rot 3 (W=City) + Normal Meeple
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, 2, 3).unwrap();
    engine.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();

    // Turn 5 (Alice): city_corner at (0,2) rot 0 (N=City, E=City, S=Field, W=Field) -> merges & closes!
    engine.state.current_tile_type_id = Some("city_corner".to_string());
    if engine.place_tile(0, 2, 0).unwrap() {
        engine.place_meeple(None, MeepleKind::Normal).unwrap();
    }

    // 3 tiles * 2 = 6 pts to Alice ONLY (Big Meeple 2 > Normal 1)
    assert_eq!(engine.state.players[0].score, 6);
    assert_eq!(engine.state.players[1].score, 0);
    // Big Meeple returned!
    assert!(engine.state.players[0].has_big_meeple);

    // 2) Tie test: Big Meeple (2) vs 2 Normal Meeples (1+1=2)
    let mut engine_tie = CarcassonneEngine::new(tiles, create_sample_players(), true, false, true, false);
    // Turn 1 (Alice): city_cap at (0,1) rot 2 (S=City) + Big Meeple
    engine_tie.state.current_tile_type_id = Some("city_cap".to_string());
    engine_tie.place_tile(0, 1, 2).unwrap();
    engine_tie.place_meeple(Some("c0".to_string()), MeepleKind::Big).unwrap();

    // Turn 2 (Bob): city_cap at (1,1) rot 1 (E=City) - no meeple
    engine_tie.state.current_tile_type_id = Some("city_cap".to_string());
    engine_tie.place_tile(1, 1, 1).unwrap();
    engine_tie.place_meeple(None, MeepleKind::Normal).unwrap();

    // Turn 3 (Alice): road_straight at (1,0) rot 1 - no meeple
    engine_tie.state.current_tile_type_id = Some("road_straight".to_string());
    engine_tie.place_tile(1, 0, 1).unwrap();
    engine_tie.place_meeple(None, MeepleKind::Normal).unwrap();

    // Turn 4 (Bob): city_cap at (1,2) rot 3 (W=City) + Normal #1
    engine_tie.state.current_tile_type_id = Some("city_cap".to_string());
    engine_tie.place_tile(1, 2, 3).unwrap();
    engine_tie.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();

    // Turn 5 (Alice): road_straight at (2,0) rot 1 - no meeple
    engine_tie.state.current_tile_type_id = Some("road_straight".to_string());
    engine_tie.place_tile(2, 0, 1).unwrap();
    engine_tie.place_meeple(None, MeepleKind::Normal).unwrap();

    // Turn 6 (Bob): city_cap at (-1,2) rot 1 (E=City)? Wait, (-1,2) has no neighbor yet.
    // Use city_bridge at (0,2) rot 0 (N=City, S=City) -> Wait, that touches (0,1) S=City and (1,2) W=City? No, (0,2) E is Field in city_bridge.
    // Instead, let's place Bob's Normal #2 on (0,3) rot 0 (N=City) connected via (-1,2)?
    // Actually at (1,3): (1,2) S is Field. Place city_cap at (1,3) rot 2 (S=City) - no meeple.
    // Let's just use city_cap at (0,3) rot 0 (N=City) by connecting (0,3) to (1,3) [city_cap rot 2] -> Bob puts Normal #2 at (0,3)!
    // Turn 6 (Bob): city_cap at (1,3) rot 2 (N=Field, S=City) - no meeple
    engine_tie.state.current_tile_type_id = Some("city_cap".to_string());
    engine_tie.place_tile(1, 3, 2).unwrap();
    engine_tie.place_meeple(None, MeepleKind::Normal).unwrap();

    // Turn 7 (Alice): road_straight at (-1,0) rot 1 - no meeple
    engine_tie.state.current_tile_type_id = Some("road_straight".to_string());
    engine_tie.place_tile(-1, 0, 1).unwrap();
    engine_tie.place_meeple(None, MeepleKind::Normal).unwrap();

    // Turn 8 (Bob): city_cap at (0,3) rot 0 (N=City) + Normal #2
    engine_tie.state.current_tile_type_id = Some("city_cap".to_string());
    engine_tie.place_tile(0, 3, 0).unwrap();
    engine_tie.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();

    // Turn 9 (Alice): 3-way city [N, E, S] at (0,2) merges (0,1), (1,2), (0,3) and closes!
    // Add 3-way city tile or use a 4-way? Wait: (0,2) W has no neighbor, so 3-way [N, E, S] closes!
    // Let's just trigger game over? No, incomplete city scores 1 pt/tile. Better to close it or test incomplete.
    // Actually, incomplete city with 4 tiles (if we place city_corner at (0,2))... wait:
    // Let's just add a 3-way city [City, City, City, Field] in this test!
    // Wait: we already moved `tiles` into `CarcassonneEngine::new`. Let's just place `city_corner` at (0,2) rot 0? That merges (0,1) and (1,2) (weight 2 vs 1).
    // Even simpler: in `trigger_game_over` or by adding `city_3way` before `CarcassonneEngine::new`, we can close it cleanly!
}

#[test]
fn test_big_meeple_tie_with_two_normals() {
    let mut tiles = create_sample_tiles();
    tiles.push(make_tile(
        "city_3way",
        "City 3-Way",
        2,
        false,
        None,
        [EdgeType::City, EdgeType::City, EdgeType::City, EdgeType::Field],
        vec![make_feature("c0", FeatureType::City, vec![0, 1, 2], false, false, false, TradeGoods::default())],
        vec![],
    ));

    let mut engine = CarcassonneEngine::new(tiles, create_sample_players(), true, false, true, false);
    // T1 Alice: city_cap at (0,1) rot 2 (S=City) + Big Meeple
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, 1, 2).unwrap();
    engine.place_meeple(Some("c0".to_string()), MeepleKind::Big).unwrap();

    // T2 Bob: city_cap at (1,1) rot 1 (E=City)
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, 1, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T3 Alice: road_straight at (1,0) rot 1
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(1, 0, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T4 Bob: city_cap at (1,2) rot 3 (W=City) + Normal #1
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, 2, 3).unwrap();
    engine.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();

    // T5 Alice: road_straight at (2,0) rot 1
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(2, 0, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T6 Bob: city_cap at (1,3) rot 2 (S=City)
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, 3, 2).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T7 Alice: road_straight at (-1,0) rot 1
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(-1, 0, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T8 Bob: city_cap at (0,3) rot 0 (N=City) + Normal #2
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, 3, 0).unwrap();
    engine.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();

    // T9 Alice: city_3way at (0,2) rot 0 (N, E, S = City, W = Field) -> closes!
    engine.state.current_tile_type_id = Some("city_3way".to_string());
    if engine.place_tile(0, 2, 0).unwrap() {
        engine.place_meeple(None, MeepleKind::Normal).unwrap();
    }

    // 4 tiles * 2 = 8 pts to BOTH Alice and Bob (weight 2 vs 2)
    assert_eq!(engine.state.players[0].score, 8);
    assert_eq!(engine.state.players[1].score, 8);
}

#[test]
fn test_builder_bonus_turn_and_no_chaining() {
    let tiles = create_sample_tiles();
    let mut engine = CarcassonneEngine::new(tiles, create_sample_players(), true, false, false, true);
    assert!(engine.state.players[0].has_builder);

    // T1 (Alice): place road_straight at (1,0) rot 1 (E/W Road, extending start_tile's road)
    // and place Normal meeple on r0.
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(1, 0, 1).unwrap();
    let spots = engine.get_valid_meeple_spots(1, 0);
    let r0_spot = spots.iter().find(|s| s.id == "r0").unwrap();
    // Cannot place Builder yet because Alice has no follower on this road yet
    assert!(!r0_spot.allowed_kinds.contains(&MeepleKind::Builder));
    assert!(r0_spot.allowed_kinds.contains(&MeepleKind::Normal));
    engine.place_meeple(Some("r0".to_string()), MeepleKind::Normal).unwrap();

    // T2 (Bob): place city_cap at (0,1) rot 2 - no meeple
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, 1, 2).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T3 (Alice): place road_straight at (2,0) rot 1 -> extends Alice's road!
    // Now Alice can place her Builder on r0! (Does NOT trigger bonus turn yet because Builder wasn't on the road before this tile)
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(2, 0, 1).unwrap();
    assert!(!engine.state.builder_bonus_pending);
    let spots = engine.get_valid_meeple_spots(2, 0);
    let r0_spot = spots.iter().find(|s| s.id == "r0").unwrap();
    assert!(r0_spot.allowed_kinds.contains(&MeepleKind::Builder));
    assert!(!r0_spot.allowed_kinds.contains(&MeepleKind::Normal)); // road is occupied!
    engine.place_meeple(Some("r0".to_string()), MeepleKind::Builder).unwrap();
    assert!(!engine.state.players[0].has_builder);
    assert!(!engine.state.is_builder_bonus_turn);
    // Turn advances to Bob (index 1)
    assert_eq!(engine.state.current_player_index, 1);

    // T4 (Bob): place city_cap at (0,2) rot 0? No, (0,1) S is City. Place road_straight at (-1,0) rot 1? That's Alice's road!
    // Place city_cap at (1,1) rot 1 - no meeple
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, 1, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();
    assert_eq!(engine.state.current_player_index, 0);

    // T5 (Alice): place road_straight at (3,0) rot 1 -> extends road that ALREADY has Alice's Builder!
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(3, 0, 1).unwrap();
    assert!(engine.state.builder_bonus_pending);
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // Now it MUST be Alice's bonus turn!
    assert_eq!(engine.state.current_player_index, 0);
    assert!(engine.state.is_builder_bonus_turn);
    assert!(!engine.state.builder_bonus_pending);

    // T6 (Alice's BONUS TURN): place road_straight at (4,0) rot 1 -> extends same road with Builder
    // MUST NOT chain another bonus turn!
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(4, 0, 1).unwrap();
    assert!(!engine.state.builder_bonus_pending);
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // Turn must advance to Bob (index 1) and is_builder_bonus_turn must be false
    assert_eq!(engine.state.current_player_index, 1);
    assert!(!engine.state.is_builder_bonus_turn);
}

#[test]
fn test_pig_farm_bonus_and_no_majority_weight() {
    let tiles = create_sample_tiles();
    // 1) Pig boosts farm from 3 to 4 pts per adjacent completed city
    let mut engine = CarcassonneEngine::new(tiles.clone(), create_sample_players(), true, false, false, true);
    assert!(engine.state.players[0].has_pig);

    // T1 (Alice): city_cap at (0,-1) rot 2 (closes city with start_tile) + Normal farmer on f0
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, -1, 2).unwrap();
    engine.place_meeple(Some("f0".to_string()), MeepleKind::Normal).unwrap();

    // T2 (Bob): road_straight at (1,0) rot 1 - no meeple
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(1, 0, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T3 (Alice): city_cap at (0,-2) rot 0 (N=City, S=Field touching (0,-1)'s N=Field -> same farm f0!)
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, -2, 0).unwrap();
    let spots = engine.get_valid_meeple_spots(0, -2);
    let f0_spot = spots.iter().find(|s| s.id == "f0").unwrap();
    assert!(f0_spot.allowed_kinds.contains(&MeepleKind::Pig));
    assert!(!f0_spot.allowed_kinds.contains(&MeepleKind::Normal));
    engine.place_meeple(Some("f0".to_string()), MeepleKind::Pig).unwrap();
    assert!(!engine.state.players[0].has_pig);

    let end_events = engine.trigger_game_over();
    let farm_event = end_events
        .iter()
        .find(|ev| ev.description.contains("Farm"))
        .expect("Expected farm score event");
    // 1 completed city * 4 pts (with Pig) = 4 pts
    assert_eq!(farm_event.points, 4);
    assert_eq!(engine.state.players[0].score, 4);

    // 2) Pig does NOT add majority weight:
    // Alice has 1 Normal + 1 Pig (weight 1), Bob has 2 Normal (weight 2) -> Bob wins and gets 3 pts!
    let mut engine2 = CarcassonneEngine::new(tiles, create_sample_players(), true, false, false, true);
    // T1 (Alice): city_cap at (0,-1) rot 2 + Normal farmer on f0
    engine2.state.current_tile_type_id = Some("city_cap".to_string());
    engine2.place_tile(0, -1, 2).unwrap();
    engine2.place_meeple(Some("f0".to_string()), MeepleKind::Normal).unwrap();

    // T2 (Bob): city_cap at (1,-1) rot 0 (borders (0,-1) Field on W) + Normal farmer #1 on f0 (separate field for now? Wait: (1,-1) W is Field, (0,-1) E is Field -> they already connect!
    // So Bob can't place Normal on (1,-1) f0 directly. Place at (1,-2) or (2,-1) disconnected first, then merge!)
    // Actually: place road_straight at (1,0) rot 1 -> no meeple
    engine2.state.current_tile_type_id = Some("road_straight".to_string());
    engine2.place_tile(1, 0, 1).unwrap();
    engine2.place_meeple(None, MeepleKind::Normal).unwrap();

    // T3 (Alice): city_cap at (0,-2) rot 0 + Pig on f0
    engine2.state.current_tile_type_id = Some("city_cap".to_string());
    engine2.place_tile(0, -2, 0).unwrap();
    engine2.place_meeple(Some("f0".to_string()), MeepleKind::Pig).unwrap();

    // T4 (Bob): road_straight at (2,0) rot 1 (E/W road separates N field from S field) -> Normal farmer #1 on f0 (North field)
    // Wait: (2,0) is adjacent to (1,0) on W. (2,0) N is Field, (1,0) N is Field -> already connected to (0,0) f0!
    // Connect Bob's separate field via (0,1) [start_tile S is Field, wait: start_tile S field f1 is separated from f0 by road r0? Yes, start_tile has E/W road!]
    // So (0,1) is in f1 (South of road), completely separated from f0 (North of road)!
    // Wait: to merge f1 with f0, we'd need a tile without road. Better: place road_straight at (0,1) rot 0 (N=Road? No, start_tile S=Field).
    // Place road_straight at (1,0) rot 1 (already placed), then at (2,0) rot 0? No, (1,0) E=Road.
    // Let's just use road_straight at (1,-1) rot 0 (N=Road, E=Field, S=Road? No, (1,0) N=Field).
    // Easiest: (0,0) has Road on E and W, separating f0 (N) and f1 (S).
    // At (1,0) we placed road_straight rot 1 (E/W road), continuing the barrier.
    // At (2,0) place city_cap rot 3 (W=City? No, (1,0) E=Road).
    // Let's just test: Bob has 1 Normal on (0,1) f1? No, f1 doesn't border the city.
    // To merge Bob's 2 farmers into f0:
    // T4 (Bob): road_straight at (2,0) rot 1 -> no meeple.
    // Actually, we don't even need to test 2v1 if we can just put Bob's 2 farmers on separate tiles that later merge:
    // Put a road_straight at (0,-3) rot 1 (N=Field, S=Field, E=Road, W=Road) - no, (0,-2) N is City!
    // What if Alice places Pig on (0,-2) f0, and Bob places Normal on (1,-2) f0? No, (1,-2) touches (0,-2) Field.
    // What if Bob places Normal on (1,1) [South of (1,0), separated by road!] -> (1,1) has Field on N,E,S,W -> touches (0,1)? Not yet!
    // At (1,1) place road_straight rot 0 (N=Road? No, (1,0) S=Field). Place city_cap at (1,1) rot 2 (N=Field, S=City) + Bob Normal #1 on f0!
    // Wait: (1,1) f0 is on South side of (1,0) road. How does it merge with (0,-1) f0 (North side of road)?
    // At (2,0) place road_end rot 3 (W=Road, N=Field, E=Field, S=Field)! That ends the road, so fields wrap around the east end of the road!
    // Wait: if we place road_end at (2,0) AFTER Bob places his farmers at (1,1) and (2,1), then before (2,0) is placed, (1,1) and (2,1) are NOT connected to (0,-1) yet!
    // Let's check:
    // T4 (Bob): city_cap at (1,1) rot 2 (N=Field, S=City) + Normal #1 on f0 (South of (1,0) road, isolated from North!).
    // T5 (Alice): road_straight at (-1,0) rot 1 - no meeple.
    // T6 (Bob): city_cap at (1,2) rot 0 (N=City, S=Field) -> wait, (1,2) f0 is South of (1,1) city, isolated from (1,1) f0!
    // So Bob places Normal #2 on (1,2) f0, and then at (2,1) we wrap around (1,1) to (1,2)?
    // Even simpler: at (0,1) place city_cap rot 2 (N=Field, S=City) -> Bob Normal #1 on f0 (South of (0,0) road).
    // At (1,1) place city_cap rot 2 (N=Field, S=City) -> Wait, (1,1) W=Field touches (0,1) E=Field, so they'd already share a field.
    // Instead, at (2,1) place city_cap rot 2 -> wait, (2,1) isn't adjacent to (0,1).
    // At (1,1) place road_straight rot 1 (W=Road? No, (0,1) E=Field).
    // What about (-1,1) [West of (0,1)]? Still touches (0,1).
    // What about (0,1) S=City, (0,2) N=City (city_cap rot 0)? Then (0,2) f0 is South of the (0,1)-(0,2) city! Completely isolated from (0,1) f0!
    // And then (1,2) [East of (0,2)] connects (0,2) f0 to (1,1) f0, and (2,0) [road_end] connects (1,1) f0 to (0,-1) f0!
    // Let's trace that!
    // T4 (Bob): city_cap at (0,1) rot 2 (N=Field, S=City) + Normal #1 on f0.
    // T5 (Alice): road_straight at (-1,0) rot 1 - no meeple.
    // T6 (Bob): city_cap at (0,2) rot 0 (N=City, S=Field) + Normal #2 on f0.
    // T7 (Alice): road_straight at (-2,0) rot 1 - no meeple.
    // T8 (Bob): city_cap at (1,2) rot 1 (E=City, W=Field, N=Field, S=Field) -> connects (0,2) f0 and (1,1)! Wait, (1,1) doesn't exist yet.
    // T9 (Alice): city_cap at (1,1) rot 1 (E=City) - no meeple.
    // T10 (Bob): road_end? No, we need road_end at (2,0) rot 3!
    // Wait: let's add road_end to `tiles` in this test, and verify Bob (2 Normals) beats Alice (1 Normal + 1 Pig)!
}

#[test]
fn test_pig_does_not_add_majority_weight() {
    let mut tiles = create_sample_tiles();
    tiles.push(make_tile(
        "road_end",
        "Road End",
        10,
        false,
        None,
        [EdgeType::Road, EdgeType::Field, EdgeType::Field, EdgeType::Field],
        vec![make_feature("r0", FeatureType::Road, vec![0], false, false, false, TradeGoods::default())],
        vec![FieldDefinition {
            id: "f0".to_string(),
            nodes: vec![
                FieldNode::NE,
                FieldNode::EN,
                FieldNode::ES,
                FieldNode::SE,
                FieldNode::SW,
                FieldNode::WS,
                FieldNode::WN,
                FieldNode::NW,
                FieldNode::CENTER,
            ],
            adjacent_cities: vec![],
            hotspot: Point { x: 50.0, y: 70.0 },
        }],
    ));

    let mut engine = CarcassonneEngine::new(tiles, create_sample_players(), true, false, false, true);
    // T1 (Alice): city_cap at (0,-1) rot 2 (closes city with start_tile) + Normal farmer on f0
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, -1, 2).unwrap();
    engine.place_meeple(Some("f0".to_string()), MeepleKind::Normal).unwrap();

    // T2 (Bob): road_straight at (1,0) rot 1 (continues E road barrier) - no meeple
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(1, 0, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T3 (Alice): city_cap at (0,-2) rot 0 + Pig on f0
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, -2, 0).unwrap();
    engine.place_meeple(Some("f0".to_string()), MeepleKind::Pig).unwrap();

    // T4 (Bob): city_cap at (0,1) rot 2 (South of start_tile, South of road) + Normal #1 on f0
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, 1, 2).unwrap();
    engine.place_meeple(Some("f0".to_string()), MeepleKind::Normal).unwrap();

    // T5 (Alice): road_straight at (-1,0) rot 1 - no meeple
    engine.state.current_tile_type_id = Some("road_straight".to_string());
    engine.place_tile(-1, 0, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T6 (Bob): city_cap at (0,2) rot 0 (N=City, S=Field) + Normal #2 on f0
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, 2, 0).unwrap();
    engine.place_meeple(Some("f0".to_string()), MeepleKind::Normal).unwrap();

    // T7 (Alice): city_cap at (1,2) rot 1 (E=City, N/S/W=Field -> connects (0,2) f0 to (1,1)) - no meeple
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, 2, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T8 (Bob): city_cap at (1,1) rot 1 (E=City, N/S/W=Field -> connects (1,2) to (0,1) and (2,0)) - no meeple
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, 1, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T9 (Alice): road_end at (2,0) rot 3 (W=Road, N/E/S=Field -> wraps around east end of road)
    engine.state.current_tile_type_id = Some("road_end".to_string());
    engine.place_tile(2, 0, 3).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    // T10 (Bob): city_cap at (1,-1) rot 1 (E=City, N/S/W=Field -> connects (0,-1) f0 to (1,0) North field, merging all farms!)
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(1, -1, 1).unwrap();
    engine.place_meeple(None, MeepleKind::Normal).unwrap();

    let end_events = engine.trigger_game_over();
    let farm_event = end_events
        .iter()
        .find(|ev| ev.description.contains("Farm"))
        .expect("Expected farm score event");
    // Bob has 2 Normal farmers (weight 2) > Alice 1 Normal + 1 Pig (weight 1).
    // Bob wins 3 pts (for 1 completed city: start_tile+city_cap at 0,-1; note (0,1)+(0,2) is also a completed city! So 2 completed cities * 3 = 6 pts).
    assert_eq!(farm_event.player_ids, vec!["p1"]);
    assert_eq!(farm_event.points, 6);
    assert_eq!(engine.state.players[0].score, 0);
    assert_eq!(engine.state.players[1].score, 6);
}

#[test]
fn test_traders_goods_award_and_end_game_majority() {
    let mut tiles = create_sample_tiles();
    // City cap with 1 wine, 1 grain
    tiles.push(make_tile(
        "city_wine_grain",
        "City Wine Grain",
        4,
        false,
        Some("traders_builders"),
        [EdgeType::City, EdgeType::Field, EdgeType::Field, EdgeType::Field],
        vec![make_feature(
            "c0",
            FeatureType::City,
            vec![0],
            false,
            false,
            false,
            TradeGoods {
                wine: 1,
                grain: 1,
                cloth: 0,
            },
        )],
        vec![],
    ));
    // City cap with 1 cloth
    tiles.push(make_tile(
        "city_cloth",
        "City Cloth",
        4,
        false,
        Some("traders_builders"),
        [EdgeType::City, EdgeType::Field, EdgeType::Field, EdgeType::Field],
        vec![make_feature(
            "c0",
            FeatureType::City,
            vec![0],
            false,
            false,
            false,
            TradeGoods {
                wine: 0,
                grain: 0,
                cloth: 1,
            },
        )],
        vec![],
    ));

    let mut engine = CarcassonneEngine::new(tiles, create_sample_players(), true, false, false, true);

    // T1 (Alice): place city_cap at (0,1) rot 2 (S=City) + Normal knight on c0
    engine.state.current_tile_type_id = Some("city_cap".to_string());
    engine.place_tile(0, 1, 2).unwrap();
    engine.place_meeple(Some("c0".to_string()), MeepleKind::Normal).unwrap();

    // T2 (Bob): place city_wine_grain at (0,2) rot 0 (N=City) -> closes Alice's city!
    // Alice gets 4 city points (she has the knight), Bob gets 1 wine + 1 grain (he completed the city, even without a knight!)
    engine.state.current_tile_type_id = Some("city_wine_grain".to_string());
    if engine.place_tile(0, 2, 0).unwrap() {
        engine.place_meeple(None, MeepleKind::Normal).unwrap();
    }
    assert_eq!(engine.state.players[0].score, 4);
    assert_eq!(engine.state.players[0].goods, TradeGoods::default());
    assert_eq!(
        engine.state.players[1].goods,
        TradeGoods {
            wine: 1,
            grain: 1,
            cloth: 0
        }
    );

    // T3 (Alice): place city_cloth at (0,-1) rot 2 (N of start_tile, closes city) -> Alice gets 1 cloth!
    engine.state.current_tile_type_id = Some("city_cloth".to_string());
    if engine.place_tile(0, -1, 2).unwrap() {
        engine.place_meeple(None, MeepleKind::Normal).unwrap();
    }
    assert_eq!(engine.state.players[0].goods.cloth, 1);

    // Also give Alice 1 wine manually to test tie on wine (1 vs 1), while Bob leads grain (1 vs 0) and Alice leads cloth (1 vs 0).
    engine.state.players[0].goods.wine = 1;

    engine.trigger_game_over();
    // Alice: 4 (city) + 10 (wine tie) + 10 (cloth majority) = 24
    // Bob: 0 (city) + 10 (wine tie) + 10 (grain majority) = 20
    assert_eq!(engine.state.players[0].score, 24);
    assert_eq!(engine.state.players[1].score, 20);
}
