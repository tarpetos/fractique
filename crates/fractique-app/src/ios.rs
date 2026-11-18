use std::ffi::{CString, c_void};
use std::path::Path;
use std::ptr::null_mut;

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

pub fn share(path: &Path) {
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
        // An iPad shows this sheet as a popover and stops the app when it has no view to hang on.
        let popover: *mut AnyObject = msg_send![sheet, popoverPresentationController];
        if !popover.is_null() {
            let view: *mut AnyObject = msg_send![root, view];
            let _: () = msg_send![popover, setSourceView: view];
        }
        let _: () = msg_send![root, presentViewController: sheet, animated: true, completion: null_mut::<AnyObject>()];
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
