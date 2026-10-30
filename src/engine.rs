use std::collections::BTreeSet;

// Types
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    Off,
    #[default]
    NewIcons,
    AllIcons,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub key: String,
    pub promoted: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub id: String,
    pub promoted: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub changes: Vec<Change>,
    pub revealed: Vec<String>,
    pub known: BTreeSet<String>,
}

// Synchronization
pub fn plan(
    entries: &[Entry],
    known: Option<&BTreeSet<String>>,
    mode: Mode,
    exceptions: &BTreeSet<String>,
) -> Plan {
    let mut plan = Plan {
        known: entries.iter().map(|entry| entry.id.clone()).collect(),
        ..Plan::default()
    };
    if entries.is_empty() {
        plan.known = known.cloned().unwrap_or_default();
        return plan;
    }
    let is_new = |entry: &Entry| known.is_some_and(|known| !known.contains(&entry.id));
    for entry in entries {
        let excepted = exceptions.contains(&entry.key);
        let target = match mode {
            Mode::AllIcons => Some(!excepted),
            Mode::NewIcons if is_new(entry) && !excepted => {
                Some(predecessor_state(entries, entry, known).unwrap_or(true))
            }
            Mode::Off | Mode::NewIcons => None,
        };
        let Some(target) = target else {
            continue;
        };
        if is_visible(entry) != target {
            plan.changes.push(Change {
                id: entry.id.clone(),
                promoted: target,
            });
        }
        if target && is_new(entry) && predecessor_state(entries, entry, known).is_none() {
            plan.revealed.push(entry.id.clone());
        }
    }
    plan
}

pub fn set_all(entries: &[Entry], exceptions: &BTreeSet<String>, promoted: bool) -> Vec<Change> {
    entries
        .iter()
        .filter_map(|entry| {
            let target = promoted && !exceptions.contains(&entry.key);
            (is_visible(entry) != target).then(|| Change {
                id: entry.id.clone(),
                promoted: target,
            })
        })
        .collect()
}

pub fn hide_key(entries: &[Entry], key: &str) -> Vec<Change> {
    entries
        .iter()
        .filter(|entry| entry.key == key && is_visible(entry))
        .map(|entry| Change {
            id: entry.id.clone(),
            promoted: false,
        })
        .collect()
}

fn predecessor_state(
    entries: &[Entry],
    entry: &Entry,
    known: Option<&BTreeSet<String>>,
) -> Option<bool> {
    let known = known?;
    let mut states = entries
        .iter()
        .filter(|other| other.id != entry.id && other.key == entry.key && known.contains(&other.id))
        .filter_map(|other| other.promoted);
    let first = states.next()?;
    Some(states.fold(first, |visible, state| visible || state))
}

