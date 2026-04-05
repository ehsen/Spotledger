//! Guest-side bindings for SpotledgerCore host functions.
//!
//! All functions use a linear-memory ABI: serialise the request to JSON,
//! write it to a shared buffer, call the host, read back the response.

/// Host function declarations.  These are filled in by the extism runtime
/// when the plugin is instantiated.
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn sl_log(ptr: *const u8, len: usize);
    fn sl_make_gl_entries(ptr: *const u8, len: usize) -> i32;
    fn sl_get_doc(ptr: *const u8, len: usize, out_ptr: *mut u8) -> usize;
    fn sl_save_doc(ptr: *const u8, len: usize) -> i32;
    fn sl_delete_doc(ptr: *const u8, len: usize) -> i32;
}

/// Write a message to the host trace log.
pub fn log(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe { sl_log(msg.as_ptr(), msg.len()) }
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("[spotledger-pdk stub] log: {msg}");
}
