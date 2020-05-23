use std::path::PathBuf;
use std::convert::{From, TryFrom};

use symspell;
use symspell::{AsciiStringStrategy, SymSpell, Verbosity};
use crate::prelude::*;

/// One word fuzzier e.g. name city country
pub struct FuzzyWord {
    corpus: Option<Vec<FuzzyConfig>>,
    engine: SymSpell<AsciiStringStrategy>,
}

impl FuzzyWord {
    pub fn new(corpus: Option<Vec<FuzzyConfig>>) -> Self {
        let mut engine: SymSpell<AsciiStringStrategy> = SymSpell::default();
        if let None = corpus {
            return Self {
                corpus,
                engine,
            };
        }

        let corpus = corpus.unwrap();
        if corpus.is_empty() {
            let corpus: Option<Vec<FuzzyConfig>> = None;
            return Self {
                corpus,
                engine,
            };
        }

        for config in &corpus {
            engine.load_dictionary(
                &config.corpus.as_path().to_string_lossy().to_string(),
                config.term_index,
                config.count_index,
                &config.separator,
            );
        };
        let corpus = Some(corpus);
        Self {
            corpus,
            engine,
        }
    }
    pub fn corpus(&self) -> Option<&Vec<FuzzyConfig>> {
        self.corpus.as_ref()
    }
    pub fn lookup(&self, correct: &str) -> Option<Vec<String>> {
        let suggestions = self.engine.lookup(correct, Verbosity::Top, 2);
        if suggestions.is_empty() {
            return None;
        };
        let mut result = Vec::<String>::with_capacity(suggestions.len());
        for suggestion in suggestions {
            result.push(suggestion.term);
        };
        Some(result)
    }
}

/// By default bootstrap from default config
impl Default for FuzzyWord {
    fn default() -> Self {
        let config = FuzzyConfig::default();
        let corpus = Some(vec![config]);
        FuzzyWord::new(corpus)
    }
}

/// Try to bootstrap from dir or path
impl TryFrom<&str> for FuzzyWord {
    type Error = IndexError;
    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let corpus = PathBuf::from(path);
        let paths = if corpus.is_dir() {
            ls(path)?
        } else {
            vec![PathBuf::from(path)]
        };
        let mut config = Vec::<FuzzyConfig>::with_capacity(paths.len());
        for path in paths {
            config.push(FuzzyConfig::from(path));
        };
        let corpus = Some(config);
        Ok(Self::new(
            corpus,
        ))
    }
}


/// Load multiple dictionary entries from a file of word/frequency count pairs.
/// This goes for bootstrapping symspell
/// * `corpus` - file paths.
/// * `term_index` - The column position of the word.
/// * `count_index` - The column position of the frequency count.
/// * `separator` - Separator between word and frequency
#[derive(Clone, Debug, PartialEq)]
pub struct FuzzyConfig {
    corpus: PathBuf,
    term_index: i64,
    count_index: i64,
    separator: String,
}

/// Creates an instance
impl FuzzyConfig {
    pub fn new(corpus: PathBuf, term_index: i64, count_index: i64, separator: String) -> Self {
        Self {
            corpus,
            term_index,
            count_index,
            separator,
        }
    }
}

/// By default load names from corpus directory
impl Default for FuzzyConfig {
    fn default() -> Self {
        let path = "corpus/frequency_names.txt";
        FuzzyConfig::from(path)
    }
}

impl From<&str> for FuzzyConfig {
    fn from(path: &str) -> Self {
        let corpus = PathBuf::from(path);
        FuzzyConfig::from(corpus)
    }
}

impl From<PathBuf> for FuzzyConfig {
    fn from(corpus: PathBuf) -> Self {
        let term_index = 0i64;
        let count_index = 1i64;
        let separator = " ".to_string();
        Self::new(
            corpus,
            term_index,
            count_index,
            separator,
        )
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_default_fuzzy_config() {
        let corpus = PathBuf::from("corpus/frequency_names.txt");
        let term_index = 0i64;
        let count_index = 1i64;
        let separator = " ".to_string();
        let expected = FuzzyConfig::new(
            corpus,
            term_index,
            count_index,
            separator,
        );
        let computed = FuzzyConfig::default();
        assert_eq!(computed, expected);
    }

    #[test]
    fn validate_default_fuzzy() {
        let expected = Some(vec![FuzzyConfig::default()]);
        let computed = FuzzyWord::default();
        assert_eq!(&expected.as_ref(), &computed.corpus());
    }

    #[test]
    fn validate_default_fuzzy_word() {
        let fuzz = FuzzyWord::default();
        let suggestions = fuzz.lookup("surav");
        assert!(suggestions.is_some());
        let suggestions = suggestions.unwrap();
        assert_eq!(suggestions, vec!["saurav".to_string()]);
    }
}
