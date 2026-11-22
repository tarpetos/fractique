use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{OnceLock, mpsc};

use eframe::egui;
use objc2::runtime::{AnyClass, AnyObject, AnyProtocol, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};

use crate::store::{Backend, Event, PRODUCT, Price, Problem, Store};

#[link(name = "StoreKit", kind = "framework")]
unsafe extern "C" {}

// SKPaymentTransactionState
const PURCHASED: isize = 1;
const FAILED: isize = 2;
const RESTORED: isize = 3;
const DEFERRED: isize = 4;
// SKErrorPaymentCancelled
const CANCELLED: isize = 2;
// NSNumberFormatterCurrencyStyle
const CURRENCY: usize = 2;

struct Link {
    events: mpsc::Sender<Event>,
    ctx: egui::Context,
    product: AtomicPtr<AnyObject>,
}

static LINK: OnceLock<Link> = OnceLock::new();

// StoreKit does not promise the thread its delegates are called on.
fn post(event: Event) {
    if let Some(link) = LINK.get() {
        let _ = link.events.send(event);
        link.ctx.request_repaint();
    }
}

unsafe fn string(text: &str) -> *mut AnyObject {
    let text = CString::new(text).unwrap_or_default();
    unsafe { msg_send![class!(NSString), stringWithUTF8String: text.as_ptr()] }
}

unsafe fn text(string: *mut AnyObject) -> String {
    if string.is_null() {
        return String::new();
    }
    unsafe {
        let bytes: *const c_char = msg_send![string, UTF8String];
        if bytes.is_null() {
            return String::new();
        }
        CStr::from_ptr(bytes).to_string_lossy().into_owned()
    }
}

unsafe fn price_of(product: *mut AnyObject) -> Price {
    unsafe {
        let amount: *mut AnyObject = msg_send![product, price];
        let locale: *mut AnyObject = msg_send![product, priceLocale];
        if amount.is_null() || locale.is_null() {
            return Price {
                shown: String::new(),
                plain: String::new(),
            };
        }
        let maker: *mut AnyObject = msg_send![class!(NSNumberFormatter), new];
        let _: () = msg_send![maker, setNumberStyle: CURRENCY];
        let _: () = msg_send![maker, setLocale: locale];
        let shown: *mut AnyObject = msg_send![maker, stringFromNumber: amount];
        let shown = text(shown);
        let _: () = msg_send![maker, release];
        let number: *mut AnyObject = msg_send![amount, stringValue];
        let code: *mut AnyObject = msg_send![locale, currencyCode];
        Price {
            shown,
            plain: format!("{} {}", text(number), text(code)).trim().to_owned(),
        }
    }
}

unsafe fn failure(error: *mut AnyObject) -> Event {
    if error.is_null() {
        return Event::Failed(Problem::Told(String::new()));
    }
    unsafe {
        let domain: *mut AnyObject = msg_send![error, domain];
        let code: isize = msg_send![error, code];
        if code == CANCELLED && text(domain) == "SKErrorDomain" {
            return Event::Cancelled;
        }
        let words: *mut AnyObject = msg_send![error, localizedDescription];
        Event::Failed(Problem::Told(text(words)))
    }
}

unsafe fn remember() {
    unsafe {
        let defaults: *mut AnyObject = msg_send![class!(NSUserDefaults), standardUserDefaults];
        if !defaults.is_null() {
            let _: () = msg_send![defaults, setBool: true, forKey: string(PRODUCT)];
        }
    }
}

unsafe fn remembered() -> bool {
    unsafe {
        let defaults: *mut AnyObject = msg_send![class!(NSUserDefaults), standardUserDefaults];
        !defaults.is_null() && msg_send![defaults, boolForKey: string(PRODUCT)]
    }
}

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

const RTLD_NOW: i32 = 2;
#[cfg(target_os = "ios")]
const WATCHER: &CStr = c"@executable_path/Frameworks/FractiquePro.framework/FractiquePro";
#[cfg(target_os = "macos")]
const WATCHER: &CStr = c"@executable_path/../Frameworks/FractiquePro.framework/FractiquePro";

