//! Cairn Map — minimap + Xaero-style checkpoints. New Checkpoint screen sets
//! name (type letter-by-letter), color, icon, temp, visible, place — no chat.
use horizon_mod_sdk::{
    Mod, Registration, chat, game,
    hud::{self, Corner as HudCorner, Line},
    input::{self, Event},
    render::{
        self, Corner, FillShape, Minimap as MapView, Point, QuadShape, Rect, Shape, TextShape,
    },
    storage,
    ui::{self, ButtonWidget, ListWidget, Screen, TextWidget, Widget},
};

const KEY_MAP: &str = "map";
const KEY_WAYPOINT: &str = "waypoint";
const KEY_LIST: &str = "waypoints";
const KEY_HIDE: &str = "hide";

const CORNERS: [(Corner, &str, &str); 4] = [
    (Corner::TopLeft, "top_left", "Top left"),
    (Corner::TopRight, "top_right", "Top right"),
    (Corner::BottomLeft, "bottom_left", "Bottom left"),
    (Corner::BottomRight, "bottom_right", "Bottom right"),
];
const SIZES: [(u8, &str); 3] = [(64, "Small"), (96, "Medium"), (128, "Large")];
const ZOOMS: [f32; 4] = [0.5, 1.0, 2.0, 4.0];
const LARGE_SIZE: f32 = 256.0;
const LARGE_ZOOMS: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
const MAP_TEX: f32 = 128.0;
const MARGIN: f32 = 2.0;
const MAX_WAYPOINTS: usize = 64;

const NAMES: &[&str] = &[
    "Home", "Base", "Mine", "Portal", "Farm", "Village", "Camp", "Nether", "End", "Cache",
    "WP",
];
/// Xaero-ish palette: (label, 0xAARRGGBB fill, 0xRRGGBB text).
const COLORS: &[(&str, u32, u32)] = &[
    ("White", 0xFFFFFFFF, 0xFFFFFF),
    ("Red", 0xFFFF5555, 0xFF5555),
    ("Orange", 0xFFFFAA00, 0xFFAA00),
    ("Yellow", 0xFFFFFF55, 0xFFFF55),
    ("Lime", 0xFF55FF55, 0x55FF55),
    ("Green", 0xFF00AA00, 0x00AA00),
    ("Cyan", 0xFF55FFFF, 0x55FFFF),
    ("Blue", 0xFF5555FF, 0x5555FF),
    ("Purple", 0xFFAA00AA, 0xAA00AA),
    ("Magenta", 0xFFFF55FF, 0xFF55FF),
    ("Pink", 0xFFFFAAAA, 0xFFAAAA),
    ("Gray", 0xFFAAAAAA, 0xAAAAAA),
    ("Black", 0xFF222222, 0x555555),
    ("Brown", 0xFF8B4513, 0xCD853F),
];
const ICONS: &[&str] = &["Diamond", "Square", "Circle", "Triangle", "Star", "Skull", "House", "Cross"];
/// Horizon has no text box — type names from this set on the New screen.
const LETTERS: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
    'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
    'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1',
    '2', '3', '4', '5', '6', '7', '8', '9', ' ', '-', '_',
];

const CORNER_BUTTON: u32 = 1;
const SIZE_BUTTON: u32 = 2;
const SHAPE_BUTTON: u32 = 3;
const ZOOM_BUTTON: u32 = 4;
const ROTATE_BUTTON: u32 = 5;
const COORDS_BUTTON: u32 = 6;
const MARKERS_BUTTON: u32 = 7;

const WP_LIST: u32 = 20;
const WP_EDIT: u32 = 21;
const WP_ACTIVE: u32 = 22;
const WP_DELETE: u32 = 23;
const WP_CLEAR_DEATH: u32 = 24;
const WP_NEW: u32 = 25;

const CR_PRESET: u32 = 30;
const CR_LETTER: u32 = 31;
const CR_ADD: u32 = 32;
const CR_BACK: u32 = 33;
const CR_CLEAR: u32 = 34;
const CR_NEXT: u32 = 35;
const CR_COLOR: u32 = 36;
const CR_ICON: u32 = 37;
const CR_TEMP: u32 = 38;
const CR_VISIBLE: u32 = 39;
const CR_PLACE: u32 = 40;
const CR_CREATE: u32 = 41;
const CR_BACK_PAGE: u32 = 42;

const ED_PRESET: u32 = 50;
const ED_LETTER: u32 = 51;
const ED_ADD: u32 = 52;
const ED_BACK: u32 = 53;
const ED_CLEAR: u32 = 54;
const ED_COLOR: u32 = 55;
const ED_ICON: u32 = 56;
const ED_TEMP: u32 = 57;
const ED_VISIBLE: u32 = 58;
const ED_HERE: u32 = 59;
const ED_DONE: u32 = 60;

#[derive(Clone)]
struct Waypoint {
    name: String,
    x: f64,
    y: f64,
    z: f64,
    dim: String,
    color: usize,
    icon: usize,
    temporary: bool,
    visible: bool,
}

enum UiMode {
    None,
    List,
    /// New checkpoint — name step (types on-screen).
    CreateName,
    /// New checkpoint — color/icon/place step.
    CreateStyle,
    Edit(usize),
}