fn is_visible(entry: &Entry) -> bool {
    entry.promoted == Some(true)
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, key: &str, promoted: Option<bool>) -> Entry {
        Entry {
            id: id.into(),
            key: key.into(),
            promoted,
        }
    }

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|&id| id.to_owned()).collect()
    }

    fn change(id: &str, promoted: bool) -> Change {
        Change {
            id: id.into(),
            promoted,
        }
    }

    #[test]
    fn first_run_only_remembers_the_icons() {
        let entries = [entry("1", "a", None), entry("2", "b", Some(false))];
        let plan = plan(&entries, None, Mode::NewIcons, &BTreeSet::new());
        assert_eq!(plan.changes, Vec::<Change>::new());
        assert_eq!(plan.revealed, Vec::<String>::new());
        assert_eq!(plan.known, set(&["1", "2"]));
    }

    #[test]
    fn shows_new_icons_once() {
        let entries = [entry("1", "a", None), entry("2", "b", None)];
        let known = set(&["1"]);
        let plan = plan(&entries, Some(&known), Mode::NewIcons, &BTreeSet::new());
        assert_eq!(plan.changes, [change("2", true)]);
        assert_eq!(plan.revealed, ["2"]);
        assert_eq!(plan.known, set(&["1", "2"]));
    }

    #[test]
    fn respects_later_changes_to_known_icons() {
        let entries = [entry("1", "a", Some(false))];
        let known = set(&["1"]);
        let plan = plan(&entries, Some(&known), Mode::NewIcons, &BTreeSet::new());
        assert_eq!(plan.changes, Vec::<Change>::new());
    }

    #[test]
    fn skips_new_icons_on_the_exception_list() {
        let entries = [entry("1", "a", None)];
        let plan = plan(
            &entries,
            Some(&BTreeSet::new()),
            Mode::NewIcons,
            &set(&["a"]),
        );
        assert_eq!(plan.changes, Vec::<Change>::new());
        assert_eq!(plan.revealed, Vec::<String>::new());
        assert_eq!(plan.known, set(&["1"]));
    }

    #[test]
    fn keeps_a_hidden_app_hidden_after_an_update() {
        let entries = [entry("old", "a", Some(false)), entry("new", "a", None)];
        let known = set(&["old"]);
        let plan = plan(&entries, Some(&known), Mode::NewIcons, &BTreeSet::new());
        assert_eq!(plan.changes, Vec::<Change>::new());
        assert_eq!(plan.revealed, Vec::<String>::new());
    }

    #[test]
    fn shows_an_updated_app_quietly() {
        let entries = [entry("old", "a", Some(true)), entry("new", "a", None)];
        let known = set(&["old"]);
        let plan = plan(&entries, Some(&known), Mode::NewIcons, &BTreeSet::new());
        assert_eq!(plan.changes, [change("new", true)]);
        assert_eq!(plan.revealed, Vec::<String>::new());
    }

    #[test]
    fn ignores_untouched_predecessors() {
        let entries = [entry("old", "a", None), entry("new", "a", None)];
        let known = set(&["old"]);
        let plan = plan(&entries, Some(&known), Mode::NewIcons, &BTreeSet::new());
        assert_eq!(plan.changes, [change("new", true)]);
        assert_eq!(plan.revealed, ["new"]);
    }

    #[test]
    fn shows_all_icons_except_the_exceptions() {
        let entries = [
            entry("1", "a", Some(false)),
            entry("2", "b", Some(true)),
            entry("3", "c", None),
            entry("4", "d", Some(true)),
        ];
        let known = set(&["1", "2", "4"]);
        let plan = plan(&entries, Some(&known), Mode::AllIcons, &set(&["d"]));
        assert_eq!(
            plan.changes,
            [change("1", true), change("3", true), change("4", false)]
        );
        assert_eq!(plan.revealed, ["3"]);
    }

    #[test]
    fn changes_nothing_when_off() {
        let entries = [entry("1", "a", None), entry("2", "b", Some(false))];
        let known = set(&["1"]);
        let plan = plan(&entries, Some(&known), Mode::Off, &BTreeSet::new());
        assert_eq!(plan.changes, Vec::<Change>::new());
        assert_eq!(plan.known, set(&["1", "2"]));
    }

    #[test]
    fn keeps_the_known_icons_when_the_list_is_empty() {
        let known = set(&["1", "2"]);
        let plan = plan(&[], Some(&known), Mode::AllIcons, &BTreeSet::new());
        assert_eq!(plan.known, known);
    }

    #[test]
    fn shows_and_hides_everything() {
        let entries = [
            entry("1", "a", None),
            entry("2", "b", Some(true)),
            entry("3", "c", Some(true)),
        ];
        assert_eq!(
            set_all(&entries, &set(&["c"]), true),
            [change("1", true), change("3", false)]
        );
        assert_eq!(
            set_all(&entries, &BTreeSet::new(), false),
            [change("2", false), change("3", false)]
        );
    }

    #[test]
    fn hides_every_entry_of_an_app() {
        let entries = [
            entry("1", "a", Some(true)),
            entry("2", "a", None),
            entry("3", "b", Some(true)),
        ];
        assert_eq!(hide_key(&entries, "a"), [change("1", false)]);
    }
}
