use crate::{Color, draw_text_ex, TextParams, HashMap, Font, Config, FONT_SIZE, string_to_color};

/// Looks up the currently selected font in the cache.
/// Falls back to the "Default" font if the selection is not found.
pub fn get_current_font<'a>(
    font_cache: &'a HashMap<String, Font>,
    config: &Config,
) -> &'a Font {
    font_cache
    .get(&config.font_selection)
    .unwrap_or_else(|| &font_cache["Default"])
}

// A new function specifically for drawing text that respects the config color
pub fn text_with_config_color(font_cache: &HashMap<String, Font>, config: &Config, text: &str, x: f32, y: f32, font_size: u16) {
    let font = get_current_font(font_cache, config);

    // Shadow should scale with font size
    let shadow_offset = 1.0 * (font_size as f32 / FONT_SIZE as f32);

    // Shadow
    draw_text_ex(text, x + shadow_offset, y + shadow_offset, TextParams {
        font: Some(font),
        font_size,
        color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.9 },
        ..Default::default()
    });

    // Main Text (using the color from config)
    draw_text_ex(text, x, y, TextParams {
        font: Some(font),
        font_size,
        color: string_to_color(&config.font_color),
        ..Default::default()
    });
}

pub fn text_disabled(font_cache: &HashMap<String, Font>, config: &Config, text : &str, x : f32, y: f32, font_size: u16) {
    let font = get_current_font(font_cache, config);
    let shadow_offset = 1.0 * (font_size as f32 / FONT_SIZE as f32);

    // SHADOW
    draw_text_ex(text, x + shadow_offset, y + shadow_offset, TextParams {
        font: Some(font),
        //font_size: font_size,
        font_size,
        color: Color {r:0.0, g:0.0, b:0.0, a:1.0},
        ..Default::default()
    });

    // MAIN TEXT
    draw_text_ex(text, x, y, TextParams {
        font: Some(font),
        //font_size: font_size,
        font_size,
        color: Color {r:0.4, g:0.4, b:0.4, a:1.0},
        ..Default::default()
    });
}
