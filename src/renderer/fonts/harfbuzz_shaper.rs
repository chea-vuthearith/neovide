// Hybrid shaping module: Use rustybuzz for complex scripts, Swash for Latin
// This module provides HarfBuzz-based shaping for scripts that Swash doesn't handle well

use rustybuzz::{Direction, Face, UnicodeBuffer};
use swash::text::Script;

pub fn needs_harfbuzz(script: Script) -> bool {
    matches!(
        script,
        Script::Khmer
            | Script::Arabic
            | Script::Thai
            | Script::Lao
            | Script::Devanagari
            | Script::Bengali
            | Script::Tamil
            | Script::Telugu
            | Script::Myanmar
            | Script::Tibetan
            | Script::Sinhala
            | Script::Hebrew
    )
}

pub fn script_to_rustybuzz(script: Script) -> rustybuzz::Script {
    match script {
        Script::Khmer => rustybuzz::Script::Khmer,
        Script::Arabic => rustybuzz::Script::Arabic,
        Script::Thai => rustybuzz::Script::Thai,
        Script::Lao => rustybuzz::Script::Lao,
        Script::Devanagari => rustybuzz::Script::Devanagari,
        Script::Bengali => rustybuzz::Script::Bengali,
        Script::Tamil => rustybuzz::Script::Tamil,
        Script::Telugu => rustybuzz::Script::Telugu,
        Script::Myanmar => rustybuzz::Script::Myanmar,
        Script::Tibetan => rustybuzz::Script::Tibetan,
        Script::Sinhala => rustybuzz::Script::Sinhala,
        Script::Hebrew => rustybuzz::Script::Hebrew,
        _ => rustybuzz::Script::Latin,
    }
}

pub struct HarfBuzzGlyph {
    pub glyph_id: u32,
    pub cluster: u32,
    pub x_advance: i32,
    pub y_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

pub fn shape_with_harfbuzz(
    font_data: &[u8],
    text: &str,
    script: Script,
    size: f32,
) -> Option<Vec<HarfBuzzGlyph>> {
    // Create HarfBuzz face from font data
    let face = Face::from_slice(font_data, 0)?;
    
    // Create buffer and add text
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(Direction::LeftToRight);
    buffer.set_script(script_to_rustybuzz(script));
    
    // Shape the text
    let output = rustybuzz::shape(&face, &[], buffer);
    
    // Extract glyph information
    let positions = output.glyph_positions();
    let infos = output.glyph_infos();
    
    let scale = size / face.units_per_em() as f32;
    
    Some(
        infos
            .iter()
            .zip(positions.iter())
            .map(|(info, pos)| HarfBuzzGlyph {
                glyph_id: info.glyph_id,
                cluster: info.cluster,
                x_advance: (pos.x_advance as f32 * scale) as i32,
                y_advance: (pos.y_advance as f32 * scale) as i32,
                x_offset: (pos.x_offset as f32 * scale) as i32,
                y_offset: (pos.y_offset as f32 * scale) as i32,
            })
            .collect(),
    )
}
