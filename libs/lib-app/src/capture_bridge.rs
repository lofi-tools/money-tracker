//! Brave native-messaging framing and owner-only inbox. No DB opened by the bridge.
use crate::nexo_capture::{Capture, MAX_BYTES, Receipt, sha256};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pairing {
    pub version: u32,
    pub connection_id: String,
    pub extension_id: String,
    pub database: PathBuf,
    pub portfolio: String,
    pub account: String,
    pub tracker_executable: Option<PathBuf>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub version: u32,
    pub connection_id: String,
    pub payload: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Ack {
    Committed { receipt: Receipt },
    Pending { request_id: String },
    Rejected { message: String },
}
pub fn read_frame(reader: &mut impl Read) -> Result<Option<Vec<u8>>> {
    let mut first = [0u8; 1];
    if reader.read(&mut first)? == 0 {
        return Ok(None);
    }
    let mut length = [0u8; 4];
    length[0] = first[0];
    reader.read_exact(&mut length[1..])?;
    let n = u32::from_ne_bytes(length) as usize;
    ensure!(n > 0 && n <= MAX_BYTES + 4096, "native frame limit");
    let mut bytes = vec![0; n];
    reader.read_exact(&mut bytes)?;
    Ok(Some(bytes))
}
pub fn write_frame(writer: &mut impl Write, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(bytes.len() < 1_000_000, "native reply too large");
    writer.write_all(&(bytes.len() as u32).to_ne_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}
pub fn root(database: &Path) -> Result<PathBuf> {
    Ok(database
        .parent()
        .context("database parent missing")?
        .join("nexo-capture"))
}
pub fn private_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(path)?;
    }
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        meta.is_dir() && !meta.file_type().is_symlink(),
        "unsafe inbox directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            meta.permissions().mode() & 0o077 == 0,
            "inbox must be owner-only"
        );
    }
    Ok(())
}
pub fn read_private(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= limit as u64,
        "unsafe/oversized local capture file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            meta.permissions().mode() & 0o077 == 0,
            "capture file must be owner-only"
        );
    }
    Ok(fs::read(path)?)
}
pub fn atomic_private(path: &Path, bytes: &[u8]) -> Result<()> {
    private_dir(path.parent().context("file parent missing")?)?;
    let mut tmp = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
impl Pairing {
    pub fn load(path: &Path) -> Result<Self> {
        let p: Self = serde_json::from_slice(&read_private(path, 32_000)?)?;
        p.validate()?;
        Ok(p)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1
                && self.connection_id.len() == 64
                && self.connection_id.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid connection"
        );
        ensure!(
            self.extension_id.len() == 32
                && self
                    .extension_id
                    .bytes()
                    .all(|b| (b'a'..=b'p').contains(&b)),
            "invalid Brave extension id"
        );
        ensure!(
            self.database.is_absolute()
                && self
                    .database
                    .file_name()
                    .is_some_and(|name| name == crate::config::DATABASE_FILENAME)
                && self.database.is_file()
                && !self.portfolio.is_empty()
                && !self.account.is_empty(),
            "paired database/account missing"
        );
        Ok(())
    }
    pub fn authorize(&self, origin: &str) -> Result<()> {
        self.validate()?;
        ensure!(
            origin == format!("chrome-extension://{}/", self.extension_id),
            "extension origin not paired"
        );
        Ok(())
    }
    pub fn enqueue(&self, e: &Envelope) -> Result<String> {
        self.validate()?;
        ensure!(
            e.version == 1 && e.connection_id == self.connection_id,
            "connection mismatch"
        );
        Capture::parse(&e.payload)?;
        let id = sha256(e.payload.as_bytes());
        let dir = root(&self.database)?;
        private_dir(&dir)?;
        let path = dir.join(format!("{id}.request.json"));
        if path.exists() {
            ensure!(
                read_private(&path, MAX_BYTES + 4096)? == serde_json::to_vec(e)?,
                "request hash conflict"
            );
        } else {
            atomic_private(&path, &serde_json::to_vec(e)?)?;
        }
        Ok(id)
    }
}
/// Called only on the existing desktop database worker. Individual bad requests
/// receive a rejection without blocking later valid requests.
pub fn drain(
    app: &crate::Application,
    resolved: &crate::config::ResolvedConfig,
) -> Result<Vec<Receipt>> {
    let dir = root(&resolved.database)?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    private_dir(&dir)?;
    let pair = Pairing::load(&dir.join("pairing.json"))?;
    ensure!(
        pair.database.canonicalize()? == resolved.database.canonicalize()?,
        "paired database mismatch"
    );
    ensure!(
        resolved
            .config
            .active_portfolio
            .as_ref()
            .map(|p| p.0.as_str())
            .unwrap_or(lib_core::DEFAULT_PORTFOLIO)
            == pair.portfolio,
        "paired portfolio is not active"
    );
    atomic_private(
        &dir.join("worker.json"),
        &serde_json::to_vec(&chrono::Utc::now())?,
    )?;
    let mut out = vec![];
    let mut requests = fs::read_dir(&dir)?.collect::<std::io::Result<Vec<_>>>()?;
    requests.sort_by_key(|e| e.file_name());
    for entry in requests
        .into_iter()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".request.json"))
        .take(10)
    {
        let filename = entry.file_name().to_string_lossy().to_string();
        let id = filename.trim_end_matches(".request.json");
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let result = dir.join(format!("{id}.result.json"));
        if result.exists() {
            continue;
        }
        let process = || -> Result<Receipt> {
            let e: Envelope =
                serde_json::from_slice(&read_private(&entry.path(), MAX_BYTES + 4096)?)?;
            ensure!(
                e.version == 1
                    && e.connection_id == pair.connection_id
                    && sha256(e.payload.as_bytes()) == id,
                "invalid queued capture"
            );
            app.ingest_nexo_capture(
                &pair.portfolio.clone().into(),
                &pair.account.clone().into(),
                &e.payload,
            )
        };
        let ack = match process() {
            Ok(r) => {
                out.push(r.clone());
                Ack::Committed { receipt: r }
            }
            Err(_) => Ack::Rejected {
                message: "Capture rejected: check schema, pairing and evidence in tracker".into(),
            },
        };
        atomic_private(&result, &serde_json::to_vec(&ack)?)?;
        fs::remove_file(entry.path())?;
    }
    Ok(out)
}
