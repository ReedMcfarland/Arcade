use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeType {
    City,
    Road,
    Field,
    River,
}

impl Serialize for EdgeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            EdgeType::City => serializer.serialize_str("C"),
            EdgeType::Road => serializer.serialize_str("R"),
            EdgeType::Field => serializer.serialize_str("F"),
            EdgeType::River => serializer.serialize_str("W"),
        }
    }
}

impl<'de> Deserialize<'de> for EdgeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "C" | "City" | "city" => Ok(EdgeType::City),
            "R" | "Road" | "road" => Ok(EdgeType::Road),
            "F" | "Field" | "field" => Ok(EdgeType::Field),
            "W" | "River" | "river" => Ok(EdgeType::River),
            other => Err(serde::de::Error::custom(format!("Unknown edge type: {}", other))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldNode {
    NW,
    NE,
    EN,
    ES,
    SE,
    SW,
    WS,
    WN,
    CENTER,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureType {
    City,
    Road,
    Monastery,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeGoods {
    #[serde(default)]
    pub wine: u32,
    #[serde(default)]
    pub grain: u32,
    #[serde(default)]
    pub cloth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MeepleKind {
    #[default]
    Normal,
    Big,
    Builder,
    Pig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureDefinition {
    pub id: String,
    #[serde(rename = "type")]
    pub feature_type: FeatureType,
    #[serde(default)]
    pub edges: Vec<usize>,
    #[serde(default)]
    pub shield: bool,
    #[serde(default)]
    pub has_inn: bool,
    #[serde(default)]
    pub has_cathedral: bool,
    #[serde(default)]
    pub goods: TradeGoods,
    pub hotspot: Point,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub id: String,
    pub nodes: Vec<FieldNode>,
    #[serde(default)]
    pub adjacent_cities: Vec<String>,
    pub hotspot: Point,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileDefinition {
    pub id: String,
    pub name: String,
    #[serde(default = "default_count")]
    pub count: usize,
    #[serde(default)]
    pub is_start: bool,
    #[serde(default)]
    pub is_river: bool,
    #[serde(default)]
    pub is_spring: bool,
    #[serde(default)]
    pub is_lake: bool,
    #[serde(default)]
    pub expansion: Option<String>,
    pub asset: String,
    pub edges: [EdgeType; 4],
    #[serde(default)]
    pub features: Vec<FeatureDefinition>,
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
}

fn default_count() -> usize {
    1
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meeple {
    pub player_id: String,
    pub feature_id: String,
    #[serde(default)]
    pub is_farmer: bool,
    #[serde(default)]
    pub kind: MeepleKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardTile {
    pub type_id: String,
    pub rotation: u8,
    pub x: i32,
    pub y: i32,
    pub meeple: Option<Meeple>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub color_key: String,
    pub hex: String,
    pub score: i32,
    pub meeples: u32,
    #[serde(default = "default_true")]
    pub has_big_meeple: bool,
    #[serde(default = "default_true")]
    pub has_builder: bool,
    #[serde(default = "default_true")]
    pub has_pig: bool,
    #[serde(default)]
    pub goods: TradeGoods,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GamePhase {
    Setup,
    PlaceTile,
    PlaceMeeple,
    GameOver,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: f64,
    pub text: String,
    pub tone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub phase: GamePhase,
    pub players: Vec<Player>,
    pub current_player_index: usize,
    pub board: HashMap<String, BoardTile>,
    pub deck: Vec<String>,
    #[serde(default)]
    pub river_deck: Vec<String>,
    #[serde(default)]
    pub is_river_phase: bool,
    #[serde(default)]
    pub river_enabled: bool,
    #[serde(default)]
    pub last_river_turn: Option<i8>,
    pub current_tile_type_id: Option<String>,
    pub current_rotation: u8,
    pub last_placed_coord: Option<String>,
    pub turn_number: u32,
    pub log: Vec<LogEntry>,
    pub farmers_enabled: bool,
    #[serde(default)]
    pub inns_cathedrals_enabled: bool,
    #[serde(default)]
    pub traders_builders_enabled: bool,
    #[serde(default)]
    pub builder_bonus_pending: bool,
    #[serde(default)]
    pub is_builder_bonus_turn: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeepleSpot {
    pub id: String,
    #[serde(rename = "type")]
    pub spot_type: String, // "city", "road", "monastery", "field"
    pub hotspot: Point,
    pub is_farmer: bool,
    #[serde(default)]
    pub allowed_kinds: Vec<MeepleKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreEvent {
    pub player_ids: Vec<String>,
    pub player_names: Vec<String>,
    pub points: i32,
    pub description: String,
    pub x: i32,
    pub y: i32,
    pub hex: String,
}
