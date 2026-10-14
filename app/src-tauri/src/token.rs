// Reads the access token Claude Desktop keeps for an instance and asks
// api.anthropic.com who it belongs to / how much of the plan is used.
// Read-only on purpose: refreshing here would rotate the refresh token the
// app itself holds and sign that instance out.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

// One client so requests to api.anthropic.com reuse the connection.
static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder().timeout(Duration::from_secs(8)).build().expect("http client")
});

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|_| "not signed in yet".to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn access_token(user_data_dir: &Path) -> Result<String, String> {
    let config = read_json(&user_data_dir.join("config.json"))?;
    // V1 can be an empty placeholder on newer builds; V2 is the real one.
    let blob = ["oauth:tokenCacheV2", "oauth:tokenCache"]
        .iter()
        .find_map(|k| config[*k].as_str().filter(|s| !s.is_empty()))
        .ok_or("not signed in yet")?;
    let blob = B64.decode(blob).map_err(|e| e.to_string())?;
    if !blob.starts_with(b"v10") {
        return Err("unknown token format".into());
    }
    let plain = decrypt(user_data_dir, &blob[3..])?;
    let cache: Value = serde_json::from_slice(&plain).map_err(|e| e.to_string())?;
    cache
        .as_object()
        .and_then(|entries| entries.values().find_map(|e| e["token"].as_str()))
        .map(str::to_string)
        .ok_or_else(|| "not signed in yet".into())
}

// Electron safeStorage = Chromium os_crypt: AES-256-GCM with a DPAPI-wrapped
// key from "Local State".
#[cfg(windows)]
fn decrypt(user_data_dir: &Path, data: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    if data.len() < 12 + 16 {
        return Err("token too short".into());
    }
    let state = read_json(&user_data_dir.join("Local State"))?;
    let wrapped = B64
        .decode(state["os_crypt"]["encrypted_key"].as_str().ok_or("no os_crypt key")?)
        .map_err(|e| e.to_string())?;
    let key = dpapi_unprotect(wrapped.strip_prefix(b"DPAPI").ok_or("unexpected key format")?)?;
    let (nonce, sealed) = data.split_at(12);
    Aes256Gcm::new_from_slice(&key)
        .map_err(|e| e.to_string())?
        .decrypt(Nonce::from_slice(nonce), sealed)
        .map_err(|_| "could not decrypt token".into())
}

#[cfg(windows)]
fn dpapi_unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(&input, None, None, None, None, 0, &mut output).map_err(|e| e.to_string())?;
        let key = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(Some(HLOCAL(output.pbData as _)));
        Ok(key)
    }
}

// macOS os_crypt: AES-128-CBC, key = PBKDF2-SHA1 of the "Claude Safe Storage"
// Keychain password (macOS asks the user once to allow access).
#[cfg(target_os = "macos")]
fn decrypt(_user_data_dir: &Path, data: &[u8]) -> Result<Vec<u8>, String> {
    use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
    // Same Keychain password for every instance - derive once; a denial is
    // not cached, so allowing access later works without a restart.
    static KEY: std::sync::OnceLock<[u8; 16]> = std::sync::OnceLock::new();
    let key = match KEY.get() {
        Some(k) => *k,
        None => {
            let k = keychain_key().ok_or("Keychain access to \"Claude Safe Storage\" was denied")?;
            *KEY.get_or_init(|| k)
        }
    };
    cbc::Decryptor::<aes::Aes128>::new(&key.into(), &[b' '; 16].into())
        .decrypt_padded_vec_mut::<Pkcs7>(data)
        .map_err(|_| "could not decrypt token".into())
}

#[cfg(target_os = "macos")]
fn keychain_key() -> Option<[u8; 16]> {
    let out = std::process::Command::new("security")
        .args(["find-generic-password", "-w", "-s", "Claude Safe Storage"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let password = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(password.as_bytes(), b"saltysalt", 1003, &mut key);
    Some(key)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn decrypt(_: &Path, _: &[u8]) -> Result<Vec<u8>, String> {
    Err("unsupported platform".into())
}

async fn api(path: &str, token: &str) -> Result<Value, String> {
    let resp = HTTP
        .get(format!("https://api.anthropic.com{path}"))
        .bearer_auth(token)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "oauth-2025-04-20")
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;
    match resp.status().as_u16() {
        200 => resp.json().await.map_err(|e| e.to_string()),
        401 => Err("signed-out or expired - open the account to refresh".into()),
        code => Err(format!("api.anthropic.com returned {code}")),
    }
}

pub async fn email(user_data_dir: &Path) -> Result<String, String> {
    let profile = api("/api/oauth/profile", &access_token(user_data_dir)?).await?;
    profile["account"]["email"].as_str().map(str::to_string).ok_or_else(|| "no email in profile".into())
}

pub async fn usage(user_data_dir: &Path) -> Result<Value, String> {
    api("/api/oauth/usage", &access_token(user_data_dir)?).await
}
