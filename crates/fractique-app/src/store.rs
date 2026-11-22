#![cfg_attr(
    not(all(feature = "store", any(target_os = "ios", target_os = "macos"))),
    allow(dead_code)
)]

use std::sync::mpsc;

pub const PRODUCT: &str = "app.fractique.pro";
pub const FREE_A_DAY: u32 = 3;

// The pictures saved or shared without the purchase, counted by the day on the device itself.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Saves {
    day: String,
    made: u32,
}

impl Saves {
    pub fn parse(text: &str) -> Self {
        let Some((day, made)) = text.split_once(' ') else {
            return Self::default();
        };
        Self {
            day: day.to_owned(),
            made: made.parse().unwrap_or(0),
        }
    }

    pub fn text(&self) -> String {
        format!("{} {}", self.day, self.made)
    }

    pub fn left(&self, today: &str) -> u32 {
        if self.day == today {
            FREE_A_DAY.saturating_sub(self.made)
        } else {
            FREE_A_DAY
        }
    }

    pub fn count(&mut self, today: &str) {
        if self.day != today {
            self.day = today.to_owned();
            self.made = 0;
        }
        self.made += 1;
    }
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[derive(Clone, PartialEq, Debug)]
pub struct Price {
    pub shown: String,
    pub plain: String,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Problem {
    Told(String),
    NoProduct,
    NothingToRestore,
    NotAllowed,
    Deferred,
}

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Loading,
    Ready,
    Buying,
    Owned,
    Failed(Problem),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Event {
    Product(Price),
    Owned,
    Cancelled,
    Restored,
    Failed(Problem),
}

pub trait Backend {
    fn load(&mut self);
    fn buy(&mut self);
    fn restore(&mut self);
}

pub struct Store {
    state: State,
    price: Option<Price>,
    backend: Box<dyn Backend>,
    events: mpsc::Receiver<Event>,
}

impl Store {
    pub fn new(owned: bool, events: mpsc::Receiver<Event>, mut backend: Box<dyn Backend>) -> Self {
        let state = if owned {
            State::Owned
        } else {
            backend.load();
            State::Loading
        };
        Self {
            state,
            price: None,
            backend,
            events,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn price(&self) -> Option<&Price> {
        self.price.as_ref()
    }

    pub fn owned(&self) -> bool {
        self.state == State::Owned
    }

    fn rest(&mut self) {
        if self.price.is_some() {
            self.state = State::Ready;
        } else {
            self.state = State::Loading;
            self.backend.load();
        }
    }

    pub fn wake(&mut self) {
        if matches!(self.state, State::Failed(_)) {
            self.rest();
        }
    }

    pub fn buy(&mut self) {
        if !matches!(self.state, State::Ready | State::Failed(_)) {
            return;
        }
        if self.price.is_some() {
            self.state = State::Buying;
            self.backend.buy();
        } else {
            self.rest();
        }
    }

    pub fn restore(&mut self) {
        if matches!(self.state, State::Owned | State::Buying) {
            return;
        }
        self.state = State::Buying;
        self.backend.restore();
    }

    // True when the purchase arrived in this call.
    pub fn settle(&mut self) -> bool {
        let before = self.owned();
        while let Ok(event) = self.events.try_recv() {
            self.take(event);
        }
        self.owned() && !before
    }

    fn take(&mut self, event: Event) {
        if self.owned() {
            return;
        }
        match event {
            Event::Product(price) => {
                self.price = Some(price);
                if self.state == State::Loading {
                    self.state = State::Ready;
                }
            }
            Event::Owned => self.state = State::Owned,
            Event::Cancelled => {
                if self.state == State::Buying {
                    self.rest();
                }
            }
            Event::Restored => {
                if self.state == State::Buying {
                    self.state = State::Failed(Problem::NothingToRestore);
                }
            }
            Event::Failed(problem) => self.state = State::Failed(problem),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Fake {
    Locked,
    Loading,
    Buying,
    Failed,
}

struct FakeStore {
    kind: Fake,
    events: mpsc::Sender<Event>,
}

impl FakeStore {
    fn post(&self, event: Event) {
        let _ = self.events.send(event);
    }

    fn refuse(&self) {
        self.post(Event::Failed(Problem::Told(
            "Cannot connect to the App Store".to_owned(),
        )));
    }
}

impl Backend for FakeStore {
    fn load(&mut self) {
        match self.kind {
            Fake::Locked | Fake::Buying => self.post(Event::Product(Price {
                shown: "$4.99".to_owned(),
                plain: "4.99 USD".to_owned(),
            })),
            Fake::Failed => self.refuse(),
            Fake::Loading => {}
        }
    }

    fn buy(&mut self) {
        match self.kind {
            Fake::Locked => self.post(Event::Owned),
            Fake::Failed => self.refuse(),
            Fake::Loading | Fake::Buying => {}
        }
    }

    fn restore(&mut self) {
        match self.kind {
            Fake::Locked => self.post(Event::Restored),
            Fake::Failed => self.refuse(),
            Fake::Loading | Fake::Buying => {}
        }
    }
}

// FRACTIQUE_STORE=locked|owned|failed|loading|buying
pub fn fake(kind: &str) -> Option<Store> {
    let (owned, kind) = match kind {
        "locked" => (false, Fake::Locked),
        "owned" => (true, Fake::Locked),
        "failed" => (false, Fake::Failed),
        "loading" => (false, Fake::Loading),
        "buying" => (false, Fake::Buying),
        _ => return None,
    };
    let (sender, events) = mpsc::channel();
    let mut store = Store::new(owned, events, Box::new(FakeStore { kind, events: sender }));
    store.settle();
    if kind == Fake::Buying {
        store.buy();
    }
    Some(store)
}

#[cfg(feature = "store")]
pub fn open(ctx: &eframe::egui::Context) -> Option<Store> {
    let asked = std::env::var("FRACTIQUE_STORE").ok();
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        // A release build for the App Store must not be unlocked from a terminal.
        if cfg!(debug_assertions)
            && let Some(kind) = &asked
        {
            return fake(kind);
        }
        Some(crate::storekit::open(ctx))
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let _ = ctx;
        fake(&asked?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_saves_a_day_are_free() {
        let mut saves = Saves::default();
        assert_eq!(saves.left("2026-11-22"), FREE_A_DAY);
        for left in (0..FREE_A_DAY).rev() {
            saves.count("2026-11-22");
            assert_eq!(saves.left("2026-11-22"), left);
        }
        saves.count("2026-11-22");
        assert_eq!(saves.left("2026-11-22"), 0);
        assert_eq!(Saves::parse(&saves.text()), saves);
        assert_eq!(saves.left("2026-11-23"), FREE_A_DAY);
        saves.count("2026-11-23");
        assert_eq!(saves.left("2026-11-23"), FREE_A_DAY - 1);
        assert_eq!(Saves::parse("nonsense").left("2026-11-23"), FREE_A_DAY);
    }

    fn with(kind: Fake, owned: bool) -> (Store, mpsc::Sender<Event>) {
        let (sender, events) = mpsc::channel();
        let backend = FakeStore {
            kind,
            events: sender.clone(),
        };
        (Store::new(owned, events, Box::new(backend)), sender)
    }

    #[test]
    fn a_purchase_unlocks() {
        let (mut store, _) = with(Fake::Locked, false);
        assert_eq!(*store.state(), State::Loading);
        assert!(!store.owned());
        assert!(!store.settle());
        assert_eq!(*store.state(), State::Ready);
        assert_eq!(store.price().map(|price| price.shown.as_str()), Some("$4.99"));
        store.buy();
        assert_eq!(*store.state(), State::Buying);
        assert!(store.settle());
        assert!(store.owned());
        assert!(!store.settle());
    }

    #[test]
    fn a_remembered_purchase_needs_no_store() {
        let (store, _) = with(Fake::Failed, true);
        assert!(store.owned());
    }

    #[test]
    fn nothing_is_bought_before_the_product_is_known() {
        let (mut store, sender) = with(Fake::Loading, false);
        store.buy();
        assert_eq!(*store.state(), State::Loading);
        sender.send(Event::Failed(Problem::NoProduct)).unwrap();
        store.settle();
        assert_eq!(*store.state(), State::Failed(Problem::NoProduct));
        store.buy();
        assert_eq!(*store.state(), State::Loading);
    }

    #[test]
    fn a_cancelled_purchase_can_be_tried_again() {
        let (mut store, sender) = with(Fake::Buying, false);
        store.settle();
        store.buy();
        sender.send(Event::Cancelled).unwrap();
        assert!(!store.settle());
        assert_eq!(*store.state(), State::Ready);
        store.buy();
        sender
            .send(Event::Failed(Problem::Told("declined".to_owned())))
            .unwrap();
        store.settle();
        assert_eq!(*store.state(), State::Failed(Problem::Told("declined".to_owned())));
        store.wake();
        assert_eq!(*store.state(), State::Ready);
        store.buy();
        sender.send(Event::Failed(Problem::Deferred)).unwrap();
        sender.send(Event::Owned).unwrap();
        assert!(store.settle());
    }

    #[test]
    fn a_restore_finds_the_purchase_or_says_so() {
        let (mut store, _) = with(Fake::Locked, false);
        store.settle();
        store.restore();
        assert_eq!(*store.state(), State::Buying);
        store.settle();
        assert_eq!(*store.state(), State::Failed(Problem::NothingToRestore));

        let (mut store, sender) = with(Fake::Loading, false);
        store.restore();
        sender.send(Event::Owned).unwrap();
        sender.send(Event::Restored).unwrap();
        assert!(store.settle());
        assert!(store.owned());
    }

    #[test]
    fn a_store_that_does_not_answer_fails_with_its_words() {
        let (mut store, _) = with(Fake::Failed, false);
        store.settle();
        assert!(matches!(store.state(), State::Failed(Problem::Told(words)) if !words.is_empty()));
        assert!(!store.owned());
    }

    #[test]
    fn the_hook_names_the_fake() {
        assert_eq!(fake("locked").map(|store| store.state().clone()), Some(State::Ready));
        assert_eq!(fake("owned").map(|store| store.state().clone()), Some(State::Owned));
        assert_eq!(fake("loading").map(|store| store.state().clone()), Some(State::Loading));
        assert_eq!(fake("buying").map(|store| store.state().clone()), Some(State::Buying));
        assert!(matches!(
            fake("failed").map(|store| store.state().clone()),
            Some(State::Failed(_))
        ));
        assert!(fake("").is_none());
    }
}
