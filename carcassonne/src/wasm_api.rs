use serde::Serialize;
use wasm_bindgen::prelude::*;
use crate::engine::CarcassonneEngine;
use crate::models::{GameState, MeepleKind, Player, TileDefinition};

#[wasm_bindgen]
pub struct CarcassonneEngineWasm {
    engine: CarcassonneEngine,
}

#[wasm_bindgen]
impl CarcassonneEngineWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        tiles_json: &str,
        players_json: &str,
        farmers_enabled: bool,
        river_enabled: Option<bool>,
        inns_cathedrals_enabled: Option<bool>,
        traders_builders_enabled: Option<bool>,
    ) -> Result<CarcassonneEngineWasm, JsValue> {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        let tiles: Vec<TileDefinition> = serde_json::from_str(tiles_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid tiles JSON: {}", e)))?;
        let players: Vec<Player> = serde_json::from_str(players_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid players JSON: {}", e)))?;

        let engine = CarcassonneEngine::new(
            tiles,
            players,
            farmers_enabled,
            river_enabled.unwrap_or(false),
            inns_cathedrals_enabled.unwrap_or(false),
            traders_builders_enabled.unwrap_or(false),
        );
        Ok(Self { engine })
    }

    #[wasm_bindgen]
    pub fn get_state(&self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        self.engine.state.serialize(&serializer)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn can_place_tile(&self, x: i32, y: i32, rotation: u8) -> bool {
        let current_tile = match &self.engine.state.current_tile_type_id {
            Some(id) => id,
            None => return false,
        };
        self.engine.can_place_tile_at(x, y, current_tile, rotation)
    }

    #[wasm_bindgen]
    pub fn get_valid_placements(&self, rotation: u8) -> Result<JsValue, JsValue> {
        let current_tile = match &self.engine.state.current_tile_type_id {
            Some(id) => id,
            None => return Ok(JsValue::NULL),
        };
        let placements = self.engine.get_valid_placements(current_tile, rotation);
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        placements.serialize(&serializer).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn place_tile(&mut self, x: i32, y: i32, rotation: u8) -> Result<JsValue, JsValue> {
        let needs_meeple = self
            .engine
            .place_tile(x, y, rotation)
            .map_err(|e| JsValue::from_str(&e))?;

        #[derive(serde::Serialize)]
        struct PlaceTileResult<'a> {
            needs_meeple: bool,
            state: &'a GameState,
        }

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        PlaceTileResult {
            needs_meeple,
            state: &self.engine.state,
        }
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn get_valid_meeple_spots(&self, x: i32, y: i32) -> Result<JsValue, JsValue> {
        let spots = self.engine.get_valid_meeple_spots(x, y);
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        spots.serialize(&serializer).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn place_meeple(
        &mut self,
        feature_id: Option<String>,
        meeple_kind: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let kind = match meeple_kind.as_deref() {
            None | Some("normal") | Some("Normal") => MeepleKind::Normal,
            Some("big") | Some("Big") => MeepleKind::Big,
            Some("builder") | Some("Builder") => MeepleKind::Builder,
            Some("pig") | Some("Pig") => MeepleKind::Pig,
            Some(other) => {
                return Err(JsValue::from_str(&format!("Unknown meeple kind: {}", other)));
            }
        };

        let score_events = self
            .engine
            .place_meeple(feature_id, kind)
            .map_err(|e| JsValue::from_str(&e))?;

        #[derive(serde::Serialize)]
        struct MeepleResult<'a> {
            score_events: Vec<crate::models::ScoreEvent>,
            state: &'a GameState,
        }

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        MeepleResult {
            score_events,
            state: &self.engine.state,
        }
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn trigger_game_over(&mut self) -> Result<JsValue, JsValue> {
        let score_events = self.engine.trigger_game_over();
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        score_events.serialize(&serializer).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn serialize_state(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.engine.state).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn import_state(&mut self, state_json: &str) -> Result<(), JsValue> {
        let state: GameState = serde_json::from_str(state_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid state JSON: {}", e)))?;
        self.engine.state = state;
        Ok(())
    }
}
