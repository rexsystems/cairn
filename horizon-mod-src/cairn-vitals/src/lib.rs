//! Health + hunger readout on the HUD. AppleSkin-inspired; Horizon only
//! exposes food points (0–20), not saturation or food-item tooltips.
use horizon_mod_sdk::{
    Mod, Registration,
    game,
    hud::{self, Corner, Line},
};

struct CairnVitals {
    last: Vec<String>,
}

fn hearts(n: f32) -> String {
    let full = (n / 2.0).floor() as u32;
    let half = (n as u32) % 2;
    format!("{full}♥{}", if half == 1 { "½" } else { "" })
}

fn shanks(food: u32) -> String {
    let full = food / 2;
    let half = food % 2;
    format!("{full}🍗{}", if half == 1 { "½" } else { "" })
}

fn lines() -> Option<Vec<String>> {
    let p = game::get_player()?;
    Some(vec![
        format!("Health {:.1}  {}", p.health, hearts(p.health)),
        format!("Food {}  {}", p.food, shanks(p.food)),
    ])
}

impl Mod for CairnVitals {
    fn init() -> Result<(Self, Registration), String> {
        Ok((CairnVitals { last: Vec::new() }, Registration::new().on_tick()))
    }

    fn tick(&mut self, _tick: u64) {
        let Some(now) = lines() else {
            if !self.last.is_empty() {
                hud::clear();
                self.last.clear();
            }
            return;
        };
        if now == self.last {
            return;
        }
        let panel: Vec<Line> = now
            .iter()
            .map(|text| Line {
                text: text.clone(),
                color: 0xFFCC88,
            })
            .collect();
        hud::set_panel(Corner::BottomLeft, &panel);
        self.last = now;
    }
}

horizon_mod_sdk::export_mod!(CairnVitals);