struct CairnMap {
    corner: usize,
    size: usize,
    round: bool,
    zoom: usize,
    rotate: bool,
    large: bool,
    large_zoom: usize,
    hidden: bool,
    show_coords: bool,
    show_markers: bool,
    waypoints: Vec<Waypoint>,
    active: Option<usize>,
    last_health: f32,
    list_sel: Option<u32>,
    last_hud: Vec<String>,
    mode: UiMode,
    /// Draft for Create / Edit — all set on that screen.
    draft_name: usize,
    draft_typed: String,
    draft_letter: usize,
    draft_color: usize,
    draft_icon: usize,
    draft_temp: bool,
    draft_visible: bool,
    /// Create place: false = feet, true = look target.
    draft_at_look: bool,
    /// Auto-increment for "WP N".
    wp_serial: u32,
}

impl CairnMap {
    fn map_size_bpp(&self) -> (f32, f32) {
        if self.large {
            (LARGE_SIZE, LARGE_ZOOMS[self.large_zoom])
        } else {
            (f32::from(SIZES[self.size].0), ZOOMS[self.zoom])
        }
    }

    fn show_map(&self) {
        if self.hidden && !self.large {
            render::set_minimap(None);
            return;
        }
        let (size, blocks_per_pixel) = self.map_size_bpp();
        render::set_minimap(Some(MapView {
            corner: CORNERS[self.corner].0,
            size,
            blocks_per_pixel,
            rotate: self.rotate,
            round: self.round,
        }));
    }

    fn settings_page(&self) {
        let button = |id, label: String| Widget::Button(ButtonWidget { id, label });
        let n = ZOOMS[self.zoom];
        let blocks = if n < 1.0 {
            "½ block".into()
        } else if (n - 1.0).abs() < f32::EPSILON {
            "1 block".into()
        } else {
            format!("{n} blocks")
        };
        let on = |b: bool| if b { "ON" } else { "OFF" };
        ui::set_settings(&[
            button(CORNER_BUTTON, format!("Corner: {}", CORNERS[self.corner].2)),
            button(SIZE_BUTTON, format!("Minimap size: {}", SIZES[self.size].1)),
            button(
                SHAPE_BUTTON,
                format!("Shape: {}", if self.round { "Round" } else { "Square" }),
            ),
            button(ZOOM_BUTTON, format!("Minimap zoom: {blocks} / px")),
            button(ROTATE_BUTTON, format!("Turn with player: {}", on(self.rotate))),
            button(COORDS_BUTTON, format!("Coords HUD: {}", on(self.show_coords))),
            button(
                MARKERS_BUTTON,
                format!("Checkpoint markers: {}", on(self.show_markers)),
            ),
        ]);
    }

    fn save_settings(&self) {
        let _ = storage::set("corner", CORNERS[self.corner].1.as_bytes());
        let _ = storage::set("size", SIZES[self.size].0.to_string().as_bytes());
        let _ = storage::set("shape", if self.round { b"round" } else { b"square" });
        let _ = storage::set("zoom", ZOOMS[self.zoom].to_string().as_bytes());
        let _ = storage::set("rotate", if self.rotate { b"true" } else { b"false" });
        let _ = storage::set(
            "coords",
            if self.show_coords { b"true" } else { b"false" },
        );
        let _ = storage::set(
            "markers",
            if self.show_markers { b"true" } else { b"false" },
        );
        let _ = storage::set("serial", self.wp_serial.to_string().as_bytes());
    }

    fn save_waypoints(&self) {
        let mut out = String::new();
        for w in &self.waypoints {
            out.push_str(&format!(
                "{}|{:.3}|{:.3}|{:.3}|{}|{}|{}|{}|{}\n",
                w.name.replace('|', "_"),
                w.x,
                w.y,
                w.z,
                w.dim,
                w.color,
                w.icon,
                if w.temporary { 1 } else { 0 },
                if w.visible { 1 } else { 0 },
            ));
        }
        let _ = storage::set("waypoints", out.as_bytes());
        if let Some(i) = self.active {
            let _ = storage::set("active", i.to_string().as_bytes());
        } else {
            let _ = storage::delete("active");
        }
    }

