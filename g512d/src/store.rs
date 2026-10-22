//! The saved settings, `$XDG_CONFIG_HOME/g512-control/state.toml`: the
//! lighting state, both key maps and the game switch.

use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    keymap::{KeyMap, parse_key},
    state::{LightingState, effect_from_name, format_hex, parse_hex},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved {
    pub state: LightingState,
    /// Set while disconnected; applied when the keyboard next connects.
    pub pending: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// `None` until a state has been set; the keyboard is then left alone.
    pub saved: Option<Saved>,
    pub custom: KeyMap,
    /// `None` until the user edits it: the built-in default applies.
    pub gaming: Option<KeyMap>,
    pub game_switch: bool,
}

impl Settings {
    pub fn gaming(&self) -> KeyMap {
        self.gaming.clone().unwrap_or_else(KeyMap::default_gaming)
    }
}

/// The file format. The state fields are optional so maps can be saved
/// before any state; v1 files have only the state fields.
#[derive(Serialize, Deserialize)]
struct File {
    #[serde(skip_serializing_if = "Option::is_none")]
    effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speed: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    brightness: Option<u8>,
    #[serde(default)]
    pending: bool,
    #[serde(default)]
    game_switch: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    custom: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gaming: Option<BTreeMap<String, String>>,
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

    /// `corrections.toml` next to the state file; the service only reads it.
    pub fn corrections_path(&self) -> PathBuf {
        self.path.with_file_name("corrections.toml")
    }

    /// The saved settings. A missing file gives the defaults; an unreadable
    /// or invalid one gives the defaults with a warning, so a bad file never
    /// stops the service.
    pub fn load(&self) -> Settings {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Settings::default(),
            Err(e) => {
                eprintln!("warning: cannot read {}: {e}; ignoring it", self.path.display());
                return Settings::default();
            }
        };
        match decode(&text) {
            Ok(settings) => settings,
            Err(e) => {
                eprintln!("warning: invalid state file {}: {e}; ignoring it", self.path.display());
                Settings::default()
            }
        }
    }

    /// Write atomically: a temporary file in the same directory, then rename.
    pub fn save(&self, settings: &Settings) -> io::Result<()> {
        let dir = self.path.parent().ok_or_else(|| io::Error::other("state path has no parent"))?;
        fs::create_dir_all(dir)?;
        let tmp = dir.join(".state.toml.tmp");
        fs::write(&tmp, encode(settings))?;
        fs::rename(&tmp, &self.path)
    }
}

fn encode_map(map: &KeyMap) -> BTreeMap<String, String> {
    map.entries().map(|(k, c)| (k.to_string(), format_hex(c))).collect()
}

fn decode_map(raw: &BTreeMap<String, String>) -> Result<KeyMap, String> {
    let entries = raw.iter().map(|(k, c)| {
        let key = parse_key(k).ok_or_else(|| format!("invalid key {k:?}"))?;
        let color = parse_hex(c).ok_or_else(|| format!("invalid colour {c:?} for key {k}"))?;
        Ok((key.usage(), color))
    });
    KeyMap::from_entries(entries.collect::<Result<Vec<_>, String>>()?).map_err(|e| e.to_string())
}

fn encode(settings: &Settings) -> String {
    let s = settings.saved.map(|saved| saved.state);
    let file = File {
        effect: s.map(|s| s.effect.name().to_string()),
        color: s.map(|s| format_hex(s.color)),
        speed: s.map(|s| s.speed),
        brightness: s.map(|s| s.brightness),
        pending: settings.saved.is_some_and(|saved| saved.pending),
        game_switch: settings.game_switch,
        custom: encode_map(&settings.custom),
        gaming: settings.gaming.as_ref().map(encode_map),
    };
    toml::to_string(&file).expect("settings always serialise")
}