extern "C" fn told(owned: bool) {
    if owned {
        unsafe { remember() };
        post(Event::Owned);
    }
}

// A purchase made on another device of the Apple Account: only StoreKit 2 tells it without asking the person,
// and only Swift reaches StoreKit 2, so the Xcode projects put a small framework beside the binary.
fn watch() {
    unsafe {
        let handle = dlopen(WATCHER.as_ptr(), RTLD_NOW);
        if handle.is_null() {
            return;
        }
        let found = dlsym(handle, c"fractique_pro_watch".as_ptr());
        if found.is_null() {
            return;
        }
        let watch: extern "C" fn(*const c_char, extern "C" fn(bool)) = std::mem::transmute(found);
        let product = CString::new(PRODUCT).unwrap_or_default();
        watch(product.as_ptr(), told);
    }
}

extern "C-unwind" fn answered(_this: &AnyObject, _cmd: Sel, _request: *mut AnyObject, response: *mut AnyObject) {
    let Some(link) = LINK.get() else {
        return;
    };
    if response.is_null() {
        post(Event::Failed(Problem::NoProduct));
        return;
    }
    unsafe {
        let products: *mut AnyObject = msg_send![response, products];
        let product: *mut AnyObject = if products.is_null() {
            null_mut()
        } else {
            msg_send![products, firstObject]
        };
        if product.is_null() {
            post(Event::Failed(Problem::NoProduct));
            return;
        }
        let product: *mut AnyObject = msg_send![product, retain];
        let before = link.product.swap(product, Ordering::AcqRel);
        if !before.is_null() {
            let _: () = msg_send![before, release];
        }
        post(Event::Product(price_of(product)));
    }
}

extern "C-unwind" fn refused(_this: &AnyObject, _cmd: Sel, _request: *mut AnyObject, error: *mut AnyObject) {
    post(unsafe { failure(error) });
}

extern "C-unwind" fn updated(_this: &AnyObject, _cmd: Sel, queue: *mut AnyObject, transactions: *mut AnyObject) {
    if queue.is_null() || transactions.is_null() {
        return;
    }
    unsafe {
        let count: usize = msg_send![transactions, count];
        for index in 0..count {
            let transaction: *mut AnyObject = msg_send![transactions, objectAtIndex: index];
            if transaction.is_null() {
                continue;
            }
            let state: isize = msg_send![transaction, transactionState];
            match state {
                PURCHASED | RESTORED => {
                    let payment: *mut AnyObject = msg_send![transaction, payment];
                    let name: *mut AnyObject = if payment.is_null() {
                        null_mut()
                    } else {
                        msg_send![payment, productIdentifier]
                    };
                    if text(name) == PRODUCT {
                        remember();
                        post(Event::Owned);
                    }
                    let _: () = msg_send![queue, finishTransaction: transaction];
                }
                FAILED => {
                    let error: *mut AnyObject = msg_send![transaction, error];
                    post(failure(error));
                    let _: () = msg_send![queue, finishTransaction: transaction];
                }
                DEFERRED => post(Event::Failed(Problem::Deferred)),
                _ => {}
            }
        }
    }
}

extern "C-unwind" fn restored(_this: &AnyObject, _cmd: Sel, _queue: *mut AnyObject) {
    post(Event::Restored);
}

extern "C-unwind" fn not_restored(_this: &AnyObject, _cmd: Sel, _queue: *mut AnyObject, error: *mut AnyObject) {
    post(unsafe { failure(error) });
}

