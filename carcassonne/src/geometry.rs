use crate::models::{FieldNode, Point};

pub const DIRS: [(i32, i32); 4] = [
    (0, -1), // 0: North
    (1, 0),  // 1: East
    (0, 1),  // 2: South
    (-1, 0), // 3: West
];

pub fn coord_key(x: i32, y: i32) -> String {
    format!("{},{}", x, y)
}

pub fn parse_coord_key(key: &str) -> Option<(i32, i32)> {
    let mut parts = key.split(',');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    Some((x, y))
}

pub fn opposite_dir(dir: usize) -> usize {
    (dir + 2) % 4
}

/// Convert unrotated edge index to world edge index under tile rotation (0..3)
pub fn edge_to_world(edge: usize, rotation: u8) -> usize {
    (edge + rotation as usize) % 4
}

/// Convert world edge index to tile's internal unrotated edge index
pub fn world_to_edge(world_dir: usize, rotation: u8) -> usize {
    (world_dir + 4 - (rotation as usize % 4)) % 4
}

/// Rotate a 2D point (0..100) clockwise by rotation * 90 degrees
pub fn rotate_point(point: &Point, rotation: u8) -> Point {
    match rotation % 4 {
        0 => point.clone(),
        1 => Point {
            x: 100.0 - point.y,
            y: point.x,
        },
        2 => Point {
            x: 100.0 - point.x,
            y: 100.0 - point.y,
        },
        3 => Point {
            x: point.y,
            y: 100.0 - point.x,
        },
        _ => unreachable!(),
    }
}

/// Rotate a field node clockwise by 90 degrees * rotation
pub fn rotate_field_node(node: FieldNode, rotation: u8) -> FieldNode {
    let mut curr = node;
    for _ in 0..(rotation % 4) {
        curr = match curr {
            FieldNode::NW => FieldNode::EN,
            FieldNode::NE => FieldNode::ES,
            FieldNode::EN => FieldNode::SE,
            FieldNode::ES => FieldNode::SW,
            FieldNode::SE => FieldNode::WS,
            FieldNode::SW => FieldNode::WN,
            FieldNode::WS => FieldNode::NW,
            FieldNode::WN => FieldNode::NE,
            FieldNode::CENTER => FieldNode::CENTER,
        };
    }
    curr
}

/// For a world direction (0: North, 1: East, 2: South, 3: West), return the two
/// field nodes along that world edge on the current tile, and their corresponding
/// opposite field nodes on the neighboring tile in that direction.
pub fn matching_neighbor_field_nodes(world_dir: usize) -> &'static [(FieldNode, FieldNode)] {
    match world_dir {
        0 => &[
            (FieldNode::NW, FieldNode::SW),
            (FieldNode::NE, FieldNode::SE),
        ],
        1 => &[
            (FieldNode::EN, FieldNode::WN),
            (FieldNode::ES, FieldNode::WS),
        ],
        2 => &[
            (FieldNode::SW, FieldNode::NW),
            (FieldNode::SE, FieldNode::NE),
        ],
        3 => &[
            (FieldNode::WN, FieldNode::EN),
            (FieldNode::WS, FieldNode::ES),
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_key() {
        assert_eq!(coord_key(3, -5), "3,-5");
        assert_eq!(parse_coord_key("3,-5"), Some((3, -5)));
        assert_eq!(parse_coord_key("invalid"), None);
    }

    #[test]
    fn test_edge_rotations() {
        assert_eq!(edge_to_world(0, 1), 1);
        assert_eq!(world_to_edge(1, 1), 0);
        assert_eq!(edge_to_world(3, 2), 1);
        assert_eq!(world_to_edge(1, 2), 3);
    }

    #[test]
    fn test_rotate_field_nodes() {
        assert_eq!(rotate_field_node(FieldNode::NW, 1), FieldNode::EN);
        assert_eq!(rotate_field_node(FieldNode::NW, 4), FieldNode::NW);
        assert_eq!(rotate_field_node(FieldNode::CENTER, 3), FieldNode::CENTER);
    }

    #[test]
    fn test_rotate_point() {
        let p = Point { x: 50.0, y: 16.0 };
        let r1 = rotate_point(&p, 1);
        assert_eq!(r1, Point { x: 84.0, y: 50.0 });
    }
}
