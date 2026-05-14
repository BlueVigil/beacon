#[cfg(unix)] use std::os::unix::fs::PermissionsExt;
use std::{
   path::{
      Path,
      PathBuf,
   },
   process::Command,
};

use anyhow::{
   Context as _,
   Result,
   anyhow,
   bail,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeensyDevice {
   pub raw_line: String,
   pub label:    String,
}

#[derive(Clone, Debug)]
pub struct Tycmd {
   executable: PathBuf,
}

#[derive(Clone, Debug)]
pub struct CommandOutput {
   pub status_code: Option<i32>,
   pub stdout:      String,
   pub stderr:      String,
}

impl CommandOutput {
   pub fn is_success(&self) -> bool {
      self.status_code == Some(0)
   }
}

impl Tycmd {
   pub fn resolve() -> Result<Self> {
      let candidates = [packaged_arch_resource_path(), dev_arch_resource_path()];

      for path in candidates.into_iter().flatten() {
         if path.exists() {
            validate_executable(&path)?;
            return Ok(Self { executable: path });
         }
      }

      bail!(MissingTycmd {
         expected_path: dev_arch_resource_path().unwrap_or_else(|| PathBuf::from("vendor/tycmd")),
      });
   }

   pub fn executable(&self) -> &Path {
      &self.executable
   }

   pub fn list(&self) -> Result<CommandOutput> {
      self.run("list", &[])
   }

   pub fn identify(&self, hex_path: &Path) -> Result<CommandOutput> {
      let hex = hex_path
         .to_str()
         .ok_or_else(|| anyhow!("firmware path is not valid UTF-8: {}", hex_path.display()))?;

      self.run("identify", &[hex])
   }

   pub fn upload(&self, hex_path: &Path) -> Result<CommandOutput> {
      let hex = hex_path
         .to_str()
         .ok_or_else(|| anyhow!("firmware path is not valid UTF-8: {}", hex_path.display()))?;

      self.run("upload", &[hex])
   }

   pub fn upload_wait(&self, hex_path: &Path) -> Result<CommandOutput> {
      let hex = hex_path
         .to_str()
         .ok_or_else(|| anyhow!("firmware path is not valid UTF-8: {}", hex_path.display()))?;

      self.run("upload", &["--wait", hex])
   }

   fn run(&self, subcommand: &str, args: &[&str]) -> Result<CommandOutput> {
      validate_executable(&self.executable)?;

      let output = Command::new(&self.executable)
         .arg(subcommand)
         .args(args)
         .output()
         .with_context(|| format!("failed to run tycmd {subcommand}"))?;

      Ok(CommandOutput {
         status_code: output.status.code(),
         stdout:      String::from_utf8_lossy(&output.stdout).into_owned(),
         stderr:      String::from_utf8_lossy(&output.stderr).into_owned(),
      })
   }
}

#[derive(Debug)]
pub struct MissingTycmd {
   pub expected_path: PathBuf,
}

impl std::fmt::Display for MissingTycmd {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
         f,
         "bundled tycmd sidecar is missing at {}",
         self.expected_path.display()
      )
   }
}

impl std::error::Error for MissingTycmd {}

pub fn parse_devices(output: &str) -> Vec<TeensyDevice> {
   output
      .lines()
      .map(str::trim)
      .filter(|line| !line.is_empty())
      .filter(|line| !is_obvious_header(line))
      .map(|line| {
         TeensyDevice {
            raw_line: line.to_string(),
            label:    line.to_string(),
         }
      })
      .collect()
}

pub fn is_hex_file(path: &Path) -> bool {
   path
      .extension()
      .and_then(|extension| extension.to_str())
      .is_some_and(|extension| extension.eq_ignore_ascii_case("hex"))
}

pub fn expected_resource_path() -> PathBuf {
   dev_arch_resource_path().unwrap_or_else(|| {
      PathBuf::from("vendor")
         .join("tycmd")
         .join(platform_arch_directory())
         .join(tycmd_resource_name())
   })
}

pub fn tycmd_resource_name() -> &'static str {
   if cfg!(windows) { "tycmd.exe" } else { "tycmd" }
}

