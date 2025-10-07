use macroquad::prelude::*;
 // For the Command in AUDIO OUTPUT logic

// Import types and functions from your other modules

/// Removes the file extension from a filename string slice.
pub fn trim_extension(filename: &str) -> &str {
    if let Some(dot_index) = filename.rfind('.') {
        &filename[..dot_index]
    } else {
        filename
    }
}

pub fn string_to_color(color_str: &str) -> Color {
    match color_str {
        "PINK" => PINK,
        "RED" => RED,
        "ORANGE" => ORANGE,
        "YELLOW" => YELLOW,
        "GREEN" => GREEN,
        "BLUE" => BLUE,
        "PURPLE" => VIOLET, // USING VIOLET AS A CLOSE APPROXIMATION
        _ => WHITE, // Default to WHITE
    }
}

/// Parses a resolution string and requests a window resize.
pub fn apply_resolution(resolution_str: &str) {
    if let Some((w_str, h_str)) = resolution_str.split_once('x') {
        // Parse to f32 for the resize function
        if let (Ok(w), Ok(h)) = (w_str.parse::<f32>(), h_str.parse::<f32>()) {
            // Use the correct function name
            request_new_screen_size(w, h);
        }
    }
}