fn listener() -> Option<&'static AnyClass> {
    let mut builder = ClassBuilder::new(c"FractiqueStore", class!(NSObject))?;
    unsafe {
        builder.add_method(
            sel!(productsRequest:didReceiveResponse:),
            answered as extern "C-unwind" fn(_, _, _, _),
        );
        builder.add_method(
            sel!(request:didFailWithError:),
            refused as extern "C-unwind" fn(_, _, _, _),
        );
        builder.add_method(
            sel!(paymentQueue:updatedTransactions:),
            updated as extern "C-unwind" fn(_, _, _, _),
        );
        builder.add_method(
            sel!(paymentQueueRestoreCompletedTransactionsFinished:),
            restored as extern "C-unwind" fn(_, _, _),
        );
        builder.add_method(
            sel!(paymentQueue:restoreCompletedTransactionsFailedWithError:),
            not_restored as extern "C-unwind" fn(_, _, _, _),
        );
    }
    for name in [c"SKProductsRequestDelegate", c"SKPaymentTransactionObserver"] {
        if let Some(protocol) = AnyProtocol::get(name) {
            builder.add_protocol(protocol);
        }
    }
    Some(builder.register())
}

struct Apple {
    listener: *mut AnyObject,
    request: *mut AnyObject,
}

impl Apple {
    unsafe fn drop_request(&mut self) {
        if self.request.is_null() {
            return;
        }
        unsafe {
            let _: () = msg_send![self.request, setDelegate: null_mut::<AnyObject>()];
            let _: () = msg_send![self.request, cancel];
            let _: () = msg_send![self.request, release];
        }
        self.request = null_mut();
    }
}

impl Backend for Apple {
    fn load(&mut self) {
        if self.listener.is_null() {
            post(Event::Failed(Problem::NoProduct));
            return;
        }
        unsafe {
            self.drop_request();
            let names: *mut AnyObject = msg_send![class!(NSSet), setWithObject: string(PRODUCT)];
            let request: *mut AnyObject = msg_send![class!(SKProductsRequest), alloc];
            let request: *mut AnyObject = msg_send![request, initWithProductIdentifiers: names];
            if request.is_null() {
                post(Event::Failed(Problem::NoProduct));
                return;
            }
            let _: () = msg_send![request, setDelegate: self.listener];
            let _: () = msg_send![request, start];
            self.request = request;
        }
    }

    fn buy(&mut self) {
        let product = LINK
            .get()
            .map_or(null_mut(), |link| link.product.load(Ordering::Acquire));
        if product.is_null() {
            post(Event::Failed(Problem::NoProduct));
            return;
        }
        unsafe {
            let allowed: bool = msg_send![class!(SKPaymentQueue), canMakePayments];
            if !allowed {
                post(Event::Failed(Problem::NotAllowed));
                return;
            }
            let payment: *mut AnyObject = msg_send![class!(SKPayment), paymentWithProduct: product];
            let queue: *mut AnyObject = msg_send![class!(SKPaymentQueue), defaultQueue];
            if payment.is_null() || queue.is_null() {
                post(Event::Failed(Problem::Told(String::new())));
                return;
            }
            let _: () = msg_send![queue, addPayment: payment];
        }
    }

    fn restore(&mut self) {
        unsafe {
            let queue: *mut AnyObject = msg_send![class!(SKPaymentQueue), defaultQueue];
            if queue.is_null() || self.listener.is_null() {
                post(Event::Failed(Problem::Told(String::new())));
                return;
            }
            let _: () = msg_send![queue, restoreCompletedTransactions];
        }
    }
}

pub fn open(ctx: &egui::Context) -> Store {
    let (sender, events) = mpsc::channel();
    let first = LINK
        .set(Link {
            events: sender,
            ctx: ctx.clone(),
            product: AtomicPtr::new(null_mut()),
        })
        .is_ok();
    let mut backend = Apple {
        listener: null_mut(),
        request: null_mut(),
    };
    if first && let Some(class) = listener() {
        unsafe {
            let listener: *mut AnyObject = msg_send![class, new];
            let queue: *mut AnyObject = msg_send![class!(SKPaymentQueue), defaultQueue];
            if !listener.is_null() && !queue.is_null() {
                // A purchase that was paid for while the app was closed arrives through this observer.
                let _: () = msg_send![queue, addTransactionObserver: listener];
                backend.listener = listener;
            }
        }
    }
    let owned = unsafe { remembered() };
    if first && !owned {
        watch();
    }
    Store::new(owned, events, Box::new(backend))
}
