//! The saved lighting state, `$XDG_CONFIG_HOME/g512-control/state.toml`.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::state::{LightingState, effect_from_name, format_hex, parse_hex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved {
    pub state: LightingState,
    /// Set while disconnected; applied when the keyboard next connects.
    pub pending: bool,
}

#[derive(Serialize, Deserialize)]
struct File {
    effect: String,
    color: String,
    speed: u8,
    brightness: u8,
    #[serde(default)]
    pending: bool,
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Store { path }
    }

    /// `$XDG_CONFIG_HOME/g512-control/state.toml`, falling back to `~/.config`.
    pub fn default_path() -> Option<PathBuf> {
        let base = env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|h| Path::new(&h).join(".config")))?;
        Some(base.join("g512-control").join("state.toml"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved state. A missing file is `None`; an unreadable or invalid
    /// one is `None` with a warning, so a bad file never stops the service.
    pub fn load(&self) -> Option<Saved> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return None,
            Err(e) => {
                eprintln!("warning: cannot read {}: {e}; ignoring it", self.path.display());
                return None;
            }
        };
        match decode(&text) {
            Ok(saved) => Some(saved),
            Err(e) => {
                eprintln!("warning: invalid state file {}: {e}; ignoring it", self.path.display());
                None
            }
        }
    }

    /// Write atomically: a temporary file in the same directory, then rename.
    pub fn save(&self, saved: &Saved) -> io::Result<()> {
        let dir = self.path.parent().ok_or_else(|| io::Error::other("state path has no parent"))?;
        fs::create_dir_all(dir)?;
        let tmp = dir.join(".state.toml.tmp");
        fs::write(&tmp, encode(saved))?;
        fs::rename(&tmp, &self.path)
    }
}

fn encode(saved: &Saved) -> String {
    let s = &saved.state;
    let file = File {
        effect: s.effect.name().to_string(),
        color: format_hex(s.color),
        speed: s.speed,
        brightness: s.brightness,
        pending: saved.pending,
    };
    toml::to_string(&file).expect("state always serialises")
}

fn decode(text: &str) -> Result<Saved, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let state = LightingState {
        effect: effect_from_name(&file.effect).map_err(|e| e.to_string())?,
        color: parse_hex(&file.color).ok_or_else(|| format!("invalid colour {:?}", file.color))?,
        speed: file.speed,
        brightness: file.brightness,
    };
    // Range checks against the device happen when the state is applied.
    Ok(Saved { state, pending: file.pending })
}

#[cfg(test)]
mod tests {
    use g512_hidpp::effects::{EffectKind, Rgb};

    use super::*;

    fn temp_store(name: &str) -> Store {
        let dir = env::temp_dir().join(format!("g512d-store-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Store::new(dir.join("g512-control").join("state.toml"))
    }

    #[test]
    fn round_trip() {
        let store = temp_store("round-trip");
        let saved = Saved {
            state: LightingState { effect: EffectKind::Ripple, color: Rgb::new(0, 0xff, 0xff), speed: 40, brightness: 75 },
            pending: true,
        };
        store.save(&saved).unwrap();
        assert_eq!(store.load(), Some(saved));
        let text = fs::read_to_string(store.path()).unwrap();
        assert!(text.contains("effect = \"Ripple\"") && text.contains("color = \"#00ffff\""), "{text}");
        fs::remove_dir_all(store.path().parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn missing_file_is_none() {
        assert_eq!(temp_store("missing").load(), None);
    }

    #[test]
    fn corrupt_file_is_ignored() {
        let store = temp_store("corrupt");
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        for bad in ["not toml at all [[", "effect = \"Rainbow\"\ncolor = \"#ffffff\"\nspeed = 1\nbrightness = 1\n"] {
            fs::write(store.path(), bad).unwrap();
            assert_eq!(store.load(), None, "{bad:?}");
        }
        fs::remove_dir_all(store.path().parent().unwrap().parent().unwrap()).unwrap();
    }
}
