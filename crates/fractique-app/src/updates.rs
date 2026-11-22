use std::cmp::Ordering;
use std::sync::atomic::{AtomicU64, Ordering as Atomic};
use std::sync::mpsc;

use eframe::egui;

use crate::i18n::Texts;
use crate::update::{self, Block, Kind, Release};
use crate::widgets::{self, SEMIBOLD, font};

type Event = Box<dyn FnOnce(&mut Updates) + Send>;

enum Phase {
    Idle,
    Downloading {
        version: String,
        got: u64,
        total: Option<u64>,
    },
    Ready(String),
    Failed {
        version: String,
    },
}

enum List {
    NotAsked,
    Loading,
    Loaded(Vec<Release>),
    Failed,
}

enum Checked {
    Newest,
    Available(String),
    Failed,
}

pub struct Updates {
    pub open: bool,
    kind: Kind,
    asked: bool,
    newer: Option<String>,
    list: List,
    selected: Option<String>,
    checking: bool,
    checked: Option<Checked>,
    phase: Phase,
    events: (mpsc::Sender<Event>, mpsc::Receiver<Event>),
}

impl Updates {
    pub fn new() -> Self {
        update::tidy();
        let mut updates = Self {
            open: false,
            kind: update::kind(),
            asked: false,
            newer: None,
            list: List::NotAsked,
            selected: None,
            checking: false,
            checked: None,
            phase: Phase::Idle,
            events: mpsc::channel(),
        };
        if std::env::var("FRACTIQUE_SHOW").is_ok_and(|what| what == "updates") {
            updates.preview();
        }
        updates
    }

    fn preview(&mut self) {
        let release = |version: &str, date: &str, notes: &str| Release {
            version: version.into(),
            date: date.into(),
            notes: notes.into(),
        };
        self.open = true;
        self.asked = true;
        self.kind = Kind::Auto;
        self.newer = Some("0.3.0".into());
        self.phase = match std::env::var("FRACTIQUE_UPDATE_PHASE").as_deref() {
            Ok("downloading") => Phase::Downloading {
                version: "0.3.0".into(),
                got: 9_000_000,
                total: Some(20_000_000),
            },
            Ok("ready") => Phase::Ready("0.3.0".into()),
            Ok("failed") => Phase::Failed {
                version: "0.3.0".into(),
            },
            _ => Phase::Idle,
        };
        self.list = List::Loaded(vec![
            release(
                "0.3.0",
                "2026-12-20",
                "## Downloads\n\n| System | The app |\n| --- | --- |\n| Windows | [.exe](https://example.com/Fractique_0.3.0_x64.exe) |\n\nThe other files are what the updates read.\n\n## What's new\n\n* **Layers:** add, reorder and blend flames in the app.\n* **Export:** up to 8K, with a progress bar.\n* **Fixes:**\n  * The toolbar no longer runs off a narrow screen.",
            ),
            release("0.2.0", "2026-12-02", "* **App:** glass panels, 15 languages."),
            release(
                update::current(),
                "2026-11-18",
                "First version: the renderer and the command line tool.",
            ),
        ]);
    }

