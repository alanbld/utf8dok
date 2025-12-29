//! Writing Quality Plugin (Phase 17)
//!
//! Provides Grammarly-like features for documentation:
//! - Passive voice detection
//! - Weasel words detection
//! - Readability analysis

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use crate::config::Settings;

/// Writing quality analyzer
#[derive(Debug, Clone)]
pub struct QualityPlugin {
    /// Whether the plugin is enabled
    enabled: bool,
    /// Weasel words to detect
    weasel_words: Vec<String>,
}

impl QualityPlugin {
    /// Create a new quality plugin with default settings
    pub fn new() -> Self {
        Self::with_settings(&Settings::default())
    }

    /// Create a quality plugin configured from settings
    pub fn with_settings(settings: &Settings) -> Self {
        Self {
            enabled: settings.plugins.writing_quality,
            weasel_words: if settings.plugins.custom_weasel_words.is_empty() {
                default_weasel_words()
            } else {
                settings.plugins.custom_weasel_words.clone()
            },
        }
    }

    /// Validate writing quality in text
    pub fn validate_writing_quality(&self, text: &str) -> Vec<Diagnostic> {
        if !self.enabled {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        for (line_num, line) in text.lines().enumerate() {
            // Check for passive voice
            diagnostics.extend(self.check_passive_voice(line, line_num as u32));

            // Check for weasel words
            diagnostics.extend(self.check_weasel_words(line, line_num as u32));
        }

        diagnostics
    }

    /// Check for passive voice patterns
    fn check_passive_voice(&self, line: &str, line_num: u32) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let line_lower = line.to_lowercase();

        // Passive voice patterns: "was/were/is/are/has been/have been + verb-ed"
        let patterns = [
            ("was ", "ed"),
            ("were ", "ed"),
            ("is ", "ed"),
            ("are ", "ed"),
            ("been ", "ed"),
            ("be ", "ed"),
        ];

        for (prefix, suffix) in patterns {
            let mut search_start = 0;
            while let Some(pos) = line_lower[search_start..].find(prefix) {
                let abs_pos = search_start + pos;
                let after_prefix = abs_pos + prefix.len();

                // Look for a word ending in "ed" after the prefix
                if let Some(word_end) = line_lower[after_prefix..]
                    .find(|c: char| c.is_whitespace() || c == '.' || c == ',')
                {
                    let word = &line_lower[after_prefix..after_prefix + word_end];
                    if word.ends_with(suffix) && word.len() > 3 {
                        let match_start = abs_pos;
                        let match_end = after_prefix + word_end;

                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: line_num,
                                    character: match_start as u32,
                                },
                                end: Position {
                                    line: line_num,
                                    character: match_end as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::INFORMATION),
                            code: Some(tower_lsp::lsp_types::NumberOrString::String(
                                "QUALITY001".to_string(),
                            )),
                            source: Some("writing-quality".to_string()),
                            message: format!(
                                "Passive voice detected: '{}'. Consider using active voice.",
                                &line[match_start..match_end]
                            ),
                            ..Default::default()
                        });
                    }
                }
                search_start = abs_pos + 1;
            }
        }

        diagnostics
    }

    /// Check for weasel words
    fn check_weasel_words(&self, line: &str, line_num: u32) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let line_lower = line.to_lowercase();

        for weasel_word in &self.weasel_words {
            let weasel_lower = weasel_word.to_lowercase();
            let mut search_start = 0;

            while let Some(pos) = line_lower[search_start..].find(&weasel_lower) {
                let abs_pos = search_start + pos;
                let end_pos = abs_pos + weasel_word.len();

                // Check if it's a whole word (not part of another word)
                let is_start_boundary = abs_pos == 0
                    || !line_lower
                        .chars()
                        .nth(abs_pos - 1)
                        .unwrap_or(' ')
                        .is_alphanumeric();
                let is_end_boundary = end_pos >= line_lower.len()
                    || !line_lower
                        .chars()
                        .nth(end_pos)
                        .unwrap_or(' ')
                        .is_alphanumeric();

                if is_start_boundary && is_end_boundary {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_num,
                                character: abs_pos as u32,
                            },
                            end: Position {
                                line: line_num,
                                character: end_pos as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::INFORMATION),
                        code: Some(tower_lsp::lsp_types::NumberOrString::String(
                            "QUALITY002".to_string(),
                        )),
                        source: Some("writing-quality".to_string()),
                        message: format!(
                            "Weasel word detected: '{}'. Consider removing for stronger writing.",
                            weasel_word
                        ),
                        ..Default::default()
                    });
                }

                search_start = abs_pos + 1;
            }
        }

        diagnostics
    }

    /// Calculate readability score (simplified Flesch-Kincaid)
    pub fn calculate_readability(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 100.0;
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        let sentences = text
            .chars()
            .filter(|c| *c == '.' || *c == '!' || *c == '?')
            .count()
            .max(1);
        let syllables = estimate_syllables(text);

        if words.is_empty() {
            return 100.0;
        }

        let words_per_sentence = words.len() as f32 / sentences as f32;
        let syllables_per_word = syllables as f32 / words.len() as f32;

        // Flesch Reading Ease formula
        (206.835 - (1.015 * words_per_sentence) - (84.6 * syllables_per_word)).clamp(0.0, 100.0)
    }

    /// Get a quality summary for a document
    pub fn quality_summary(&self, text: &str) -> QualitySummary {
        let diagnostics = self.validate_writing_quality(text);
        let readability = self.calculate_readability(text);

        QualitySummary {
            total_issues: diagnostics.len(),
            passive_voice_count: diagnostics
                .iter()
                .filter(|d| {
                    d.code.as_ref().is_some_and(|c| match c {
                        tower_lsp::lsp_types::NumberOrString::String(s) => s.contains("QUALITY001"),
                        tower_lsp::lsp_types::NumberOrString::Number(_) => false,
                    })
                })
                .count(),
            weasel_words_count: diagnostics
                .iter()
                .filter(|d| {
                    d.code.as_ref().is_some_and(|c| match c {
                        tower_lsp::lsp_types::NumberOrString::String(s) => s.contains("QUALITY002"),
                        tower_lsp::lsp_types::NumberOrString::Number(_) => false,
                    })
                })
                .count(),
            readability_score: readability,
            grade_level: readability_to_grade(readability),
        }
    }
}

