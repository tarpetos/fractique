#![cfg_attr(
    not(all(feature = "store", any(target_os = "ios", target_os = "macos"))),
    allow(dead_code)
)]

use std::sync::mpsc;

pub const PRODUCT: &str = "app.fractique.pro";
const FREE_BELOW: u64 = 2560 * 1440;

pub fn is_large([width, height]: [u32; 2]) -> bool {
    u64::from(width) * u64::from(height) >= FREE_BELOW
}

pub fn needs_pro(size: [u32; 2], listed: bool) -> bool {
    !listed || is_large(size)
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
    fn the_limit_is_a_number_of_pixels() {
        for free in [
            [1920, 1080],
            [1290, 2796],
            [1179, 2556],
            [2560, 1439],
            [1439, 2560],
            [1600, 2303],
            [16, 16],
            [16384, 224],
        ] {
            assert!(!is_large(free), "{free:?}");
        }
        for large in [
            [2560, 1440],
            [1440, 2560],
            [3840, 2160],
            [2048, 2732],
            [1600, 2560],
            [1920, 1920],
            [1600, 2304],
            [16384, 225],
            [16384, 16384],
        ] {
            assert!(is_large(large), "{large:?}");
        }
    }

    #[test]
    fn a_size_typed_in_needs_the_purchase() {
        assert!(needs_pro([2559, 1439], false));
        assert!(needs_pro([640, 480], false));
        assert!(!needs_pro([1920, 1080], true));
        assert!(!needs_pro([1290, 2796], true));
        assert!(needs_pro([2560, 1440], true));
        assert!(needs_pro([3840, 2160], true));
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
