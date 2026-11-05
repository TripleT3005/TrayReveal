use std::fs;
use std::path::Path;

// Constants
const VERSION: &str = env!("CARGO_PKG_VERSION");
const LANGUAGES: [&str; 2] = ["### English", "### Deutsch"];
const UNRELEASED: &str = "Unreleased";

// Sections
fn section(changelog: &str, version: &str) -> Option<String> {
    let heading = format!("## [{version}]");
    let start = changelog
        .lines()
        .position(|line| line.starts_with(&heading))?;
    Some(
        changelog
            .lines()
            .skip(start + 1)
            .take_while(|line| !line.starts_with("## ["))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn headings(changelog: &str) -> Vec<&str> {
    changelog
        .lines()
        .filter_map(|line| line.strip_prefix("## ["))
        .filter_map(|rest| rest.split(']').next())
        .collect()
}

fn read_changelog() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("CHANGELOG.md"))
        .expect("CHANGELOG.md exists")
}

// Tests
#[test]
fn changelog_starts_with_unreleased_then_current_version() {
    let changelog = read_changelog();
    let headings = headings(&changelog);
    assert_eq!(headings.first(), Some(&UNRELEASED));
    assert_eq!(headings.get(1), Some(&VERSION));
}

#[test]
fn changelog_describes_the_current_version() {
    let changelog = read_changelog();
    let section = section(&changelog, VERSION)
        .unwrap_or_else(|| panic!("CHANGELOG.md has no section for version {VERSION}"));
    for language in LANGUAGES {
        assert!(
            section.contains(language),
            "the section for {VERSION} in CHANGELOG.md has no {language:?} part"
        );
    }
    assert!(
        section.lines().any(|line| line.starts_with("- ")),
        "the section for {VERSION} in CHANGELOG.md lists no changes"
    );
}

#[test]
fn finds_only_the_requested_section() {
    let changelog =
        "## [2.0.0] – x\n\n### English\n- New\n\n## [1.0.0] – y\n\n### English\n- Old\n";
    let found = section(changelog, "2.0.0").unwrap();
    assert!(found.contains("New"));
    assert!(!found.contains("Old"));
    assert!(section(changelog, "3.0.0").is_none());
}
