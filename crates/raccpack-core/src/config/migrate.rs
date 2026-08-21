//! Schema versioning and migration pipeline for raccpack configuration files.

use toml::Value;

use super::ConfigError;

/// The current schema version of `config.toml`.
pub const CURRENT_CONFIG_VERSION: u32 = 1;

/// Default config version for newly initialized or parsed configs.
pub fn default_config_version() -> u32 {
    CURRENT_CONFIG_VERSION
}

/// Migrate raw TOML to the current config version schema.
///
/// If `config_version` is missing or 0, runs sequential migration steps up to
/// [`CURRENT_CONFIG_VERSION`]. If `config_version` exceeds [`CURRENT_CONFIG_VERSION`],
/// returns [`ConfigError::IncompatibleVersion`].
pub fn migrate_to_current(mut raw: Value) -> Result<Value, ConfigError> {
    let version = extract_version(&raw)?;
    if version > CURRENT_CONFIG_VERSION {
        return Err(ConfigError::IncompatibleVersion {
            found: version,
            current: CURRENT_CONFIG_VERSION,
        });
    }

    let mut current_v = version;
    while current_v < CURRENT_CONFIG_VERSION {
        raw = migrate_step(current_v, raw)?;
        current_v += 1;
    }

    Ok(raw)
}

/// Extract the `config_version` field from a TOML value.
///
/// Missing or zero values are treated as version 0 (pre-versioning schema).
fn extract_version(raw: &Value) -> Result<u32, ConfigError> {
    match raw {
        Value::Table(table) => match table.get("config_version") {
            Some(Value::Integer(v)) if *v >= 0 => Ok(*v as u32),
            Some(Value::Integer(v)) => Err(ConfigError::IncompatibleVersion {
                found: *v as u32,
                current: CURRENT_CONFIG_VERSION,
            }),
            Some(_) => Err(ConfigError::IncompatibleVersion {
                found: 0,
                current: CURRENT_CONFIG_VERSION,
            }),
            None => Ok(0),
        },
        _ => Ok(0),
    }
}

/// Execute a single migration step from version `from_version` to `from_version + 1`.
fn migrate_step(from_version: u32, mut raw: Value) -> Result<Value, ConfigError> {
    match from_version {
        0 => {
            // v0 -> v1 migration: inject config_version = 1
            if let Value::Table(ref mut table) = raw {
                table.insert(
                    "config_version".to_string(),
                    Value::Integer(CURRENT_CONFIG_VERSION as i64),
                );
            }
            Ok(raw)
        }
        // Future migration steps (e.g. 1 -> 2) will be added here
        _ => Ok(raw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_missing_version_sets_current_version() {
        let raw: Value = toml::from_str("[paths]\nscan_root = '/tmp'").unwrap();
        let migrated = migrate_to_current(raw).unwrap();
        assert_eq!(
            migrated.get("config_version"),
            Some(&Value::Integer(CURRENT_CONFIG_VERSION as i64))
        );
        assert_eq!(
            migrated
                .get("paths")
                .and_then(|p| p.get("scan_root"))
                .and_then(|s| s.as_str()),
            Some("/tmp")
        );
    }

    #[test]
    fn migrate_version_0_sets_current_version() {
        let raw: Value = toml::from_str("config_version = 0\n[scanner]\nmax_depth = 4").unwrap();
        let migrated = migrate_to_current(raw).unwrap();
        assert_eq!(
            migrated.get("config_version"),
            Some(&Value::Integer(CURRENT_CONFIG_VERSION as i64))
        );
    }

    #[test]
    fn migrate_version_1_is_identity() {
        let raw: Value = toml::from_str("config_version = 1\n[scanner]\nmax_depth = 4").unwrap();
        let migrated = migrate_to_current(raw).unwrap();
        assert_eq!(migrated.get("config_version"), Some(&Value::Integer(1)));
    }

    #[test]
    fn migrate_future_version_fails_with_incompatible_version() {
        let raw: Value = toml::from_str("config_version = 99\n").unwrap();
        let err = migrate_to_current(raw).unwrap_err();
        match err {
            ConfigError::IncompatibleVersion { found, current } => {
                assert_eq!(found, 99);
                assert_eq!(current, CURRENT_CONFIG_VERSION);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
