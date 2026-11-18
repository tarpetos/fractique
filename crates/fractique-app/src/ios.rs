use std::ffi::{CString, c_void};
use std::path::Path;
use std::ptr::null_mut;

use objc2::encode::{Encode, Encoding};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {
    fn UIImageWriteToSavedPhotosAlbum(
        image: *mut AnyObject,
        target: *mut AnyObject,
        selector: *const c_void,
        context: *mut c_void,
    );
}

unsafe fn string(text: &str) -> *mut AnyObject {
    let text = CString::new(text).unwrap_or_default();
    unsafe { msg_send![class!(NSString), stringWithUTF8String: text.as_ptr()] }
}

pub fn save_to_photos(path: &Path) -> bool {
    unsafe {
        let image: *mut AnyObject =
            msg_send![class!(UIImage), imageWithContentsOfFile: string(&path.to_string_lossy())];
        if image.is_null() {
            return false;
        }
        UIImageWriteToSavedPhotosAlbum(image, null_mut(), std::ptr::null(), null_mut());
        true
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Frame {
    origin: [f64; 2],
    size: [f64; 2],
}

unsafe impl Encode for Frame {
    const ENCODING: Encoding = Encoding::Struct(
        "CGRect",
        &[
            Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]),
            Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]),
        ],
    );
}

// `from` is the button the sheet comes out of: left, top, width, height in points.
pub fn share(path: &Path, from: [f32; 4]) {
    unsafe {
        let url: *mut AnyObject = msg_send![class!(NSURL), fileURLWithPath: string(&path.to_string_lossy())];
        let items: *mut AnyObject = msg_send![class!(NSArray), arrayWithObject: url];
        let sheet: *mut AnyObject = msg_send![class!(UIActivityViewController), alloc];
        let sheet: *mut AnyObject =
            msg_send![sheet, initWithActivityItems: items, applicationActivities: null_mut::<AnyObject>()];
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        let scenes: *mut AnyObject = msg_send![app, connectedScenes];
        let scene: *mut AnyObject = msg_send![scenes, anyObject];
        if scene.is_null() || sheet.is_null() {
            return;
        }
        let windows: *mut AnyObject = msg_send![scene, windows];
        let window: *mut AnyObject = msg_send![windows, firstObject];
        let root: *mut AnyObject = msg_send![window, rootViewController];
        if root.is_null() {
            return;
        }
        let device: *mut AnyObject = msg_send![class!(UIDevice), currentDevice];
        let idiom: isize = msg_send![device, userInterfaceIdiom];
        if idiom == 1 {
            // An iPad shows the sheet as a popover and stops the app when the popover has no place to grow from.
            // An iPhone places it by itself.
            let popover: *mut AnyObject = msg_send![sheet, popoverPresentationController];
            if !popover.is_null() {
                let view: *mut AnyObject = msg_send![root, view];
                let frame = Frame {
                    origin: [f64::from(from[0]), f64::from(from[1])],
                    size: [f64::from(from[2]).max(1.0), f64::from(from[3]).max(1.0)],
                };
                let _: () = msg_send![popover, setSourceView: view];
                let _: () = msg_send![popover, setSourceRect: frame];
            }
        }
        let _: () = msg_send![root, presentViewController: sheet, animated: true, completion: null_mut::<AnyObject>()];
    }
}

unsafe fn text(string: *mut AnyObject) -> String {
    if string.is_null() {
        return String::new();
    }
    unsafe {
        let bytes: *const std::ffi::c_char = msg_send![string, UTF8String];
        if bytes.is_null() {
            return String::new();
        }
        std::ffi::CStr::from_ptr(bytes).to_string_lossy().into_owned()
    }
}

// The date and the time in the words and order of the phone's own region.
pub fn clock() -> (String, String) {
    unsafe {
        let now: *mut AnyObject = msg_send![class!(NSDate), date];
        let time: *mut AnyObject =
            msg_send![class!(NSDateFormatter), localizedStringFromDate: now, dateStyle: 0usize, timeStyle: 1usize];
        let maker: *mut AnyObject = msg_send![class!(NSDateFormatter), new];
        let _: () = msg_send![maker, setLocalizedDateFormatFromTemplate: string("EEEdMMM")];
        let date: *mut AnyObject = msg_send![maker, stringFromDate: now];
        let texts = (text(date), text(time));
        let _: () = msg_send![maker, release];
        texts
    }
}

pub fn tick(strong: bool) {
    unsafe {
        let style: isize = if strong { 1 } else { 0 };
        let maker: *mut AnyObject = msg_send![class!(UIImpactFeedbackGenerator), alloc];
        let maker: *mut AnyObject = msg_send![maker, initWithStyle: style];
        if maker.is_null() {
            return;
        }
        let _: () = msg_send![maker, impactOccurred];
        let _: () = msg_send![maker, release];
    }
}
