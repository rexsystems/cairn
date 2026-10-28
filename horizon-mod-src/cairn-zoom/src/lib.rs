//! Hold C to zoom (4×). While held, the mouse wheel changes the zoom.
use horizon_mod_sdk::{
    Mod, Registration,
    input::{self, Event},
    render::{self, Zoom as ZoomView},
};

const KEY: &str = "zoom";
const START: f32 = 4.0;
const MIN: f32 = 1.25;
const MAX: f32 = 10.0;
const STEP: f32 = 1.25;

struct CairnZoom {
    held: Option<f32>,
}

impl CairnZoom {
    fn show(&self) {
        let factor = self.held.unwrap_or(1.0);
        render::set_zoom(ZoomView {
            factor,
            smooth: true,
            turn: 1.0 / factor,
        });
    }
}

impl Mod for CairnZoom {
    fn init() -> Result<(Self, Registration), String> {
        input::register_key(KEY, "Zoom", "key.keyboard.c")?;
        Ok((CairnZoom { held: None }, Registration::new().on_frame()))
    }

    fn frame(&mut self, _seconds: f32) {
        let mut changed = false;
        for event in input::poll() {
            match event {
                Event::KeyDown(k) if k == KEY => {
                    self.held = Some(START);
                    input::claim_wheel(true);
                    changed = true;
                }
                Event::KeyUp(k) if k == KEY => {
                    self.held = None;
                    input::claim_wheel(false);
                    changed = true;
                }
                Event::Wheel(steps) => {
                    if let Some(z) = &mut self.held {
                        *z = (*z * STEP.powf(steps)).clamp(MIN, MAX);
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        if changed {
            self.show();
        }
    }
}

horizon_mod_sdk::export_mod!(CairnZoom);