    fn load_waypoints() -> (Vec<Waypoint>, Option<usize>) {
        let mut waypoints = Vec::new();
        if let Some(raw) = storage::get("waypoints").and_then(|b| String::from_utf8(b).ok()) {
            for line in raw.lines() {
                let p: Vec<&str> = line.split('|').collect();
                if p.len() < 5 {
                    continue;
                }
                let (Ok(x), Ok(y), Ok(z)) = (p[1].parse(), p[2].parse(), p[3].parse()) else {
                    continue;
                };
                let color = p.get(5).and_then(|s| s.parse().ok()).unwrap_or(1) % COLORS.len();
                let icon = p.get(6).and_then(|s| s.parse().ok()).unwrap_or(0) % ICONS.len();
                let temporary = p.get(7).map(|s| *s == "1").unwrap_or(false);
                let visible = p.get(8).map(|s| *s != "0").unwrap_or(true);
                waypoints.push(Waypoint {
                    name: p[0].into(),
                    x,
                    y,
                    z,
                    dim: p[4].into(),
                    color,
                    icon,
                    temporary,
                    visible,
                });
                if waypoints.len() >= MAX_WAYPOINTS {
                    break;
                }
            }
        }
        let active = storage::get("active")
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|s| s.trim().parse().ok())
            .filter(|&i: &usize| i < waypoints.len());
        (waypoints, active)
    }

    fn draft_label(&self) -> String {
        if NAMES[self.draft_name] == "WP" {
            format!("WP {}", self.wp_serial)
        } else {
            NAMES[self.draft_name].into()
        }
    }

    fn letter_label(&self) -> String {
        let c = LETTERS[self.draft_letter % LETTERS.len()];
        if c == ' ' {
            "Space".into()
        } else {
            c.to_string()
        }
    }

    fn append_letter(&mut self) {
        if self.draft_typed.chars().count() >= 32 {
            return;
        }
        self.draft_typed
            .push(LETTERS[self.draft_letter % LETTERS.len()]);
    }

    fn backspace_letter(&mut self) {
        let mut t = self.draft_typed.clone();
        t.pop();
        self.draft_typed = t;
    }

    fn open_create(&mut self) {
        self.mode = UiMode::CreateName;
        self.draft_name = 0;
        self.draft_typed = self.draft_label();
        self.draft_letter = 0;
        self.draft_color = 1;
        self.draft_icon = 0;
        self.draft_temp = false;
        self.draft_visible = true;
        self.draft_at_look = false;
        self.paint_create_name();
    }

    fn paint_create_name(&self) {
        let shown = if self.draft_typed.is_empty() {
            "(empty) — type below".into()
        } else {
            format!("{}▌", self.draft_typed)
        };
        ui::open_screen(&Screen {
            title: "New Checkpoint — Name".into(),
            widgets: vec![
                Widget::Text(TextWidget {
                    id: 1,
                    text: format!("Name: {shown}"),
                    color: 0xFFFFFF,
                }),
                Widget::Button(ButtonWidget {
                    id: CR_PRESET,
                    label: format!("Preset: {}", self.draft_label()),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_LETTER,
                    label: format!("Letter: {}", self.letter_label()),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_ADD,
                    label: "+ Add letter".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_BACK,
                    label: "« Backspace".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_NEXT,
                    label: "Next: style & place →".into(),
                }),
            ],
        });
    }

    fn flags_label(&self) -> String {
        match (self.draft_visible, self.draft_temp) {
            (true, false) => "Visible · Permanent".into(),
            (true, true) => "Visible · Temporary".into(),
            (false, false) => "Hidden · Permanent".into(),
            (false, true) => "Hidden · Temporary".into(),
        }
    }

    fn paint_create_style(&self) {
        let place = if self.draft_at_look {
            "Look target"
        } else {
            "My feet"
        };
        ui::open_screen(&Screen {
            title: "New Checkpoint — Style".into(),
            widgets: vec![
                Widget::Button(ButtonWidget {
                    id: CR_BACK_PAGE,
                    label: "← Back to name".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_COLOR,
                    label: format!("Color: {}", COLORS[self.draft_color].0),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_ICON,
                    label: format!("Icon: {}", ICONS[self.draft_icon]),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_TEMP,
                    label: format!("Flags: {}", self.flags_label()),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_PLACE,
                    label: format!("Place: {place}"),
                }),
                Widget::Button(ButtonWidget {
                    id: CR_CREATE,
                    label: "★ Create checkpoint".into(),
                }),
            ],
        });
    }

    fn open_list(&mut self) {
        self.mode = UiMode::List;
        let rows: Vec<String> = if self.waypoints.is_empty() {
            vec!["(none — press B for a checkpoint)".into()]
        } else {
            self.waypoints
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let star = if Some(i) == self.active { "★ " } else { "  " };
                    let hide = if w.visible { "" } else { " [hidden]" };
                    let tmp = if w.temporary { " ~" } else { "" };
                    format!(
                        "{star}{} · {} {}{tmp}{hide}  ({:.0}, {:.0}, {:.0}) {}",
                        w.name,
                        COLORS[w.color % COLORS.len()].0,
                        ICONS[w.icon % ICONS.len()],
                        w.x.floor(),
                        w.y.floor(),
                        w.z.floor(),
                        short_dim(&w.dim)
                    )
                })
                .collect()
        };
        let sel = self
            .list_sel
            .filter(|&r| (r as usize) < rows.len().max(1))
            .or(Some(0));
        self.list_sel = sel;
        ui::open_screen(&Screen {
            title: "Checkpoints".into(),
            widgets: vec![
                Widget::Text(TextWidget {
                    id: 1,
                    text: "Pick one, then Edit / Set active / Delete.".into(),
                    color: 0xAAAAAA,
                }),
                Widget::List(ListWidget {
                    id: WP_LIST,
                    rows,
                    selected: sel,
                }),
                Widget::Button(ButtonWidget {
                    id: WP_NEW,
                    label: "New checkpoint".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: WP_EDIT,
                    label: "Edit selected".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: WP_ACTIVE,
                    label: "Set active".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: WP_DELETE,
                    label: "Delete selected".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: WP_CLEAR_DEATH,
                    label: "Clear death point".into(),
                }),
            ],
        });
    }

    fn open_edit(&mut self, i: usize) {
        let Some(w) = self.waypoints.get(i) else {
            self.open_list();
            return;
        };
        self.mode = UiMode::Edit(i);
        self.draft_typed = w.name.clone();
        self.draft_letter = 0;
        self.draft_color = w.color % COLORS.len();
        self.draft_icon = w.icon % ICONS.len();
        self.draft_temp = w.temporary;
        self.draft_visible = w.visible;
        self.draft_name = NAMES
            .iter()
            .position(|&n| n == w.name || (n == "WP" && w.name.starts_with("WP ")))
            .unwrap_or(NAMES.len() - 1);
        self.paint_edit(i);
    }

    fn paint_edit(&self, i: usize) {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        let shown = if self.draft_typed.is_empty() {
            "(empty)".into()
        } else {
            format!("{}▌", self.draft_typed)
        };
        ui::open_screen(&Screen {
            title: format!("Edit checkpoint #{i}"),
            widgets: vec![
                Widget::Text(TextWidget {
                    id: 1,
                    text: format!("Name: {shown}"),
                    color: 0xFFFFFF,
                }),
                Widget::Button(ButtonWidget {
                    id: ED_PRESET,
                    label: format!("Fill preset: {}", self.draft_label()),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_LETTER,
                    label: format!("Letter: {}", self.letter_label()),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_ADD,
                    label: "Add letter".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_BACK,
                    label: "Backspace".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_CLEAR,
                    label: "Clear name".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_COLOR,
                    label: format!("Color: {}", COLORS[self.draft_color].0),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_ICON,
                    label: format!("Icon: {}", ICONS[self.draft_icon]),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_TEMP,
                    label: format!("Temporary: {}", on(self.draft_temp)),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_VISIBLE,
                    label: format!("Visible: {}", on(self.draft_visible)),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_HERE,
                    label: "Move to me".into(),
                }),
                Widget::Button(ButtonWidget {
                    id: ED_DONE,
                    label: "Save".into(),
                }),
            ],
        });
    }

    fn sync_edit_name(&mut self, i: usize) {
        let name = if self.draft_typed.trim().is_empty() {
            "Checkpoint".into()
        } else {
            self.draft_typed.chars().take(32).collect()
        };
        if let Some(w) = self.waypoints.get_mut(i) {
            w.name = name;
            w.color = self.draft_color;
            w.icon = self.draft_icon;
            w.temporary = self.draft_temp;
            w.visible = self.draft_visible;
        }
        self.save_waypoints();
    }

    /// Place from the New screen. `at` = feet, or look-hit block center.
    fn create_from_draft(&mut self, at: Option<(f64, f64, f64)>) {
        let Some(p) = game::get_player() else {
            chat::show("Not in a world.");
            return;
        };
        if self.waypoints.len() >= MAX_WAYPOINTS {
            chat::show("Checkpoint limit (64).");
            return;
        }
        let name = if self.draft_typed.trim().is_empty() {
            let n = self.draft_label();
            if NAMES[self.draft_name] == "WP" {
                self.wp_serial = self.wp_serial.saturating_add(1);
                self.save_settings();
            }
            n
        } else {
            self.draft_typed.chars().take(32).collect()
        };
        let (x, y, z) = at.unwrap_or((p.x, p.y, p.z));
        self.waypoints.push(Waypoint {
            name,
            x,
            y,
            z,
            dim: p.dimension.clone(),
            color: self.draft_color,
            icon: self.draft_icon,
            temporary: self.draft_temp,
            visible: self.draft_visible,
        });
        self.active = Some(self.waypoints.len() - 1);
        self.save_waypoints();
        chat::show(&format!(
            "Checkpoint \"{}\" @ {:.0} {:.0} {:.0}",
            self.waypoints.last().unwrap().name,
            x.floor(),
            y.floor(),
            z.floor()
        ));
        ui::close_screen();
        self.mode = UiMode::None;
    }

    fn create_at_look(&mut self) {
        let Some(hit) = game::raycast(128.0) else {
            chat::show("Nothing in sight — Create here instead.");
            return;
        };
        self.create_from_draft(Some((
            f64::from(hit.block_x) + 0.5,
            f64::from(hit.block_y) + 0.5,
            f64::from(hit.block_z) + 0.5,
        )));
    }

    fn mark_death(&mut self) {
        let Some(p) = game::get_player() else { return };
        self.waypoints.retain(|w| w.name != "Death");
        if self.waypoints.len() >= MAX_WAYPOINTS {
            self.waypoints.pop();
        }
        self.waypoints.push(Waypoint {
            name: "Death".into(),
            x: p.x,
            y: p.y,
            z: p.z,
            dim: p.dimension.clone(),
            color: 1,
            icon: 5,
            temporary: true,
            visible: true,
        });
        self.active = Some(self.waypoints.len() - 1);
        self.save_waypoints();
        chat::show("Death checkpoint saved.");
    }

    fn refresh_hud(&mut self) {
        if !self.show_coords {
            if !self.last_hud.is_empty() {
                hud::clear();
                self.last_hud.clear();
            }
            return;
        }
        let Some(p) = game::get_player() else {
            if !self.last_hud.is_empty() {
                hud::clear();
                self.last_hud.clear();
            }
            return;
        };
        let mut lines = vec![
            format!(
                "XYZ {:.0} / {:.0} / {:.0}",
                p.x.floor(),
                p.y.floor(),
                p.z.floor()
            ),
            format!("{}  {}", short_dim(&p.dimension), facing(p.yaw)),
        ];
        if let Some(i) = self.active {
            if let Some(w) = self.waypoints.get(i) {
                if w.dim == p.dimension {
                    let dx = w.x - p.x;
                    let dz = w.z - p.z;
                    let dist = (dx * dx + dz * dz).sqrt();
                    lines.push(format!(
                        "→ {}  {:.0}m  {}",
                        w.name,
                        dist,
                        bearing(dx, dz)
                    ));
                } else {
                    lines.push(format!("→ {}  (other dim)", w.name));
                }
            }
        }
        if self.large {
            lines.push(format!(
                "LARGE  {:.0} b/px  (scroll)",
                LARGE_ZOOMS[self.large_zoom]
            ));
        }
        if lines == self.last_hud {
            return;
        }
        let panel: Vec<Line> = lines
            .iter()
            .map(|text| Line {
                text: text.clone(),
                color: 0xFFE08A,
            })
            .collect();
        hud::set_panel(HudCorner::TopLeft, &panel);
        self.last_hud = lines;
    }

    /// Diamonds on the minimap + in-view markers (Xaero-style).
    fn draw_markers(&self) {
        if !self.show_markers || (self.hidden && !self.large) {
            render::set_hud(&[]);
            return;
        }
        let Some(p) = game::get_player() else {
            render::set_hud(&[]);
            return;
        };
        let (gui_w, gui_h) = render::gui_size();
        let (size, bpp) = self.map_size_bpp();
        let (map_x, map_y) = map_origin(CORNERS[self.corner].0, size, gui_w, gui_h);
        let (r_axis, f_axis) = map_axes(self.rotate, p.yaw);
        let scale = size / MAP_TEX;
        let mut shapes = Vec::new();

        for (i, w) in self.waypoints.iter().enumerate() {
            if !w.visible || w.dim != p.dimension {
                continue;
            }
            let dx = w.x - p.x;
            let dy = w.y - p.y;
            let dz = w.z - p.z;
            let dist = (dx * dx + dz * dz).sqrt();
            let color = COLORS[w.color % COLORS.len()].1;
            let text_c = COLORS[w.color % COLORS.len()].2;

            // Map-space u (right), v (down).
            let u = (r_axis[0] * dx + r_axis[1] * dz) / f64::from(bpp);
            let v = -(f_axis[0] * dx + f_axis[1] * dz) / f64::from(bpp);
            let mx = map_x + size / 2.0 + u as f32 * scale;
            let my = map_y + size / 2.0 + v as f32 * scale;
            let half = size / 2.0;
            let cx = map_x + half;
            let cy = map_y + half;
            let on_map = if self.round {
                let ox = mx - cx;
                let oy = my - cy;
                (ox * ox + oy * oy).sqrt() <= half - 3.0
            } else {
                mx >= map_x + 3.0
                    && mx <= map_x + size - 3.0
                    && my >= map_y + 3.0
                    && my <= map_y + size - 3.0
            };
            if on_map {
                push_icon(&mut shapes, w.icon, mx, my, 4.0, color);
                if Some(i) == self.active || dist < 80.0 {
                    shapes.push(Shape::Text(TextShape {
                        text: w.name.chars().take(12).collect(),
                        at: Point {
                            x: mx,
                            y: my - 8.0,
                        },
                        color: text_c,
                        centered: true,
                        shadow: true,
                    }));
                }
            } else {
                // Edge pin on the minimap rim.
                let ang = (my - cy).atan2(mx - cx);
                let reach = half - 5.0;
                let ex = cx + ang.cos() * reach;
                let ey = cy + ang.sin() * reach;
                push_icon(&mut shapes, w.icon, ex, ey, 3.5, color);
            }

            // In-view diamond (center of screen when looking toward it).
            let target_yaw = (-dx).atan2(dz).to_degrees() as f32;
            let mut dyaw = target_yaw - p.yaw;
            while dyaw > 180.0 {
                dyaw -= 360.0;
            }
            while dyaw < -180.0 {
                dyaw += 360.0;
            }
            let horiz = dist.max(0.001);
            let target_pitch = (-dy).atan2(horiz).to_degrees() as f32;
            let dpitch = target_pitch - p.pitch;
            if dyaw.abs() < 55.0 && dpitch.abs() < 40.0 && dist > 2.0 {
                let sx = gui_w / 2.0 + dyaw / 55.0 * (gui_w * 0.35);
                let sy = gui_h / 2.0 + dpitch / 40.0 * (gui_h * 0.35);
                push_icon(&mut shapes, w.icon, sx, sy, 6.0, color);
                shapes.push(Shape::Text(TextShape {
                    text: format!("{} [{:.0}m]", w.name, dist),
                    at: Point {
                        x: sx,
                        y: sy + 8.0,
                    },
                    color: text_c,
                    centered: true,
                    shadow: true,
                }));
            }
        }
        render::set_hud(&shapes);
    }
}