fn platform_arch_directory() -> &'static str {
   match (std::env::consts::OS, std::env::consts::ARCH) {
      ("macos", "aarch64" | "arm64") => "macos-arm64",
      ("macos", "x86_64") => "macos-x86_64",
      ("windows", "aarch64" | "arm64") => "windows-arm64",
      ("windows", "x86_64") => "windows-x86_64",
      ("linux", "aarch64" | "arm64") => "linux-arm64",
      ("linux", "x86_64") => "linux-x86_64",
      _ => "unknown",
   }
}

fn resource_root() -> PathBuf {
   PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("vendor")
      .join("tycmd")
}

fn dev_arch_resource_path() -> Option<PathBuf> {
   Some(
      resource_root()
         .join(platform_arch_directory())
         .join(tycmd_resource_name()),
   )
}

fn packaged_resource_root() -> Option<PathBuf> {
   let executable = std::env::current_exe().ok()?;

   for ancestor in executable.ancestors() {
      if ancestor
         .extension()
         .is_some_and(|extension| extension == "app")
      {
         return Some(ancestor.join("Contents").join("Resources").join("tycmd"));
      }
   }

   executable
      .parent()
      .map(|app_dir| app_dir.join("resources").join("tycmd"))
}

fn packaged_arch_resource_path() -> Option<PathBuf> {
   Some(
      packaged_resource_root()?
         .join(platform_arch_directory())
         .join(tycmd_resource_name()),
   )
}

fn validate_executable(path: &Path) -> Result<()> {
   if !path.exists() {
      bail!("tycmd sidecar does not exist at {}", path.display());
   }

   if !path.is_file() {
      bail!("tycmd sidecar is not a file at {}", path.display());
   }

   #[cfg(unix)]
   {
      let mode = path
         .metadata()
         .with_context(|| format!("failed to inspect {}", path.display()))?
         .permissions()
         .mode();

      if mode & 0o111 == 0 {
         bail!("tycmd sidecar is not executable at {}", path.display());
      }
   }

   Ok(())
}

fn is_obvious_header(line: &str) -> bool {
   let lowercase = line.to_ascii_lowercase();

   line.starts_with("---")
      || lowercase == "teensy"
      || lowercase.starts_with("found ")
      || lowercase.contains("serial")
         && lowercase.contains("board")
         && lowercase.contains("firmware")
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn empty_device_output_returns_no_devices() {
      assert!(parse_devices("").is_empty());
   }

   #[test]
   fn single_raw_line_returns_one_device() {
      let devices = parse_devices("123456 Teensy 4.1 Bootloader");

      assert_eq!(devices.len(), 1);
      assert_eq!(devices[0].label, "123456 Teensy 4.1 Bootloader");
   }

   #[test]
   fn multiple_raw_lines_return_multiple_devices() {
      let devices = parse_devices("a Teensy 4.0\nb Teensy 4.1\n");

      assert_eq!(devices.len(), 2);
      assert_eq!(devices[0].raw_line, "a Teensy 4.0");
      assert_eq!(devices[1].raw_line, "b Teensy 4.1");
   }

   #[test]
   fn whitespace_lines_are_ignored() {
      let devices = parse_devices("\n  \n  a Teensy\n\t\n");

      assert_eq!(devices.len(), 1);
      assert_eq!(devices[0].label, "a Teensy");
   }

   #[test]
   fn obvious_headers_are_ignored() {
      let devices = parse_devices("Serial Board Firmware\n---\nabc Teensy 4.1");

      assert_eq!(devices.len(), 1);
      assert_eq!(devices[0].label, "abc Teensy 4.1");
   }

   #[test]
   fn hex_extension_is_case_insensitive() {
      assert!(is_hex_file(Path::new("firmware.hex")));
      assert!(is_hex_file(Path::new("FIRMWARE.HEX")));
      assert!(!is_hex_file(Path::new("firmware.bin")));
      assert!(!is_hex_file(Path::new("firmware")));
   }

   #[test]
   fn resource_name_matches_platform() {
      if cfg!(windows) {
         assert_eq!(tycmd_resource_name(), "tycmd.exe");
      } else {
         assert_eq!(tycmd_resource_name(), "tycmd");
      }
   }
}
