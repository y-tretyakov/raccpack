//! JVM ecosystem markers: `pom.xml`, `build.gradle`, `build.gradle.kts`.

use super::types::{MarkerDef, MarkerKind};

/// JVM markers (`pom.xml`, `build.gradle`, `build.gradle.kts`).
pub static MARKERS: &[MarkerDef] = &[
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "pom.xml",
        language_hint: Some("Java"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "build.gradle",
        language_hint: Some("Java"),
    },
    MarkerDef {
        kind: MarkerKind::FileName,
        name: "build.gradle.kts",
        language_hint: Some("Kotlin"),
    },
];
