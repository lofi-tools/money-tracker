//! Brave host. Pairing path is fixed by the installed wrapper, never by page JSON.
use anyhow::{Context, Result, ensure};
use lib_app::capture_bridge::{
    Ack, Envelope, Pairing, atomic_private, read_frame, read_private, root, write_frame,
};
use std::{
    fs, io,
    path::PathBuf,
    time::{Duration, Instant},
};
fn main() {
    if run().is_err() {
        let _ = write_frame(
            &mut io::stdout(),
            &Ack::Rejected {
                message: "Native bridge unavailable; check local pairing and tracker installation"
                    .into(),
            },
        );
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "prepare") {
        return prepare(&args[1..]);
    }
    ensure!(
        args.len() >= 3 && args[0] == "--pairing",
        "missing native pairing"
    );
    let pair = Pairing::load(&PathBuf::from(&args[1]))?;
    pair.authorize(&args[2])?;
    while let Some(bytes) = read_frame(&mut io::stdin())? {
        let reply = (|| -> Result<Ack> {
            let e: Envelope = serde_json::from_slice(&bytes)?;
            let id = pair.enqueue(&e)?;
            let dir = root(&pair.database)?;
            let result = dir.join(format!("{id}.result.json"));
            if !result.exists() {
                let alive = read_private(&dir.join("worker.json"), 1024)
                    .ok()
                    .and_then(|v| serde_json::from_slice::<chrono::DateTime<chrono::Utc>>(&v).ok())
                    .is_some_and(|t| {
                        chrono::Utc::now()
                            .signed_duration_since(t)
                            .num_seconds()
                            .abs()
                            < 8
                    });
                if !alive {
                    if let Some(exe) = &pair.tracker_executable {
                        // Trusted local setup supplies the executable. Browser cannot supply arguments/paths.
                        std::process::Command::new(exe)
                            .env("APP_DATA_DIR", pair.database.parent().unwrap())
                            .stdin(std::process::Stdio::null())
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn()?;
                    }
                }
            }
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(30) {
                if result.exists() {
                    let ack = serde_json::from_slice(&read_private(&result, 1_000_000)?)?;
                    let request = dir.join(format!("{id}.request.json"));
                    if request.exists() {
                        fs::remove_file(request)?;
                    }
                    return Ok(ack);
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Ok(Ack::Pending { request_id: id })
        })()
        .unwrap_or(Ack::Rejected {
            message: "Capture rejected or tracker unavailable; retry after checking pairing".into(),
        });
        write_frame(&mut io::stdout(), &reply)?;
    }
    Ok(())
}
/// Prepares files in an explicit output directory; does not install or grant site access.
fn prepare(a: &[String]) -> Result<()> {
    ensure!(
        (6..=7).contains(&a.len()),
        "usage: prepare OUTPUT_DIR EXTENSION_ID EXISTING_DB PORTFOLIO ACCOUNT TRACKER_EXECUTABLE [CONNECTION_ID]"
    );
    let dir = PathBuf::from(&a[0]);
    lib_app::capture_bridge::private_dir(&dir)?;
    let db = PathBuf::from(&a[2]).canonicalize()?;
    let id = if let Some(id) = a.get(6) {
        id.clone()
    } else {
        #[cfg(unix)]
        {
            use std::io::Read;
            let mut random = [0; 32];
            fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
            lib_app::nexo_capture::sha256(&random)
        }
        #[cfg(not(unix))]
        {
            anyhow::bail!("Brave setup currently supports macOS/Linux only");
        }
    };
    let pair = Pairing {
        version: 1,
        connection_id: id,
        extension_id: a[1].clone(),
        database: db,
        portfolio: a[3].clone(),
        account: a[4].clone(),
        tracker_executable: Some(PathBuf::from(&a[5]).canonicalize()?),
    };
    pair.validate()?;
    let inbox = root(&pair.database)?;
    lib_app::capture_bridge::private_dir(&inbox)?;
    let pairing = inbox.join("pairing.json");
    ensure!(
        !pairing.exists(),
        "pairing already exists; review it before replacing"
    );
    atomic_private(&pairing, &serde_json::to_vec_pretty(&pair)?)?;
    let exe = std::env::current_exe()?.canonicalize()?;
    fn quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
    let wrapper = dir.canonicalize()?.join("nexo-capture-host");
    atomic_private(
        &wrapper,
        format!(
            "#!/bin/sh\nexec {} --pairing {} \"$@\"\n",
            quote(exe.to_str().context("executable path encoding")?),
            quote(pairing.to_str().context("pairing path encoding")?)
        )
        .as_bytes(),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700))?;
    }
    let manifest = serde_json::json!({"name":"dev.money_tracker.nexo_capture","description":"Money Tracker Nexo ingestion","path":wrapper,"type":"stdio","allowed_origins":[format!("chrome-extension://{}/",pair.extension_id)]});
    atomic_private(
        &dir.join("dev.money_tracker.nexo_capture.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    println!(
        "Prepared Brave host manifest. Connection ID: {}. Install the manifest in Brave NativeMessagingHosts; files were not installed automatically.",
        pair.connection_id
    );
    Ok(())
}