fn map_origin(corner: Corner, size: f32, gui_w: f32, gui_h: f32) -> (f32, f32) {
    let left = matches!(corner, Corner::TopLeft | Corner::BottomLeft);
    let top = matches!(corner, Corner::TopLeft | Corner::TopRight);
    let x = if left {
        MARGIN
    } else {
        gui_w - MARGIN - size
    };
    let y = if top {
        MARGIN
    } else {
        gui_h - MARGIN - size
    };
    (x, y)
}

fn map_axes(rotate: bool, yaw: f32) -> ([f64; 2], [f64; 2]) {
    if !rotate {
        return ([1.0, 0.0], [0.0, -1.0]);
    }
    let (s, c) = f64::from(yaw).to_radians().sin_cos();
    ([-c, -s], [-s, c])
}

fn push_icon(shapes: &mut Vec<Shape>, icon: usize, x: f32, y: f32, r: f32, color: u32) {
    match icon % ICONS.len() {
        1 => shapes.push(Shape::Fill(FillShape {
            rect: Rect {
                x: x - r,
                y: y - r,
                w: r * 2.0,
                h: r * 2.0,
            },
            color,
        })),
        2 => {
            // Soft circle from a small fill + diamond core.
            shapes.push(Shape::Fill(FillShape {
                rect: Rect {
                    x: x - r * 0.7,
                    y: y - r * 0.7,
                    w: r * 1.4,
                    h: r * 1.4,
                },
                color,
            }));
        }
        3 => {
            let p = |dx, dy| Point {
                x: x + dx,
                y: y + dy,
            };
            shapes.push(Shape::Quad(QuadShape {
                corners: (p(0.0, -r), p(r, r), p(0.0, r * 0.4), p(-r, r)),
                color,
            }));
        }
        4 | 7 => {
            shapes.push(Shape::Fill(FillShape {
                rect: Rect {
                    x: x - r,
                    y: y - r * 0.3,
                    w: r * 2.0,
                    h: r * 0.6,
                },
                color,
            }));
            shapes.push(Shape::Fill(FillShape {
                rect: Rect {
                    x: x - r * 0.3,
                    y: y - r,
                    w: r * 0.6,
                    h: r * 2.0,
                },
                color,
            }));
        }
        5 => shapes.push(Shape::Text(TextShape {
            text: "X".into(),
            at: Point { x, y: y - 3.0 },
            color: color & 0xFFFFFF,
            centered: true,
            shadow: true,
        })),
        6 => shapes.push(Shape::Text(TextShape {
            text: "H".into(),
            at: Point { x, y: y - 3.0 },
            color: color & 0xFFFFFF,
            centered: true,
            shadow: true,
        })),
        _ => {
            // Diamond — Xaero default.
            let p = |dx, dy| Point {
                x: x + dx,
                y: y + dy,
            };
            shapes.push(Shape::Quad(QuadShape {
                corners: (p(0.0, -r), p(r, 0.0), p(0.0, r), p(-r, 0.0)),
                color,
            }));
        }
    }
}

