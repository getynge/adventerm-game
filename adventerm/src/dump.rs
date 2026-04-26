use std::path::Path;

use adventerm_lib::{Dungeon, Room, TileKind};

pub fn run(seed: u64, count: usize, path: &Path) -> std::io::Result<()> {
    let dungeon = Dungeon::generate_with_room_count(seed, count);
    std::fs::write(path, format_dungeon(&dungeon))
}

fn format_dungeon(d: &Dungeon) -> String {
    let mut out = format!("Dungeon seed={} rooms={}\n", d.seed, d.rooms.len());
    for room in &d.rooms {
        out.push_str(&format!(
            "\n=== Room {} ({}x{}) ===\n",
            room.id.0, room.width, room.height
        ));
        for y in 0..room.height {
            for x in 0..room.width {
                out.push(glyph_for(room, x, y));
            }
            out.push('\n');
        }
    }
    out
}

fn glyph_for(room: &Room, x: usize, y: usize) -> char {
    match room.kind_at(x, y) {
        Some(TileKind::Wall) | None => '#',
        Some(TileKind::Floor) => '.',
        Some(TileKind::Door(_)) => '+',
    }
}
