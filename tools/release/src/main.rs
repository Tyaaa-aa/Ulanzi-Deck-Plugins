//! Release helper, run by CI:
//!
//! ```text
//! ulanzi-release sign <file>                     writes <file>.minisig
//! ulanzi-release index <dir> --repo <owner/name> prints index.json for the packages in <dir>
//! ```
//!
//! `sign` reads the secret key (the text of a minisign `.key` file) from `MINISIGN_KEY` and its
//! password from `MINISIGN_PASSWORD`, and refuses to write a signature the app's public key would
//! not accept. The signed trusted comment names the file, so a signature cannot be moved to another
//! package.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use minisign::{PublicKey, SecretKey, SecretKeyBox, SignatureBox};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use ulanzi_core::plugin::Manifest;

/// The key the app trusts (`minisign -G`, key id 58804B29417BFCF6).
pub const PUBLIC_KEY: &str = "RWT2/HtBKUuAWImA/92lW+GmNaz2nfsNJrxbGrfVmSj/sPyQlngs0pCf";

/// The index format the app reads.
const INDEX_FORMAT: u32 = 1;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["sign", file] => sign_from_env(Path::new(file)),
        ["index", dir, "--repo", repo] => index(Path::new(dir), repo).map(|v| {
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        }),
        _ => Err("usage: ulanzi-release sign <file> | index <dir> --repo <owner/name>".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ulanzi-release: {e}");
            ExitCode::FAILURE
        }
    }
}

fn sign_from_env(file: &Path) -> Result<(), String> {
    let key = std::env::var("MINISIGN_KEY").map_err(|_| "MINISIGN_KEY is not set")?;
    let password = std::env::var("MINISIGN_PASSWORD").map_err(|_| "MINISIGN_PASSWORD is not set")?;
    let sk = SecretKeyBox::from_string(&key)
        .and_then(|b| b.into_secret_key(Some(password)))
        .map_err(|e| format!("cannot open the secret key: {e}"))?;
    let pk = PublicKey::from_base64(PUBLIC_KEY).map_err(|e| e.to_string())?;
    let sig = sign_file(file, &sk, &pk)?;
    let out = sig_path(file);
    std::fs::write(&out, sig).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    eprintln!("signed {}", file.display());
    Ok(())
}

fn sig_path(file: &Path) -> PathBuf {
    let mut s = file.as_os_str().to_owned();
    s.push(".minisig");
    PathBuf::from(s)
}