fn decode(text: &str) -> Result<Settings, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let saved = match (&file.effect, &file.color, file.speed, file.brightness) {
        (Some(effect), Some(color), Some(speed), Some(brightness)) => {
            let state = LightingState {
                effect: effect_from_name(effect).map_err(|e| e.to_string())?,
                color: parse_hex(color).ok_or_else(|| format!("invalid colour {color:?}"))?,
                speed,
                brightness,
            };
            // Range checks against the device happen when the state is applied.
            Some(Saved { state, pending: file.pending })
        }
        (None, None, None, None) => None,
        _ => return Err("effect, color, speed and brightness must be given together".into()),
    };
    Ok(Settings {
        saved,
        custom: decode_map(&file.custom)?,
        gaming: file.gaming.as_ref().map(decode_map).transpose()?,
        game_switch: file.game_switch,
    })
}

#[cfg(test)]
mod tests {
    use g512_hidpp::effects::{EffectKind, Rgb};

    use super::*;
    use crate::state::Effect;

    fn temp_store(name: &str) -> Store {
        let dir = env::temp_dir().join(format!("g512d-store-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Store::new(dir.join("g512-control").join("state.toml"))
    }

    #[test]
    fn round_trip() {
        let store = temp_store("round-trip");
        let state = LightingState {
            effect: Effect::Zone(EffectKind::Ripple),
            color: Rgb::new(0, 0xff, 0xff),
            speed: 40,
            brightness: 75,
        };
        let settings = Settings {
            saved: Some(Saved { state, pending: true }),
            custom: KeyMap::from_entries([(0x1a, Rgb::new(0, 0xff, 0))]).unwrap(),
            gaming: None,
            game_switch: true,
        };
        store.save(&settings).unwrap();
        assert_eq!(store.load(), settings);
        let text = fs::read_to_string(store.path()).unwrap();
        assert!(text.contains("effect = \"Ripple\"") && text.contains("color = \"#00ffff\""), "{text}");
        assert!(text.contains("[custom]") && text.contains("1a = \"#00ff00\""), "{text}");
        assert!(!text.contains("[gaming]"), "an unedited Gaming map is not written: {text}");

        let gaming = KeyMap::from_entries([(0x04, Rgb::new(1, 2, 3))]).unwrap();
        let edited = Settings { gaming: Some(gaming), ..settings };
        store.save(&edited).unwrap();
        assert_eq!(store.load(), edited);
        assert!(fs::read_to_string(store.path()).unwrap().contains("[gaming]"));
        fs::remove_dir_all(store.path().parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn v1_file_loads_with_defaults() {
        let store = temp_store("v1");
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        // The user's file as written by v1.
        let v1 = "effect = \"Ripple\"\ncolor = \"#3d3846\"\nspeed = 52\nbrightness = 100\npending = false\n";
        fs::write(store.path(), v1).unwrap();
        let s = store.load();
        let state = s.saved.unwrap().state;
        assert_eq!(
            (state.effect, state.color, state.speed, state.brightness),
            (Effect::Zone(EffectKind::Ripple), Rgb::new(0x3d, 0x38, 0x46), 52, 100)
        );
        assert_eq!((&s.custom, &s.gaming, s.game_switch), (&KeyMap::default(), &None, false));
        assert_eq!(s.gaming(), KeyMap::default_gaming());
        fs::remove_dir_all(store.path().parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn maps_without_a_state() {
        let store = temp_store("maps-only");
        let custom = KeyMap::from_entries([(0xe0, Rgb::new(9, 9, 9))]).unwrap();
        let settings = Settings { custom, ..Default::default() };
        store.save(&settings).unwrap();
        assert_eq!(store.load(), settings);
        fs::remove_dir_all(store.path().parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn missing_file_is_default() {
        assert_eq!(temp_store("missing").load(), Settings::default());
    }

    #[test]
    fn corrupt_file_is_ignored() {
        let store = temp_store("corrupt");
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        for bad in [
            "not toml at all [[",
            "effect = \"Rainbow\"\ncolor = \"#ffffff\"\nspeed = 1\nbrightness = 1\n",
            "effect = \"Static\"\n",
            "[custom]\n64 = \"#ffffff\"\n",
        ] {
            fs::write(store.path(), bad).unwrap();
            assert_eq!(store.load(), Settings::default(), "{bad:?}");
        }
        fs::remove_dir_all(store.path().parent().unwrap().parent().unwrap()).unwrap();
    }
}
