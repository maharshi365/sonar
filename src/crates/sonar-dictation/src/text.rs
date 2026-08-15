//! Conservative final-transcript cleanup, adapted from Handy.

use strsim::normalized_levenshtein;

const UNIVERSAL_FILLERS: &[&str] = &[
    "uh", "uhm", "umm", "uhh", "uhhh", "ehh", "ehm", "ahm", "hmm", "hm", "mmm", "хм", "ммм",
];

fn key(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

fn punctuation(value: &str) -> (String, String) {
    let prefix = value
        .chars()
        .take_while(|character| !character.is_alphanumeric())
        .collect();
    let suffix = value
        .chars()
        .rev()
        .take_while(|character| !character.is_alphanumeric())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    (prefix, suffix)
}

fn correct_custom_words(text: &str, custom_words: &[String], threshold: f64) -> String {
    if custom_words.is_empty() || threshold <= 0.0 {
        return text.to_owned();
    }
    let words = text.split_whitespace().collect::<Vec<_>>();
    let custom = custom_words
        .iter()
        .filter_map(|word| {
            let match_key = key(word);
            (!match_key.is_empty()).then_some((word, match_key))
        })
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut index = 0;
    while index < words.len() {
        let mut best: Option<(usize, &String, f64)> = None;
        for count in 1..=3 {
            let Some(candidate_words) = words.get(index..index.saturating_add(count)) else {
                break;
            };
            let candidate = key(&candidate_words.concat());
            if candidate.is_empty() || candidate.len() > 50 {
                continue;
            }
            for (replacement, custom_key) in &custom {
                let length_difference = candidate.len().abs_diff(custom_key.len());
                let allowed = candidate
                    .len()
                    .max(custom_key.len())
                    .checked_div(4)
                    .unwrap_or_default()
                    .saturating_add(2);
                if length_difference > allowed {
                    continue;
                }
                let distance = 1.0 - normalized_levenshtein(&candidate, custom_key);
                if distance < threshold
                    && best
                        .as_ref()
                        .is_none_or(|(_, _, best_distance)| distance < *best_distance)
                {
                    best = Some((count, replacement, distance));
                }
            }
        }
        if let Some((count, replacement, _)) = best {
            let first = words.get(index).copied().unwrap_or_default();
            let last = words
                .get(index.saturating_add(count).saturating_sub(1))
                .copied()
                .unwrap_or_default();
            let (prefix, _) = punctuation(first);
            let (_, suffix) = punctuation(last);
            output.push(format!("{prefix}{replacement}{suffix}"));
            index = index.saturating_add(count);
        } else {
            output.push(words.get(index).copied().unwrap_or_default().to_owned());
            index = index.saturating_add(1);
        }
    }
    output.join(" ")
}

fn remove_fillers(text: &str, enabled: bool, custom_fillers: &[String]) -> String {
    if !enabled {
        return text.to_owned();
    }
    let configured = custom_fillers
        .iter()
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>();
    text.split_whitespace()
        .filter(|word| {
            let normalized = word
                .trim_matches(|character: char| !character.is_alphanumeric())
                .to_lowercase();
            if configured.is_empty() {
                !UNIVERSAL_FILLERS.contains(&normalized.as_str())
            } else {
                !configured.contains(&normalized)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn process(
    text: &str,
    custom_words: &[String],
    threshold: f64,
    filler_removal: bool,
    custom_fillers: &[String],
) -> String {
    let corrected = correct_custom_words(text, custom_words, threshold);
    remove_fillers(&corrected, filler_removal, custom_fillers)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::process;

    #[test]
    fn fuzzy_corrects_single_and_multiword_terms() {
        let custom = vec!["Sonar".to_owned(), "ChargeBee".to_owned()];
        assert_eq!(
            process("use soner and Charge B", &custom, 0.4, false, &[]),
            "use Sonar and ChargeBee"
        );
    }

    #[test]
    fn builtins_are_conservative_and_custom_list_overrides() {
        assert_eq!(
            process("uh um continue", &[], 0.3, true, &[]),
            "um continue"
        );
        assert_eq!(
            process("uh okay continue", &[], 0.3, true, &["okay".to_owned()]),
            "uh continue"
        );
    }

    #[test]
    fn normalizes_whitespace_even_without_filtering() {
        assert_eq!(
            process("  hello   world ", &[], 0.3, false, &[]),
            "hello world"
        );
    }
}
