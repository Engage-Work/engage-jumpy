/// The native game process needs its own Dock icon even when a shell script launches it.
#[cfg(target_os = "macos")]
pub fn set_icon(_world: &mut bones_bevy_renderer::bevy::prelude::World) {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    use std::{ffi::CString, path::PathBuf};
    let path = PathBuf::from(std::env::var("JUMPY_ASSETS").unwrap_or_else(|_| "assets".into()))
        .join("ui/jumpy.png");
    let Ok(path) = path.canonicalize() else {
        return;
    };
    let Ok(path) = CString::new(path.to_string_lossy().as_bytes()) else {
        return;
    };
    // The exclusive startup system runs on AppKit's main thread. NSApplication retains the image.
    unsafe {
        let string: *mut Object = msg_send![class!(NSString), alloc];
        let string: *mut Object = msg_send![string, initWithUTF8String: path.as_ptr()];
        let image: *mut Object = msg_send![class!(NSImage), alloc];
        let image: *mut Object = msg_send![image, initWithContentsOfFile: string];
        if !image.is_null() {
            let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, setApplicationIconImage: image];
            let _: () = msg_send![image, release];
        }
        let _: () = msg_send![string, release];
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_icon(_world: &mut bones_bevy_renderer::bevy::prelude::World) {}
