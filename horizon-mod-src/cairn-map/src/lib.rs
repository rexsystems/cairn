//! Cairn Map: corner minimap + toggleable larger map (Horizon `set_minimap`).
//! Not a full Xaero world map — same native map API, two sizes.
use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render::{self, Corner, Minimap as MapView},
    storage,
    ui::{self, ButtonWidget, Widget},
};

const KEY: &str = "map";
const CORNERS: [(Corner, &str, &str); 4] = [
    (Corner::TopLeft, "top_left", "Top left"),
    (Corner::TopRight, "top_right", "Top right"),
    (Corner::BottomLeft, "bottom_left", "Bottom left"),
    (Corner::BottomRight, "bottom_right", "Bottom right"),
];
const SIZES: [(u8, &str); 3] = [(64, "Small"), (96, "Medium"), (128, "Large")];
/// Blocks per pixel for the corner minimap.
const ZOOMS: [f32; 4] = [0.5, 1.0, 2.0, 4.0];
/// Large view uses max GUI size and a wide blocks-per-pixel.
const LARGE_SIZE: f32 = 256.0;
const LARGE_ZOOM: f32 = 4.0;

const CORNER_BUTTON: u32 = 1;
const SIZE_BUTTON: u32 = 2;
const SHAPE_BUTTON: u32 = 3;
const ZOOM_BUTTON: u32 = 4;
const ROTATE_BUTTON: u32 = 5;

struct CairnMap {
    corner: usize,
    size: usize,
    round: bool,
    zoom: usize,
    rotate: bool,
    /// Large map overlay toggled with the Map key.
    large: bool,
}

impl CairnMap {
    fn show(&self) {
        let (size, blocks_per_pixel) = if self.large {
            (LARGE_SIZE, LARGE_ZOOM)
        } else {
            (f32::from(SIZES[self.size].0), ZOOMS[self.zoom])
        };
        render::set_minimap(Some(MapView {
            corner: CORNERS[self.corner].0,
            size,
            blocks_per_pixel,
            rotate: self.rotate,
            round: self.round,
        }));
    }

    fn page(&self) {
        let button = |id, label: String| Widget::Button(ButtonWidget { id, label });
        let n = ZOOMS[self.zoom];
        let blocks = if n < 1.0 {
            "½ block".to_owned()
        } else if (n - 1.0).abs() < f32::EPSILON {
            "1 block".to_owned()
        } else {
            format!("{n} blocks")
        };
        ui::set_settings(&[
            button(CORNER_BUTTON, format!("Corner: {}", CORNERS[self.corner].2)),
            button(SIZE_BUTTON, format!("Minimap size: {}", SIZES[self.size].1)),
            button(
                SHAPE_BUTTON,
                format!("Shape: {}", if self.round { "Round" } else { "Square" }),
            ),
            button(ZOOM_BUTTON, format!("Minimap zoom: {blocks} / px")),
            button(
                ROTATE_BUTTON,
                format!("Turn with player: {}", if self.rotate { "ON" } else { "OFF" }),
            ),
        ]);
    }

    fn save(&self) {
        let _ = storage::set("corner", CORNERS[self.corner].1.as_bytes());
        let _ = storage::set("size", SIZES[self.size].0.to_string().as_bytes());
        let _ = storage::set("shape", if self.round { b"round" } else { b"square" });
        let _ = storage::set("zoom", ZOOMS[self.zoom].to_string().as_bytes());
        let _ = storage::set("rotate", if self.rotate { b"true" } else { b"false" });
    }
}

fn stored(key: &str) -> Option<String> {
    storage::get(key)
        .and_then(|v| String::from_utf8(v).ok())
        .map(|v| v.trim().to_owned())
}

impl Mod for CairnMap {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, "Large map", "key.keyboard.m")?;
        let find = |key: &str, values: &[String], default: usize| {
            stored(key)
                .and_then(|v| values.iter().position(|s| *s == v))
                .unwrap_or(default)
        };
        let corners: Vec<String> = CORNERS.iter().map(|c| c.1.to_owned()).collect();
        let sizes: Vec<String> = SIZES.iter().map(|s| s.0.to_string()).collect();
        let zooms: Vec<String> = ZOOMS.iter().map(f32::to_string).collect();
        let map = CairnMap {
            corner: find("corner", &corners, 1),
            size: find("size", &sizes, 1),
            round: stored("shape").as_deref() == Some("round"),
            zoom: find("zoom", &zooms, 1),
            rotate: stored("rotate").as_deref() == Some("true"),
            large: false,
        };
        map.show();
        map.page();
        Ok((map, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        for event in input::poll() {
            if let Event::KeyDown(k) = event {
                if k == KEY {
                    self.large = !self.large;
                    self.show();
                }
            }
        }
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        let next = |i: usize, n: usize| (i + 1) % n;
        match id {
            CORNER_BUTTON => self.corner = next(self.corner, CORNERS.len()),
            SIZE_BUTTON => self.size = next(self.size, SIZES.len()),
            SHAPE_BUTTON => self.round = !self.round,
            ZOOM_BUTTON => self.zoom = next(self.zoom, ZOOMS.len()),
            ROTATE_BUTTON => self.rotate = !self.rotate,
            _ => return,
        }
        self.save();
        self.show();
        self.page();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(CairnMap);
