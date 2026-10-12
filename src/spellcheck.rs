use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use zspell::Dictionary;

static DICTIONARY: OnceLock<Option<Dictionary>> = OnceLock::new();

fn dictionary() -> Option<&'static Dictionary> {
    DICTIONARY.get_or_init(load_system_dictionary).as_ref()
}

fn load_system_dictionary() -> Option<Dictionary> {
    let locale = sys_locale::get_locale().unwrap_or_default();
    let mut locales = Vec::new();
    if !locale.is_empty() {
        let locale = locale
            .split(['.', '@'])
            .next()
            .unwrap_or(&locale)
            .replace('-', "_");
        locales.push(locale.clone());
        if let Some(language) = locale.split('_').next() {
            locales.push(language.to_owned());
        }
    }
    for fallback in ["en_AU", "en_US", "en_GB"] {
        if !locales.iter().any(|locale| locale == fallback) {
            locales.push(fallback.to_owned());
        }
    }

    for (affix, dictionary) in dictionary_paths(&locales) {
        let (Ok(affix), Ok(dictionary)) = (
            std::fs::read_to_string(&affix),
            std::fs::read_to_string(&dictionary),
        ) else {
            continue;
        };
        match zspell::builder()
            .config_str(&affix)
            .dict_str(&dictionary)
            .build()
        {
            Ok(dictionary) => return Some(dictionary),
            Err(error) => tracing::warn!(%error, "could not load system spelling dictionary"),
        }
    }

    let result = zspell::builder()
        .config_str(include_str!("../assets/spellcheck/en_US.aff"))
        .dict_str(include_str!("../assets/spellcheck/en_US.dic"))
        .build();
    match result {
        Ok(dictionary) => Some(dictionary),
        Err(error) => {
            tracing::warn!(%error, "could not load bundled spelling dictionary");
            None
        }
    }
}

fn dictionary_paths(locales: &[String]) -> Vec<(PathBuf, PathBuf)> {
    let mut directories = vec![
        PathBuf::from("/usr/share/hunspell"),
        PathBuf::from("/usr/share/myspell/dicts"),
        PathBuf::from("/usr/local/share/hunspell"),
        PathBuf::from("/opt/homebrew/share/hunspell"),
        PathBuf::from("/Library/Spelling"),
        PathBuf::from("/System/Library/Spelling"),
    ];
    if let Some(path) = std::env::var_os("DICPATH") {
        directories.extend(std::env::split_paths(&path));
    }
    locales
        .iter()
        .flat_map(|locale| {
            directories.iter().flat_map(move |directory| {
                [locale.clone(), locale.to_lowercase()].map(|name| {
                    (
                        dictionary_file(directory, &name, "aff"),
                        dictionary_file(directory, &name, "dic"),
                    )
                })
            })
        })
        .collect()
}

fn dictionary_file(directory: &Path, locale: &str, extension: &str) -> PathBuf {
    directory.join(format!("{locale}.{extension}"))
}

pub(super) fn misspellings(text: &str) -> Vec<Range<usize>> {
    let Some(dictionary) = dictionary() else {
        return Vec::new();
    };
    dictionary
        .check_indices(text)
        .map(|(start, word)| start..start + word.len())
        .collect()
}

pub(super) fn suggestions(word: &str) -> Vec<String> {
    dictionary().map_or_else(Vec::new, |dictionary| {
        dictionary
            .entry(word)
            .suggest()
            .unwrap_or_default()
            .into_iter()
            .take(5)
            .map(str::to_owned)
            .collect()
    })
}