impl Default for QualityPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Quality analysis summary
#[derive(Debug, Clone)]
pub struct QualitySummary {
    pub total_issues: usize,
    pub passive_voice_count: usize,
    pub weasel_words_count: usize,
    pub readability_score: f32,
    pub grade_level: String,
}

/// Estimate syllables in text (rough approximation)
fn estimate_syllables(text: &str) -> usize {
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
    let mut syllable_count = 0;

    for word in text.split_whitespace() {
        let word_lower = word.to_lowercase();
        let chars: Vec<char> = word_lower.chars().collect();
        let mut prev_was_vowel = false;
        let mut word_syllables = 0;

        for ch in &chars {
            let is_vowel = vowels.contains(ch);
            if is_vowel && !prev_was_vowel {
                word_syllables += 1;
            }
            prev_was_vowel = is_vowel;
        }

        // Every word has at least one syllable
        syllable_count += word_syllables.max(1);
    }

    syllable_count
}

/// Convert readability score to grade level
fn readability_to_grade(score: f32) -> String {
    match score {
        s if s >= 90.0 => "5th grade".to_string(),
        s if s >= 80.0 => "6th grade".to_string(),
        s if s >= 70.0 => "7th grade".to_string(),
        s if s >= 60.0 => "8th-9th grade".to_string(),
        s if s >= 50.0 => "10th-12th grade".to_string(),
        s if s >= 30.0 => "College".to_string(),
        _ => "College graduate".to_string(),
    }
}

/// Default weasel words list
fn default_weasel_words() -> Vec<String> {
    vec![
        "clearly".to_string(),
        "obviously".to_string(),
        "basically".to_string(),
        "simply".to_string(),
        "just".to_string(),
        "actually".to_string(),
        "really".to_string(),
        "very".to_string(),
        "quite".to_string(),
        "perhaps".to_string(),
        "maybe".to_string(),
        "possibly".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passive_voice_detection() {
        let plugin = QualityPlugin::new();
        let text = "The error was caused by the system. The file was deleted by the user.";

        let diagnostics = plugin.validate_writing_quality(text);

        assert!(!diagnostics.is_empty(), "Should detect passive voice");
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("Passive voice")));
    }

    #[test]
    fn test_weasel_words_detection() {
        let plugin = QualityPlugin::new();
        let text = "Clearly, this is the best solution. Obviously, everyone agrees.";

        let diagnostics = plugin.validate_writing_quality(text);

        assert!(!diagnostics.is_empty(), "Should detect weasel words");
        assert!(diagnostics
            .iter()
            .any(|d| d.message.to_lowercase().contains("clearly")));
        assert!(diagnostics
            .iter()
            .any(|d| d.message.to_lowercase().contains("obviously")));
    }

    #[test]
    fn test_plugin_disabled() {
        let mut settings = Settings::default();
        settings.plugins.writing_quality = false;

        let plugin = QualityPlugin::with_settings(&settings);
        let diagnostics = plugin.validate_writing_quality("The file was deleted.");

        assert!(diagnostics.is_empty(), "Should respect disabled plugin");
    }

    #[test]
    fn test_custom_weasel_words() {
        let mut settings = Settings::default();
        settings.plugins.custom_weasel_words = vec!["actually".to_string(), "simply".to_string()];

        let plugin = QualityPlugin::with_settings(&settings);
        let diagnostics = plugin.validate_writing_quality("Actually, it's simply perfect.");

        assert_eq!(diagnostics.len(), 2, "Should detect custom weasel words");
    }

    #[test]
    fn test_readability_calculation() {
        let plugin = QualityPlugin::new();

        // Simple text should have high readability
        let simple = "The cat sat. The dog ran. The bird flew.";
        let simple_score = plugin.calculate_readability(simple);
        assert!(simple_score > 70.0, "Simple text should be readable");

        // Complex text should have lower readability
        let complex = "The implementation of sophisticated algorithms necessitates comprehensive understanding of computational complexity theory and mathematical foundations.";
        let complex_score = plugin.calculate_readability(complex);
        assert!(
            complex_score < simple_score,
            "Complex text should be less readable"
        );
    }

    #[test]
    fn test_quality_summary() {
        let plugin = QualityPlugin::new();
        let text = "Clearly, the file was deleted. Obviously, this is very important.";

        let summary = plugin.quality_summary(text);

        assert!(summary.total_issues > 0);
        assert!(summary.passive_voice_count > 0);
        assert!(summary.weasel_words_count > 0);
        assert!(summary.readability_score >= 0.0);
        assert!(!summary.grade_level.is_empty());
    }

    #[test]
    fn test_information_severity() {
        let plugin = QualityPlugin::new();
        let text = "Obviously, the decision was made by the team.";

        let diagnostics = plugin.validate_writing_quality(text);

        // All writing quality issues should be INFORMATION severity
        for diag in &diagnostics {
            assert_eq!(
                diag.severity,
                Some(DiagnosticSeverity::INFORMATION),
                "Quality issues should be informational"
            );
        }
    }
}
