//! Pause menu. Escape toggles this overlay — never releases the mouse
//! without a menu. Settings pages cover keybinds, display, and graphics.

use crate::hud::{card, draw_text, sprite, HudRect, ItemMeshes};
use crate::scene::DrawRequest;
use crate::settings::{keycode_label, Action, Quality, Settings, RESOLUTIONS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PausePage {
    Root,
    Settings,
    ConfirmExit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    Keybinds,
    Display,
    Graphics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseHit {
    Resume,
    OpenSettings,
    Exit,
    ConfirmYes,
    ConfirmNo,
    Back,
    Tab(SettingsTab),
    Bind(Action),
    Resolution(usize),
    Fullscreen(bool),
    Vsync(bool),
    Quality(Quality),
    Brightness(i8),
    Sens(i8),
}

#[derive(Clone, Debug)]
pub struct PauseMenu {
    pub open: bool,
    pub page: PausePage,
    pub tab: SettingsTab,
    pub waiting: Option<Action>,
    pub hover: Option<PauseHit>,
}

impl Default for PauseMenu {
    fn default() -> Self {
        Self {
            open: false,
            page: PausePage::Root,
            tab: SettingsTab::Keybinds,
            waiting: None,
            hover: None,
        }
    }
}

impl PauseMenu {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.page = PausePage::Root;
            self.waiting = None;
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.waiting = None;
        self.page = PausePage::Root;
    }

    pub fn layout(&self, settings: &Settings) -> Vec<(PauseHit, HudRect)> {
        match self.page {
            PausePage::Root => vec![
                (PauseHit::Resume, btn(0.0, 0.18, 0.62, 0.10)),
                (PauseHit::OpenSettings, btn(0.0, 0.00, 0.62, 0.10)),
                (PauseHit::Exit, btn(0.0, -0.18, 0.62, 0.10)),
            ],
            PausePage::ConfirmExit => vec![
                (PauseHit::ConfirmYes, btn(-0.22, -0.08, 0.28, 0.09)),
                (PauseHit::ConfirmNo, btn(0.22, -0.08, 0.28, 0.09)),
            ],
            PausePage::Settings => {
                let mut out = vec![
                    (PauseHit::Tab(SettingsTab::Keybinds), btn(-0.42, 0.50, 0.32, 0.075)),
                    (PauseHit::Tab(SettingsTab::Display), btn(0.00, 0.50, 0.32, 0.075)),
                    (PauseHit::Tab(SettingsTab::Graphics), btn(0.42, 0.50, 0.32, 0.075)),
                    (PauseHit::Back, btn(0.00, -0.62, 0.40, 0.08)),
                ];
                match self.tab {
                    SettingsTab::Keybinds => {
                        for (i, a) in Action::ALL.iter().enumerate() {
                            let (col, row) = (i / 8, i % 8);
                            let y = 0.36 - row as f32 * 0.072;
                            let x = if col == 0 { -0.18 } else { 0.58 };
                            out.push((
                                PauseHit::Bind(*a),
                                HudRect {
                                    x,
                                    y,
                                    hw: 0.16,
                                    hh: 0.028,
                                },
                            ));
                        }
                    }
                    SettingsTab::Display => {
                        for (i, _) in RESOLUTIONS.iter().enumerate() {
                            let y = 0.32 - i as f32 * 0.09;
                            out.push((PauseHit::Resolution(i), btn(0.0, y, 0.55, 0.075)));
                        }
                        let fs = settings.fullscreen;
                        out.push((PauseHit::Fullscreen(!fs), btn(0.0, -0.22, 0.55, 0.075)));
                    }
                    SettingsTab::Graphics => {
                        out.push((PauseHit::Vsync(!settings.vsync), btn(0.0, 0.30, 0.55, 0.075)));
                        for (i, q) in [Quality::Low, Quality::Medium, Quality::High, Quality::Ultra]
                            .iter()
                            .enumerate()
                        {
                            let x = -0.36 + i as f32 * 0.24;
                            out.push((PauseHit::Quality(*q), btn(x, 0.10, 0.20, 0.075)));
                        }
                        out.push((PauseHit::Brightness(-5), btn(-0.38, -0.12, 0.10, 0.07)));
                        out.push((PauseHit::Brightness(5), btn(0.38, -0.12, 0.10, 0.07)));
                        out.push((PauseHit::Sens(-5), btn(-0.38, -0.30, 0.10, 0.07)));
                        out.push((PauseHit::Sens(5), btn(0.38, -0.30, 0.10, 0.07)));
                    }
                }
                out
            }
        }
    }

    pub fn hit(&self, settings: &Settings, mx: f32, my: f32) -> Option<PauseHit> {
        self.layout(settings)
            .into_iter()
            .rev()
            .find(|(_, r)| r.contains(mx, my))
            .map(|(h, _)| h)
    }

    pub fn click(&mut self, settings: &mut Settings, hit: PauseHit) -> PauseResult {
        match hit {
            PauseHit::Resume => {
                self.close();
                PauseResult::Resume
            }
            PauseHit::OpenSettings => {
                self.page = PausePage::Settings;
                PauseResult::None
            }
            PauseHit::Exit => {
                self.page = PausePage::ConfirmExit;
                PauseResult::None
            }
            PauseHit::ConfirmYes => PauseResult::Exit,
            PauseHit::ConfirmNo => {
                self.page = PausePage::Root;
                PauseResult::None
            }
            PauseHit::Back => {
                self.page = PausePage::Root;
                self.waiting = None;
                PauseResult::None
            }
            PauseHit::Tab(t) => {
                self.tab = t;
                self.waiting = None;
                PauseResult::None
            }
            PauseHit::Bind(a) => {
                self.waiting = Some(a);
                PauseResult::None
            }
            PauseHit::Resolution(i) => {
                if let Some(&(w, h, _)) = RESOLUTIONS.get(i) {
                    settings.width = w;
                    settings.height = h;
                    settings.save();
                    PauseResult::ApplyDisplay
                } else {
                    PauseResult::None
                }
            }
            PauseHit::Fullscreen(v) => {
                settings.fullscreen = v;
                settings.save();
                PauseResult::ApplyDisplay
            }
            PauseHit::Vsync(v) => {
                settings.vsync = v;
                settings.save();
                PauseResult::ApplyGraphics
            }
            PauseHit::Quality(q) => {
                settings.quality = q;
                settings.save();
                PauseResult::ApplyGraphics
            }
            PauseHit::Brightness(d) => {
                settings.brightness = (settings.brightness as i16 + d as i16).clamp(50, 150) as u8;
                settings.save();
                PauseResult::ApplyGraphics
            }
            PauseHit::Sens(d) => {
                settings.mouse_sens = (settings.mouse_sens as i16 + d as i16).clamp(50, 200) as u8;
                settings.save();
                PauseResult::None
            }
        }
    }

    pub fn capture_rebind(&mut self, settings: &mut Settings, code: winit::keyboard::KeyCode) {
        if let Some(a) = self.waiting.take() {
            if code == winit::keyboard::KeyCode::Escape {
                return;
            }
            settings.set_key(a, code);
            settings.save();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseResult {
    None,
    Resume,
    Exit,
    ApplyDisplay,
    ApplyGraphics,
}

fn btn(x: f32, y: f32, w: f32, h: f32) -> HudRect {
    HudRect {
        x,
        y,
        hw: w * 0.5,
        hh: h * 0.5,
    }
}

pub fn pause_draws(
    meshes: &ItemMeshes,
    menu: &PauseMenu,
    settings: &Settings,
    hover: Option<PauseHit>,
) -> Vec<DrawRequest> {
    let mut out = Vec::new();
    // Dim the world.
    out.push(DrawRequest {
        handle: meshes.dim,
        model: crate::hud::sprite(0.0, 0.0, 2.2, 2.2),
    });

    let (pw, ph) = match menu.page {
        PausePage::Settings => (1.55, 1.46),
        _ => (0.92, 0.92),
    };
    out.push(DrawRequest {
        handle: meshes.slot_border,
        model: card(0.0, 0.0, pw + 0.04, ph + 0.04),
    });
    out.push(DrawRequest {
        handle: meshes.panel_dark,
        model: card(0.0, 0.0, pw, ph),
    });

    match menu.page {
        PausePage::Root => {
            draw_text(&mut out, meshes, "PAUSED", -0.22, 0.36, 0.055);
            draw_text(&mut out, meshes, "THE WORLD HOLDS", -0.30, 0.28, 0.022);
            for (hit, rect) in menu.layout(settings) {
                let on = hover == Some(hit);
                let label = match hit {
                    PauseHit::Resume => "RESUME",
                    PauseHit::OpenSettings => "SETTINGS",
                    PauseHit::Exit => "EXIT GAME",
                    _ => "",
                };
                push_btn(&mut out, meshes, rect, label, on);
            }
        }
        PausePage::ConfirmExit => {
            draw_text(&mut out, meshes, "LEAVE THE WORLD?", -0.38, 0.16, 0.032);
            for (hit, rect) in menu.layout(settings) {
                let on = hover == Some(hit);
                let label = match hit {
                    PauseHit::ConfirmYes => "YES",
                    PauseHit::ConfirmNo => "NO",
                    _ => "",
                };
                push_btn(&mut out, meshes, rect, label, on);
            }
        }
        PausePage::Settings => {
            draw_text(&mut out, meshes, "SETTINGS", -0.24, 0.62, 0.042);
            for (hit, rect) in menu.layout(settings) {
                if let PauseHit::Tab(t) = hit {
                    let on = menu.tab == t || hover == Some(hit);
                    let label = match t {
                        SettingsTab::Keybinds => "KEYS",
                        SettingsTab::Display => "DISPLAY",
                        SettingsTab::Graphics => "GRAPHICS",
                    };
                    push_btn(&mut out, meshes, rect, label, on);
                }
            }
            match menu.tab {
                SettingsTab::Keybinds => {
                    for (i, a) in Action::ALL.iter().enumerate() {
                        let (col, row) = (i / 8, i % 8);
                        let y = 0.36 - row as f32 * 0.072;
                        let lx = if col == 0 { -0.70 } else { 0.08 };
                        let bx = if col == 0 { -0.18 } else { 0.58 };
                        draw_text(&mut out, meshes, a.label(), lx, y, 0.016);
                        let waiting = menu.waiting == Some(*a);
                        let key = if waiting {
                            "..."
                        } else {
                            keycode_label(settings.key(*a))
                        };
                        let on = hover == Some(PauseHit::Bind(*a)) || waiting;
                        push_btn(
                            &mut out,
                            meshes,
                            HudRect {
                                x: bx,
                                y,
                                hw: 0.16,
                                hh: 0.028,
                            },
                            key,
                            on,
                        );
                    }
                    draw_text(
                        &mut out,
                        meshes,
                        "CLICK A BIND THEN PRESS A KEY   ESC CANCELS",
                        -0.68,
                        -0.50,
                        0.016,
                    );
                }
                SettingsTab::Display => {
                    for (i, &(w, h, label)) in RESOLUTIONS.iter().enumerate() {
                        let y = 0.32 - i as f32 * 0.09;
                        let on = settings.width == w && settings.height == h
                            || hover == Some(PauseHit::Resolution(i));
                        push_btn(&mut out, meshes, btn(0.0, y, 0.55, 0.075), label, on);
                    }
                    let fs = if settings.fullscreen {
                        "MODE  FULLSCREEN"
                    } else {
                        "MODE  WINDOWED"
                    };
                    push_btn(
                        &mut out,
                        meshes,
                        btn(0.0, -0.22, 0.55, 0.075),
                        fs,
                        hover == Some(PauseHit::Fullscreen(!settings.fullscreen)),
                    );
                }
                SettingsTab::Graphics => {
                    let vs = if settings.vsync {
                        "VSYNC  ON"
                    } else {
                        "VSYNC  OFF"
                    };
                    push_btn(
                        &mut out,
                        meshes,
                        btn(0.0, 0.30, 0.55, 0.075),
                        vs,
                        hover == Some(PauseHit::Vsync(!settings.vsync)),
                    );
                    draw_text(&mut out, meshes, "QUALITY", -0.16, 0.20, 0.020);
                    for q in [Quality::Low, Quality::Medium, Quality::High, Quality::Ultra] {
                        let i = q.as_u8() as usize;
                        let x = -0.36 + i as f32 * 0.24;
                        let on = settings.quality == q || hover == Some(PauseHit::Quality(q));
                        push_btn(&mut out, meshes, btn(x, 0.10, 0.20, 0.075), q.name(), on);
                    }
                    draw_text(&mut out, meshes, "BRIGHTNESS", -0.22, -0.02, 0.020);
                    push_btn(
                        &mut out,
                        meshes,
                        btn(-0.38, -0.12, 0.10, 0.07),
                        "-",
                        hover == Some(PauseHit::Brightness(-5)),
                    );
                    let b = format!("{}", settings.brightness);
                    draw_text(&mut out, meshes, &b, -0.06, -0.12, 0.028);
                    push_btn(
                        &mut out,
                        meshes,
                        btn(0.38, -0.12, 0.10, 0.07),
                        "+",
                        hover == Some(PauseHit::Brightness(5)),
                    );
                    draw_text(&mut out, meshes, "MOUSE SENS", -0.22, -0.22, 0.020);
                    push_btn(
                        &mut out,
                        meshes,
                        btn(-0.38, -0.30, 0.10, 0.07),
                        "-",
                        hover == Some(PauseHit::Sens(-5)),
                    );
                    let s = format!("{}", settings.mouse_sens);
                    draw_text(&mut out, meshes, &s, -0.06, -0.30, 0.028);
                    push_btn(
                        &mut out,
                        meshes,
                        btn(0.38, -0.30, 0.10, 0.07),
                        "+",
                        hover == Some(PauseHit::Sens(5)),
                    );
                }
            }
            let back_on = hover == Some(PauseHit::Back);
            push_btn(&mut out, meshes, btn(0.0, -0.62, 0.40, 0.08), "BACK", back_on);
        }
    }
    out
}

fn push_btn(
    out: &mut Vec<DrawRequest>,
    meshes: &ItemMeshes,
    rect: HudRect,
    label: &str,
    on: bool,
) {
    let frame = if on {
        meshes.slot_sel
    } else {
        meshes.slot_station
    };
    out.push(DrawRequest {
        handle: frame,
        model: sprite(rect.x, rect.y, rect.hw * 2.0, rect.hh * 2.0),
    });
    let size = 0.022;
    let w = label.len() as f32 * size * 0.72;
    draw_text(out, meshes, label, rect.x - w * 0.5, rect.y, size);
}