/// The signature of `file`, checked against `pk` before it is returned.
pub fn sign_file(file: &Path, sk: &SecretKey, pk: &PublicKey) -> Result<String, String> {
    let data = std::fs::read(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let name = file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("the file needs a plain name")?;
    let trusted = format!("file:{name}");
    let sig =
        minisign::sign(None, sk, Cursor::new(&data), Some(&trusted), None).map_err(|e| e.to_string())?;
    let text = sig.into_string();
    let check = SignatureBox::from_string(&text).map_err(|e| e.to_string())?;
    minisign::verify(pk, &check, Cursor::new(&data), true, false, false)
        .map_err(|_| "the secret key does not match the app's public key".to_string())?;
    Ok(text)
}

/// The manifest inside a `.ulanziplugin` package.
fn package_manifest(path: &Path) -> Result<Manifest, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut text = String::new();
    zip.by_name("plugin.toml")
        .map_err(|_| format!("{} has no plugin.toml", path.display()))?
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    Manifest::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The release tag of a plugin version.
pub fn tag(id: &str, version: &str) -> String {
    format!("{id}-v{version}")
}

/// The index of every `.ulanziplugin` in `dir`, by id. Each entry points at the plugin's release
/// in `repo` and carries the package's SHA-256, so signing the index covers every package.
pub fn index(dir: &Path, repo: &str) -> Result<Value, String> {
    let mut entries = Vec::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "ulanziplugin"))
        .collect();
    files.sort();
    for path in files {
        let m = package_manifest(&path)?;
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let tag = tag(&m.id, &m.version);
        let raw = |p: &str| {
            format!(
                "https://raw.githubusercontent.com/{repo}/{tag}/plugins/{}/{p}",
                m.id
            )
        };
        let download = format!("https://github.com/{repo}/releases/download/{tag}/{file}");
        entries.push(json!({
            "id": m.id,
            "name": m.name,
            "version": m.version,
            "description": m.description,
            "categories": m.categories,
            "contract": m.contract,
            "min_app": m.min_app,
            "tested_on_hardware": m.tested_on_hardware,
            "icon": m.icon.as_deref().filter(|i| i.starts_with("assets/")).map(raw),
            "readme": raw("README.md"),
            "package": {
                "url": download,
                "signature": format!("{download}.minisig"),
                "sha256": hex(&Sha256::digest(&bytes)),
                "size": bytes.len(),
            },
        }));
    }
    entries.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let ids: Vec<&str> = entries.iter().filter_map(|e| e["id"].as_str()).collect();
    if let Some(w) = ids.windows(2).find(|w| w[0] == w[1]) {
        return Err(format!("two packages of plugin {}", w[0]));
    }
    Ok(json!({ "format": INDEX_FORMAT, "plugins": entries }))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use minisign::KeyPair;

    use super::*;

    fn package(dir: &Path, id: &str, version: &str) -> PathBuf {
        let path = dir.join(format!("{id}-{version}.ulanziplugin"));
        let mut z = zip::ZipWriter::new(std::fs::File::create(&path).expect("create"));
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("plugin.toml", opts).expect("start");
        write!(
            z,
            "id = \"{id}\"\nname = \"Name\"\nversion = \"{version}\"\ncontract = \"1.2\"\nmin_app = \"2.1.0\"\nicon = \"assets/icon.svg\"\n"
        )
        .expect("write");
        z.start_file("plugin.wasm", opts).expect("start");
        z.write_all(b"\0asm").expect("write");
        z.finish().expect("finish");
        path
    }

    #[test]
    fn signatures_check_out_and_name_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = package(dir.path(), "ha", "1.2.0");
        let kp = KeyPair::generate_encrypted_keypair(Some("pw".into())).expect("keys");
        // As CI gets it: the key file's text and its password.
        let text = kp.sk.to_box(None).expect("box").into_string();
        let sk = SecretKeyBox::from_string(&text)
            .and_then(|b| b.into_secret_key(Some("pw".into())))
            .expect("open key");
        let sig = sign_file(&file, &sk, &kp.pk).expect("sign");
        let sb = SignatureBox::from_string(&sig).expect("parse");
        assert_eq!(
            sb.trusted_comment().expect("comment"),
            "file:ha-1.2.0.ulanziplugin"
        );
        let data = std::fs::read(&file).expect("read");
        assert!(minisign::verify(&kp.pk, &sb, Cursor::new(&data), true, false, false).is_ok());
        // Another key's public half refuses it: CI fails instead of shipping a bad signature.
        let other = KeyPair::generate_unencrypted_keypair().expect("keys");
        assert!(sign_file(&file, &sk, &other.pk).is_err());
        // The app's key parses.
        assert!(PublicKey::from_base64(PUBLIC_KEY).is_ok());
    }

    #[test]
    fn the_index_lists_each_package_with_its_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        package(dir.path(), "obs", "1.1.0");
        package(dir.path(), "ha", "1.2.0");
        let v = index(dir.path(), "me/repo").expect("index");
        assert_eq!(v["format"], 1);
        let p = v["plugins"].as_array().expect("plugins");
        assert_eq!(p.len(), 2);
        assert_eq!(p[0]["id"], "ha", "sorted by id");
        assert_eq!(
            p[0]["package"]["url"],
            "https://github.com/me/repo/releases/download/ha-v1.2.0/ha-1.2.0.ulanziplugin"
        );
        assert_eq!(
            p[0]["icon"],
            "https://raw.githubusercontent.com/me/repo/ha-v1.2.0/plugins/ha/assets/icon.svg"
        );
        assert_eq!(p[0]["min_app"], "2.1.0");
        assert_eq!(p[0]["package"]["sha256"].as_str().map(str::len), Some(64));
        package(dir.path(), "ha", "1.3.0");
        assert!(
            index(dir.path(), "me/repo").is_err(),
            "two versions of one plugin"
        );
    }
}
