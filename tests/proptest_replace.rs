//! Random chains with random `replace=` lines, loaded by the crate and by a model of `OpenMW`'s
//! own reading: `ConfigurationManager::readConfiguration` drops the configs before a
//! `replace=config` except the root, then `mergeComposingVariables` merges the files from the
//! last to the first, each list entry kept unless a file after its own names the list in
//! `replace=`, exactly as written. `replace` is one of those lists: a file's `replace=replace`
//! drops the `replace=` values of the files before it.

mod common;

use common::{temp_dir, write_cfg};
use openmw_config::{FileSetting, OpenMWConfiguration};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::PathBuf;

/// The lists a line adds to, by the option name `replace=` gives each.
const LISTS: [&str; 6] = [
    "content",
    "groundcover",
    "fallback-archive",
    "data",
    "fallback",
    "custom",
];

/// What a `replace=` line names: every list, lists in another case or misspelled, single
/// values, `config` and `replace`.
const REPLACED: [&str; 13] = [
    "content",
    "groundcover",
    "fallback-archive",
    "data",
    "fallback",
    "custom",
    "Content",
    "DATA",
    "fallback-archives",
    "resources",
    "encoding",
    "config",
    "replace",
];

#[derive(Debug, Clone)]
enum Line {
    /// An entry of `LISTS[list]`.
    Entry(usize),
    /// `replace=REPLACED[name]`.
    Replace(usize),
}

fn line() -> impl Strategy<Value = Line> {
    prop_oneof![
        3 => (0..LISTS.len()).prop_map(Line::Entry),
        1 => (0..REPLACED.len()).prop_map(Line::Replace),
    ]
}

fn chain_files() -> impl Strategy<Value = Vec<Vec<Line>>> {
    prop::collection::vec(prop::collection::vec(line(), 0..10), 1..5)
}

/// The value of the entry on line `index` of file `file`: unique across the chain, so no name
/// is listed twice.
fn entry_value(list: &str, file: usize, index: usize) -> String {
    match list {
        "content" => format!("F{file}L{index}.esp"),
        "groundcover" => format!("G{file}L{index}.esp"),
        "fallback-archive" => format!("A{file}L{index}.bsa"),
        "data" => format!("/data/f{file}l{index}"),
        "fallback" => format!("iF{file}L{index},{index}"),
        _ => format!("f{file}l{index}"),
    }
}

/// How `lists` reads an entry back: `fallback=` entries by key, the rest by value.
fn read_back(list: &str, value: String) -> String {
    if list == "fallback" {
        value.split(',').next().unwrap_or_default().to_owned()
    } else {
        value
    }
}

/// A chain of these files, root first, each ending in a `config=` line naming the next.
fn write_chain(files: &[Vec<Line>]) -> Vec<PathBuf> {
    let dirs: Vec<PathBuf> = files.iter().map(|_| temp_dir("prop_replace")).collect();
    for (file, lines) in files.iter().enumerate() {
        let mut text = String::new();
        for (index, line) in lines.iter().enumerate() {
            match line {
                Line::Entry(list) => {
                    let value = entry_value(LISTS[*list], file, index);
                    writeln!(text, "{}={value}", LISTS[*list]).unwrap();
                }
                Line::Replace(name) => writeln!(text, "replace={}", REPLACED[*name]).unwrap(),
            }
        }
        if let Some(next) = dirs.get(file + 1) {
            writeln!(text, "config={}", next.display()).unwrap();
        }
        write_cfg(&dirs[file], &text);
    }
    dirs
}

/// Each list as `OpenMW`'s code reads the chain, `fallback=` keys sorted.
fn model(files: &[Vec<Line>]) -> Vec<Vec<String>> {
    let replace_lines = |file: usize| {
        files[file].iter().filter_map(|line| match line {
            Line::Replace(name) => Some(REPLACED[*name]),
            Line::Entry(_) => None,
        })
    };

    // readConfiguration: replace=config after the root drops the configs before it but the
    // root.
    let mut loaded = vec![0];
    for file in 1..files.len() {
        if loaded.len() > 1 && replace_lines(file).any(|name| name == "config") {
            loaded.truncate(1);
        }
        loaded.push(file);
    }

    // mergeComposingVariables, from the last file to the first. The replace= values are a list
    // too: once a file above says replace=replace, a file's own replace= values are dropped.
    let mut replaced: HashSet<&str> = HashSet::new();
    let mut kept_per_file = Vec::new();
    for &file in loaded.iter().rev() {
        let kept: Vec<(usize, String)> = files[file]
            .iter()
            .enumerate()
            .filter_map(|(index, line)| match line {
                Line::Entry(list) if !replaced.contains(LISTS[*list]) => Some((
                    *list,
                    read_back(LISTS[*list], entry_value(LISTS[*list], file, index)),
                )),
                _ => None,
            })
            .collect();
        kept_per_file.push(kept);
        if !replaced.contains("replace") {
            replaced.extend(replace_lines(file));
        }
    }

    let mut lists = vec![Vec::new(); LISTS.len()];
    for kept in kept_per_file.into_iter().rev() {
        for (list, value) in kept {
            lists[list].push(value);
        }
    }
    lists[4].sort();
    lists
}

/// Each list as the crate holds it, `fallback=` keys sorted: `game_settings()` yields the
/// latest first.
fn lists(config: &OpenMWConfiguration) -> Vec<Vec<String>> {
    let names = |files: Vec<&FileSetting>| -> Vec<String> {
        files.into_iter().map(|file| file.value().clone()).collect()
    };
    let mut fallback: Vec<String> = config
        .game_settings()
        .map(|setting| setting.key().clone())
        .collect();
    fallback.sort();
    vec![
        names(config.content_files_iter().collect()),
        names(config.groundcover_iter().collect()),
        names(config.fallback_archives_iter().collect()),
        config
            .data_directories_iter()
            .map(|dir| dir.original().clone())
            .collect(),
        fallback,
        config
            .generic_settings_iter()
            .filter(|setting| setting.key() == "custom")
            .map(|setting| setting.value().to_owned())
            .collect(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn prop_a_chain_loads_as_openmw_merges_it(files in chain_files()) {
        let dirs = write_chain(&files);
        let config = OpenMWConfiguration::new(Some(dirs[0].clone())).unwrap();

        prop_assert_eq!(lists(&config), model(&files));
        prop_assert_eq!(config.user_config_path(), dirs[dirs.len() - 1].clone());
    }

    #[test]
    fn prop_save_user_writes_what_reloads_as_it_was(files in chain_files()) {
        let dirs = write_chain(&files);
        let mut config = OpenMWConfiguration::new(Some(dirs[0].clone())).unwrap();

        // Take out the first plugin, which may be a parent's, replace a list, and add a plugin.
        let first = config.content_files_iter().next().map(|file| file.value().clone());
        if let Some(first) = first {
            config.remove_content_file(&first);
        }
        config.set_generic_settings("custom", Some(vec!["user".to_owned()]));
        config.add_content_file("Added.esp").unwrap();
        config.save_user().unwrap();

        let reloaded = OpenMWConfiguration::new(Some(dirs[0].clone())).unwrap();
        prop_assert_eq!(lists(&reloaded), lists(&config));
    }
}