fn short_dim(dim: &str) -> &str {
    match dim {
        "minecraft:overworld" => "Overworld",
        "minecraft:the_nether" => "Nether",
        "minecraft:the_end" => "End",
        other => other.rsplit(':').next().unwrap_or(other),
    }
}

fn facing(yaw: f32) -> &'static str {
    let i = (yaw / 90.0 + 0.5).floor() as i32 & 3;
    ["S", "W", "N", "E"][i as usize]
}

fn bearing(dx: f64, dz: f64) -> &'static str {
    let deg = (-dx).atan2(dz).to_degrees();
    let a = ((deg % 360.0) + 360.0) % 360.0;
    const DIRS: [&str; 8] = ["S", "SW", "W", "NW", "N", "NE", "E", "SE"];
    let i = ((a + 22.5) / 45.0).floor() as usize % 8;
    DIRS[i]
}

fn stored(key: &str) -> Option<String> {
    storage::get(key)
        .and_then(|v| String::from_utf8(v).ok())
        .map(|v| v.trim().to_owned())
}

impl Mod for CairnMap {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY_MAP, "Large map", "key.keyboard.m")?;
        input::register_key(KEY_WAYPOINT, "New checkpoint", "key.keyboard.b")?;
        input::register_key(KEY_LIST, "Checkpoint list", "key.keyboard.n")?;
        input::register_key(KEY_HIDE, "Hide minimap", "key.keyboard.h")?;

        let find = |key: &str, values: &[String], default: usize| {
            stored(key)
                .and_then(|v| values.iter().position(|s| *s == v))
                .unwrap_or(default)
        };
        let corners: Vec<String> = CORNERS.iter().map(|c| c.1.to_owned()).collect();
        let sizes: Vec<String> = SIZES.iter().map(|s| s.0.to_string()).collect();
        let zooms: Vec<String> = ZOOMS.iter().map(f32::to_string).collect();
        let (waypoints, active) = Self::load_waypoints();
        let wp_serial = stored("serial")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let mut map = CairnMap {
            corner: find("corner", &corners, 1),
            size: find("size", &sizes, 1),
            round: stored("shape").as_deref() == Some("round"),
            zoom: find("zoom", &zooms, 1),
            rotate: stored("rotate").as_deref() == Some("true"),
            large: false,
            large_zoom: 2,
            hidden: false,
            show_coords: stored("coords").as_deref() != Some("false"),
            show_markers: stored("markers").as_deref() != Some("false"),
            waypoints,
            active,
            last_health: 20.0,
            list_sel: None,
            last_hud: Vec::new(),
            mode: UiMode::None,
            draft_name: 0,
            draft_typed: String::new(),
            draft_letter: 0,
            draft_color: 1,
            draft_icon: 0,
            draft_temp: false,
            draft_visible: true,
            draft_at_look: false,
            wp_serial,
        };
        map.show_map();
        map.settings_page();
        map.refresh_hud();
        map.draw_markers();
        Ok((
            map,
            Registration::new().on_frame().on_tick().command("cmap"),
        ))
    }

    fn frame(&mut self, _seconds: f32) {
        for event in input::poll() {
            match event {
                Event::KeyDown(k) if k == KEY_MAP => {
                    self.large = !self.large;
                    input::claim_wheel(self.large);
                    self.show_map();
                    self.refresh_hud();
                    self.draw_markers();
                }
                Event::KeyDown(k) if k == KEY_WAYPOINT => {
                    self.open_create();
                }
                Event::KeyDown(k) if k == KEY_LIST => {
                    self.open_list();
                }
                Event::KeyDown(k) if k == KEY_HIDE => {
                    self.hidden = !self.hidden;
                    if self.hidden {
                        self.large = false;
                        input::claim_wheel(false);
                    }
                    self.show_map();
                    self.draw_markers();
                    chat::show(if self.hidden {
                        "Minimap hidden"
                    } else {
                        "Minimap shown"
                    });
                }
                Event::Wheel(steps) if self.large => {
                    let n = LARGE_ZOOMS.len() as i32;
                    let mut i = self.large_zoom as i32;
                    if steps > 0.0 {
                        i = (i + 1).rem_euclid(n);
                    } else if steps < 0.0 {
                        i = (i - 1).rem_euclid(n);
                    }
                    self.large_zoom = i as usize;
                    self.show_map();
                    self.refresh_hud();
                    self.draw_markers();
                }
                _ => {}
            }
        }
        self.draw_markers();
    }

    fn tick(&mut self, _tick: u64) {
        if let Some(p) = game::get_player() {
            if self.last_health > 0.0 && p.health <= 0.0 {
                self.mark_death();
            }
            self.last_health = p.health;
        }
        // Drop temporary death-like points when far? Keep until cleared — Xaero keeps death.
        self.refresh_hud();
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "cmap" {
            return Ok(None);
        }
        let mut parts = args.split_whitespace();
        match parts.next().unwrap_or("help") {
            "add" => {
                let label = parts.collect::<Vec<_>>().join(" ");
                self.open_create();
                if !label.is_empty() {
                    self.draft_typed = label.chars().take(32).collect();
                }
                self.create_from_draft(None);
                Ok(None)
            }
            "rename" => {
                let label = parts.collect::<Vec<_>>().join(" ");
                if label.is_empty() {
                    return Ok(Some("Usage: /cmap rename <name> — or type it on the Edit screen".into()));
                }
                self.draft_typed = label.chars().take(32).collect();
                if let UiMode::Edit(i) = self.mode {
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                    return Ok(None);
                }
                if let Some(i) = self.active {
                    if let Some(w) = self.waypoints.get_mut(i) {
                        w.name = self.draft_typed.clone();
                    }
                    let shown = self
                        .waypoints
                        .get(i)
                        .map(|w| w.name.clone())
                        .unwrap_or_default();
                    self.save_waypoints();
                    return Ok(Some(format!("Active renamed to \"{shown}\"")));
                }
                Ok(Some("Open Edit (N) or set an active checkpoint first.".into()))
            }
            "list" => {
                if self.waypoints.is_empty() {
                    return Ok(Some("No checkpoints.".into()));
                }
                Ok(Some(
                    self.waypoints
                        .iter()
                        .enumerate()
                        .map(|(i, w)| {
                            format!(
                                "{i}. {} ({:.0},{:.0},{:.0})",
                                w.name,
                                w.x.floor(),
                                w.y.floor(),
                                w.z.floor()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" | "),
                ))
            }
            "clear" => {
                self.waypoints.clear();
                self.active = None;
                self.save_waypoints();
                Ok(Some("Checkpoints cleared.".into()))
            }
            "death" => {
                self.mark_death();
                Ok(None)
            }
            _ => Ok(Some(
                "/cmap add [name] | rename <name> | list | clear | death — B new, N list, M map, H hide"
                    .into(),
            )),
        }
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        match id {
            CORNER_BUTTON => {
                self.corner = (self.corner + 1) % CORNERS.len();
                self.save_settings();
                self.show_map();
                self.settings_page();
            }
            SIZE_BUTTON => {
                self.size = (self.size + 1) % SIZES.len();
                self.save_settings();
                self.show_map();
                self.settings_page();
            }
            SHAPE_BUTTON => {
                self.round = !self.round;
                self.save_settings();
                self.show_map();
                self.settings_page();
            }
            ZOOM_BUTTON => {
                self.zoom = (self.zoom + 1) % ZOOMS.len();
                self.save_settings();
                self.show_map();
                self.settings_page();
            }
            ROTATE_BUTTON => {
                self.rotate = !self.rotate;
                self.save_settings();
                self.show_map();
                self.settings_page();
            }
            COORDS_BUTTON => {
                self.show_coords = !self.show_coords;
                self.save_settings();
                self.refresh_hud();
                self.settings_page();
            }
            MARKERS_BUTTON => {
                self.show_markers = !self.show_markers;
                self.save_settings();
                self.draw_markers();
                self.settings_page();
            }
            WP_LIST => {
                if !self.waypoints.is_empty() {
                    self.list_sel = Some(row);
                    self.open_list();
                }
            }
            WP_NEW => self.open_create(),
            WP_EDIT => {
                if let Some(r) = self.list_sel {
                    let i = r as usize;
                    if i < self.waypoints.len() {
                        self.open_edit(i);
                    }
                }
            }
            WP_ACTIVE => {
                if let Some(r) = self.list_sel {
                    let i = r as usize;
                    if i < self.waypoints.len() {
                        self.active = Some(i);
                        self.save_waypoints();
                        chat::show(&format!("Active: {}", self.waypoints[i].name));
                        self.refresh_hud();
                    }
                }
                self.open_list();
            }
            WP_DELETE => {
                if let Some(r) = self.list_sel {
                    let i = r as usize;
                    if i < self.waypoints.len() {
                        let name = self.waypoints.remove(i).name;
                        self.active = self.active.and_then(|a| {
                            if a == i {
                                None
                            } else if a > i {
                                Some(a - 1)
                            } else {
                                Some(a)
                            }
                        });
                        self.list_sel = None;
                        self.save_waypoints();
                        chat::show(&format!("Deleted \"{name}\""));
                        self.refresh_hud();
                    }
                }
                self.open_list();
            }
            WP_CLEAR_DEATH => {
                self.waypoints.retain(|w| w.name != "Death");
                if self
                    .active
                    .map(|i| i >= self.waypoints.len())
                    .unwrap_or(false)
                {
                    self.active = None;
                }
                self.save_waypoints();
                chat::show("Death point cleared.");
                self.refresh_hud();
                self.open_list();
            }
            CR_PRESET => {
                self.draft_name = (self.draft_name + 1) % NAMES.len();
                self.draft_typed = self.draft_label();
                self.paint_create_name();
            }
            CR_LETTER => {
                self.draft_letter = (self.draft_letter + 1) % LETTERS.len();
                self.paint_create_name();
            }
            CR_ADD => {
                self.append_letter();
                self.paint_create_name();
            }
            CR_BACK => {
                self.backspace_letter();
                self.paint_create_name();
            }
            CR_CLEAR => {
                self.draft_typed.clear();
                self.paint_create_name();
            }
            CR_NEXT => {
                self.mode = UiMode::CreateStyle;
                self.paint_create_style();
            }
            CR_BACK_PAGE => {
                self.mode = UiMode::CreateName;
                self.paint_create_name();
            }
            CR_COLOR => {
                self.draft_color = (self.draft_color + 1) % COLORS.len();
                self.paint_create_style();
            }
            CR_ICON => {
                self.draft_icon = (self.draft_icon + 1) % ICONS.len();
                self.paint_create_style();
            }
            CR_TEMP => {
                // Cycle visible/temporary flag pairs.
                match (self.draft_visible, self.draft_temp) {
                    (true, false) => {
                        self.draft_temp = true;
                    }
                    (true, true) => {
                        self.draft_visible = false;
                        self.draft_temp = false;
                    }
                    (false, false) => {
                        self.draft_temp = true;
                    }
                    (false, true) => {
                        self.draft_visible = true;
                        self.draft_temp = false;
                    }
                }
                self.paint_create_style();
            }
            CR_PLACE => {
                self.draft_at_look = !self.draft_at_look;
                self.paint_create_style();
            }
            CR_CREATE => {
                if self.draft_at_look {
                    self.create_at_look();
                } else {
                    self.create_from_draft(None);
                }
            }
            CR_VISIBLE => {}
            ED_PRESET => {
                if let UiMode::Edit(i) = self.mode {
                    self.draft_name = (self.draft_name + 1) % NAMES.len();
                    if NAMES[self.draft_name] == "WP" {
                        self.wp_serial = self.wp_serial.saturating_add(1);
                        self.save_settings();
                    }
                    self.draft_typed = self.draft_label();
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                }
            }
            ED_LETTER => {
                if let UiMode::Edit(i) = self.mode {
                    self.draft_letter = (self.draft_letter + 1) % LETTERS.len();
                    self.paint_edit(i);
                }
            }
            ED_ADD => {
                if let UiMode::Edit(i) = self.mode {
                    self.append_letter();
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                }
            }
            ED_BACK => {
                if let UiMode::Edit(i) = self.mode {
                    self.backspace_letter();
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                }
            }
            ED_CLEAR => {
                if let UiMode::Edit(i) = self.mode {
                    self.draft_typed.clear();
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                }
            }
            ED_COLOR => {
                if let UiMode::Edit(i) = self.mode {
                    self.draft_color = (self.draft_color + 1) % COLORS.len();
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                }
            }
            ED_ICON => {
                if let UiMode::Edit(i) = self.mode {
                    self.draft_icon = (self.draft_icon + 1) % ICONS.len();
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                }
            }
            ED_TEMP => {
                if let UiMode::Edit(i) = self.mode {
                    self.draft_temp = !self.draft_temp;
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                }
            }
            ED_VISIBLE => {
                if let UiMode::Edit(i) = self.mode {
                    self.draft_visible = !self.draft_visible;
                    self.sync_edit_name(i);
                    self.paint_edit(i);
                }
            }
            ED_HERE => {
                if let UiMode::Edit(i) = self.mode {
                    if let Some(p) = game::get_player() {
                        if let Some(w) = self.waypoints.get_mut(i) {
                            w.x = p.x;
                            w.y = p.y;
                            w.z = p.z;
                            w.dim = p.dimension.clone();
                            self.save_waypoints();
                            chat::show("Checkpoint moved to you.");
                        }
                    }
                    self.paint_edit(i);
                }
            }
            ED_DONE => {
                if let UiMode::Edit(i) = self.mode {
                    self.sync_edit_name(i);
                }
                ui::close_screen();
                self.mode = UiMode::None;
                self.open_list();
            }
            _ => {}
        }
    }

    fn ui_closed(&mut self) {
        self.mode = UiMode::None;
    }

    fn ui_settings_opened(&mut self) {
        self.settings_page();
    }
}

horizon_mod_sdk::export_mod!(CairnMap);
