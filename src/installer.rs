use std::fs;
use std::path::PathBuf;
use std::io::Cursor;
use flate2::read::GzDecoder;
use tar::Archive;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub enum InstallStatus {
    Idle,
    FetchingVersion,
    Downloading(f32), // Progress percentage
    Extracting,
    Completed,
    Running,
    Stopped,
    Error(String),
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    prerelease: bool,
    draft: bool,
}

pub struct RethInstaller {
    status: InstallStatus,
}

impl RethInstaller {
    pub fn new() -> Self {
        Self {
            status: InstallStatus::Idle,
        }
    }

    pub fn status(&self) -> &InstallStatus {
        &self.status
    }

    pub async fn install_reth(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.install_reth_inner().await {
            Ok(()) => Ok(()),
            Err(e) => {
                self.status = InstallStatus::Error(e.to_string());
                Err(e)
            }
        }
    }

    async fn install_reth_inner(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Fetch latest version
        self.status = InstallStatus::FetchingVersion;
        let version = fetch_latest_version().await?;
        
        // Determine platform
        let platform = get_platform();
        
        // Construct download URL
        let binary_name = format!("reth-{}-{}.tar.gz", version, platform);
        let download_url = format!(
            "https://github.com/paradigmxyz/reth/releases/download/{}/{}",
            version, binary_name
        );

        // Download binary
        self.status = InstallStatus::Downloading(0.0);
        let response = reqwest::get(&download_url).await?;
        let total_size = response.content_length().unwrap_or(0);
        
        let mut downloaded = 0;
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            bytes.extend_from_slice(&chunk);
            
            if total_size > 0 {
                let progress = (downloaded as f32 / total_size as f32) * 100.0;
                self.status = InstallStatus::Downloading(progress);
            }
        }

        // Extract binary
        self.status = InstallStatus::Extracting;
        let install_dir = get_install_directory()?;
        fs::create_dir_all(&install_dir)?;
        
        let tar = GzDecoder::new(Cursor::new(bytes));
        let mut archive = Archive::new(tar);
        archive.unpack(&install_dir)?;

        // Make binary executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let binary_path = install_dir.join("reth");
            let metadata = fs::metadata(&binary_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&binary_path, permissions)?;
        }

        self.status = InstallStatus::Completed;
        Ok(())
    }
}

fn get_platform() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "x86_64-unknown-linux-gnu";
    
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "aarch64-unknown-linux-gnu";
    
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "x86_64-apple-darwin";
    
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "aarch64-apple-darwin";
    
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "x86_64-pc-windows-gnu";
    
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    panic!("Unsupported platform");
}

async fn fetch_latest_version() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    const FALLBACK_VERSION: &str = "v1.5.0";
    
    let url = "https://api.github.com/repos/paradigmxyz/reth/releases/latest";
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    
    match client
        .get(url)
        .header("User-Agent", "reth-desktop/1.0")
        .send()
        .await
    {
        Ok(response) => {
            if !response.status().is_success() {
                eprintln!("GitHub API returned HTTP {}, using fallback version {}", response.status(), FALLBACK_VERSION);
                return Ok(FALLBACK_VERSION.to_string());
            }
            
            match response.json::<GitHubRelease>().await {
                Ok(release) => {
                    // Skip prerelease and draft versions
                    if release.prerelease || release.draft {
                        eprintln!("Latest release is prerelease/draft, using fallback version {}", FALLBACK_VERSION);
                        return Ok(FALLBACK_VERSION.to_string());
                    }
                    
                    println!("Fetched latest version: {}", release.tag_name);
                    Ok(release.tag_name)
                }
                Err(e) => {
                    eprintln!("Failed to parse GitHub API response: {}, using fallback version {}", e, FALLBACK_VERSION);
                    Ok(FALLBACK_VERSION.to_string())
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch latest version from GitHub: {}, using fallback version {}", e, FALLBACK_VERSION);
            Ok(FALLBACK_VERSION.to_string())
        }
    }
}

fn get_install_directory() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(".reth-desktop").join("bin"))
}