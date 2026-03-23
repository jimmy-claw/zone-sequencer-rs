use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::time::Duration;
use std::sync::OnceLock;

use lb_core::mantle::ops::channel::ChannelId;
use lb_key_management_system_service::keys::Ed25519Key;
use logos_blockchain_zone_sdk::sequencer::ZoneSequencer;
use reqwest::Url;
use tokio::runtime::Runtime;

// Global single-threaded tokio runtime, initialized once
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// Publish data to a zone channel via the blockchain node.
#[no_mangle]
pub extern "C" fn zone_publish(
    node_url: *const c_char,
    signing_key_hex: *const c_char,
    data: *const c_char,
) -> *mut c_char {
    let result = std::panic::catch_unwind(|| zone_publish_inner(node_url, signing_key_hex, data));
    match result {
        Ok(Some(s)) => s.into_raw(),
        Ok(None) => {
            eprintln!("zone_publish: returned None");
            std::ptr::null_mut()
        }
        Err(e) => {
            eprintln!("zone_publish: panicked: {:?}", e);
            std::ptr::null_mut()
        }
    }
}

fn zone_publish_inner(
    node_url: *const c_char,
    signing_key_hex: *const c_char,
    data: *const c_char,
) -> Option<CString> {
    if node_url.is_null() || signing_key_hex.is_null() || data.is_null() {
        eprintln!("zone_publish: null argument");
        return None;
    }

    let node_url_str = unsafe { CStr::from_ptr(node_url) }.to_str().ok()?;
    let signing_key_str = unsafe { CStr::from_ptr(signing_key_hex) }.to_str().ok()?;
    let data_str = unsafe { CStr::from_ptr(data) }.to_str().ok()?;

    let key_bytes: [u8; 32] = hex::decode(signing_key_str).ok()?.try_into().ok()?;
    let signing_key = Ed25519Key::from_bytes(&key_bytes);
    let channel_bytes: [u8; 32] = signing_key.public_key().to_bytes();
    let channel_id = ChannelId::from(channel_bytes);
    let url: Url = node_url_str.parse().ok()?;

    eprintln!("zone_publish: node={}, channel={}", url, hex::encode(channel_bytes));
    // Write debug marker to confirm we got here
    let _ = std::fs::write("/tmp/zone_publish_called.txt",
        format!("called: node={} channel={}", url, hex::encode(channel_bytes)));

    let data_bytes = data_str.as_bytes().to_vec();
    eprintln!("zone_publish: publishing {} bytes...", data_bytes.len());

    let rt = get_runtime();

    let inscription_id = rt.block_on(async {
        let sequencer = ZoneSequencer::init(channel_id, signing_key, url, None, None);

        let mut attempts = 0;
        loop {
            attempts += 1;
            match sequencer.publish(data_bytes.clone()).await {
                Ok(result) => {
                    let id_bytes: [u8; 32] = result.inscription_id.into();
                    let id_hex = hex::encode(id_bytes);
                    eprintln!("zone_publish: inscription_id={}", id_hex);
                    // Give sequencer actor time to post tx to node
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    return Some(id_hex);
                }
                Err(e) => {
                    if attempts > 5 {
                        eprintln!("zone_publish: failed after {} attempts: {}", attempts, e);
                        return None;
                    }
                    eprintln!("zone_publish: attempt {}: {} — retrying in 1s...", attempts, e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    })?;

    CString::new(inscription_id).ok()
}

/// Free a string returned by `zone_publish`.
#[no_mangle]
pub extern "C" fn zone_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)); }
    }
}