    fn post(&self, ctx: &egui::Context) -> impl Fn(Event) + Send + 'static {
        let (sender, ctx) = (self.events.0.clone(), ctx.clone());
        move |event| {
            if sender.send(event).is_ok() {
                ctx.request_repaint();
            }
        }
    }

    /// The newer version, once the check at start has found one.
    pub fn newer(&self) -> Option<&str> {
        self.newer.as_deref()
    }

    pub fn settle(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events.1.try_recv() {
            event(self);
        }
        if self.asked {
            return;
        }
        self.asked = true;
        if self.kind == Kind::Never {
            return;
        }
        let post = self.post(ctx);
        std::thread::spawn(move || {
            if let Ok(Some(found)) = update::check() {
                post(Box::new(move |updates| updates.newer = Some(found.version)));
            }
        });
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.open = true;
        if matches!(self.list, List::NotAsked | List::Failed) {
            self.list = List::Loading;
            let post = self.post(ctx);
            std::thread::spawn(move || {
                let loaded = update::releases();
                post(Box::new(move |updates| {
                    updates.list = match loaded {
                        Ok(releases) => List::Loaded(releases),
                        Err(error) => {
                            log::warn!("the versions were not loaded: {error}");
                            List::Failed
                        }
                    }
                }));
            });
        }
    }

    fn check_now(&mut self, ctx: &egui::Context) {
        if self.checking {
            return;
        }
        self.checking = true;
        self.checked = None;
        let post = self.post(ctx);
        std::thread::spawn(move || {
            let result = update::check();
            post(Box::new(move |updates| {
                updates.checking = false;
                updates.checked = Some(match result {
                    Ok(Some(found)) => {
                        updates.selected = Some(found.version.clone());
                        updates.newer = Some(found.version.clone());
                        Checked::Available(found.version)
                    }
                    Ok(None) => Checked::Newest,
                    Err(error) => {
                        log::warn!("the updates were not checked: {error}");
                        Checked::Failed
                    }
                });
            }));
        });
    }

    fn start_install(&mut self, ctx: &egui::Context, version: String) {
        if matches!(self.phase, Phase::Downloading { .. }) {
            return;
        }
        update::discard_pending();
        self.phase = Phase::Downloading {
            version: version.clone(),
            got: 0,
            total: None,
        };
        let post = self.post(ctx);
        std::thread::spawn(move || {
            let shown = AtomicU64::new(0);
            let progress = |got: u64, total: Option<u64>| {
                // One event per 200 kB: every chunk would flood the window with repaints.
                if got - shown.load(Atomic::Relaxed) >= 200_000 {
                    shown.store(got, Atomic::Relaxed);
                    post(Box::new(move |updates| {
                        if let Phase::Downloading {
                            got: seen, total: size, ..
                        } = &mut updates.phase
                        {
                            (*seen, *size) = (got, total);
                        }
                    }));
                }
            };
            let result = update::fetch(&version).and_then(|found| update::download(&found, progress));
            post(Box::new(move |updates| {
                updates.phase = match result {
                    Ok(()) => Phase::Ready(version),
                    Err(error) => {
                        log::warn!("the update was not downloaded: {error}");
                        Phase::Failed { version }
                    }
                }
            }));
        });
    }

    pub fn on_exit(&self) {
        update::install_pending(false);
    }

    fn releases(&self) -> &[Release] {
        match &self.list {
            List::Loaded(releases) => releases,
            _ => &[],
        }
    }

    fn chosen(&self) -> Option<String> {
        self.selected
            .clone()
            .or_else(|| self.newer.clone())
            .or_else(|| self.releases().first().map(|release| release.version.clone()))
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, texts: &Texts, width: f32) {
        let ctx = ui.ctx().clone();
        let dim = egui::Color32::from_white_alpha(170);
        ui.set_width(width);
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 2.0);
        widgets::title(ui, &texts.get("updates_title"), 17.0);
        ui.add_space(4.0);
        let own = texts.get("you_have").replace("{v}", update::current());
        let about = match self.kind {
            Kind::Auto => own,
            Kind::Manual => format!("{own} {}", texts.get("manual_copy")),
            Kind::Never => format!("{own} {}", texts.get("dev_build")),
        };
        ui.label(egui::RichText::new(about).size(13.0).color(dim));
        ui.add_space(10.0);
        let chosen = self.chosen();
        let mut install = None;
        let mut page = None;
        let mut restart = false;
        ui.horizontal(|ui| {
            let label = texts.get(if self.checking { "checking" } else { "check" });
            if widgets::pill(ui, &label, false, 0.0) {
                self.check_now(&ctx);
            }
            if let Some(result) = &self.checked {
                let text = match result {
                    Checked::Newest => texts.get("newest"),
                    Checked::Available(version) => texts.get("available").replace("{v}", version),
                    Checked::Failed => texts.get("check_failed"),
                };
                ui.label(egui::RichText::new(text).size(13.0).color(egui::Color32::WHITE));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match (&self.phase, chosen.as_deref()) {
                    (Phase::Downloading { .. }, _) => {}
                    (Phase::Ready(_), _) => restart = widgets::pill(ui, &texts.get("restart_now"), true, 0.0),
                    (_, Some(version)) if update::compare(version) == Ordering::Equal => {}
                    (_, Some(version)) if self.kind == Kind::Auto => {
                        let key = if update::compare(version) == Ordering::Greater {
                            "update_to"
                        } else {
                            "install_v"
                        };
                        if widgets::pill(ui, &texts.get(key).replace("{v}", version), true, 0.0) {
                            install = Some(version.to_owned());
                        }
                    }
                    (_, Some(version)) => {
                        if widgets::pill(ui, &texts.get("download_page"), true, 0.0) {
                            page = Some(update::release_page(version));
                        }
                    }
                    (_, None) => {}
                }
            });
        });
        // The gap above the list holds the progress of a download and what the window has to say, at a height that
        // stays, so the window keeps its size.
        ui.add_space(8.0);
        let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
        if let Phase::Downloading { got, total, .. } = &self.phase {
            let share = total.map_or(0.0, |total| *got as f32 / total.max(1) as f32);
            let done = egui::Rect::from_min_size(bar.min, egui::vec2(bar.width() * share, bar.height()));
            ui.painter()
                .rect_filled(bar, egui::CornerRadius::same(3), egui::Color32::from_white_alpha(45));
            ui.painter()
                .rect_filled(done, egui::CornerRadius::same(3), egui::Color32::from_white_alpha(225));
        }
        ui.add_space(4.0);
        let said = match (&self.phase, chosen.as_deref()) {
            (Phase::Downloading { version, got, .. }, _) => format!(
                "{} · {:.1} MB",
                texts.get("downloading").replace("{v}", version),
                *got as f64 / 1e6
            ),
            (Phase::Ready(version), _) => texts.get("ready").replace("{v}", version),
            (Phase::Failed { version }, _) => format!("Fractique {version} {}", texts.get("download_failed")),
            (_, Some(version)) if update::compare(version) == Ordering::Equal => texts.get("this_version"),
            _ => String::new(),
        };
        let line = egui::vec2(ui.available_width(), 18.0);
        ui.allocate_ui_with_layout(line, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(said).size(13.0).color(dim)).truncate());
        });
        ui.add_space(8.0);

        let height = 250.0;
        let mut pick = None;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(176.0);
                egui::ScrollArea::vertical()
                    .id_salt("versions")
                    .max_height(height)
                    .min_scrolled_height(height)
                    .show(ui, |ui| match &self.list {
                        List::NotAsked | List::Loading => {
                            ui.label(egui::RichText::new(texts.get("loading_versions")).size(13.0).color(dim));
                        }
                        List::Failed => {
                            let text = texts.get("versions_failed");
                            ui.label(egui::RichText::new(text).size(13.0).color(dim));
                        }
                        List::Loaded(releases) => {
                            for release in releases {
                                let selected = chosen.as_deref() == Some(release.version.as_str());
                                if widgets::noted_row(ui, &release.version, &release.date, selected) {
                                    pick = Some(release.version.clone());
                                }
                            }
                        }
                    });
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .id_salt("notes")
                    .max_height(height)
                    .min_scrolled_height(height)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        let notes = chosen
                            .as_deref()
                            .and_then(|version| self.releases().iter().find(|r| r.version == version));
                        let blocks = notes.map(|release| update::blocks(&release.notes)).unwrap_or_default();
                        if blocks.is_empty() {
                            ui.label(egui::RichText::new(texts.get("no_notes")).size(13.0).color(dim));
                        }
                        for block in blocks {
                            match block {
                                Block::Heading(text) => {
                                    ui.label(
                                        egui::RichText::new(text)
                                            .font(font(14.0, SEMIBOLD))
                                            .color(egui::Color32::WHITE),
                                    );
                                }
                                Block::Item(text) => {
                                    ui.horizontal_top(|ui| {
                                        ui.label(egui::RichText::new("•").size(13.5).color(dim));
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(text)
                                                    .size(13.5)
                                                    .color(egui::Color32::from_white_alpha(225)),
                                            )
                                            .wrap(),
                                        );
                                    });
                                }
                                Block::Text(text) => {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(text)
                                                .size(13.5)
                                                .color(egui::Color32::from_white_alpha(225)),
                                        )
                                        .wrap(),
                                    );
                                }
                            }
                        }
                    });
            });
        });
        if let Some(version) = pick {
            self.selected = Some(version);
        }

        if let Some(version) = install {
            self.start_install(&ctx, version);
        }
        if let Some(url) = page {
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
        if restart {
            update::install_pending(true);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
