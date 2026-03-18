use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::app::GifItem;

/// Result of a fuzzy match: `(original_index, matched_char_indices, score)`.
pub struct MatchResult {
    pub index: usize,
    pub matched_indices: Vec<usize>,
    pub score: i64,
}

/// Run fuzzy search against item keys. Returns results sorted by descending score.
/// An empty query returns all items in original order with no highlights.
pub fn fuzzy_search(
    matcher: &SkimMatcherV2,
    query: &str,
    items: &[GifItem],
) -> Vec<MatchResult> {
    if query.is_empty() {
        return items
            .iter()
            .enumerate()
            .map(|(i, _)| MatchResult { index: i, matched_indices: vec![], score: 0 })
            .collect();
    }

    let mut results: Vec<MatchResult> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            matcher
                .fuzzy_indices(&item.key, query)
                .map(|(score, indices)| MatchResult { index: i, matched_indices: indices, score })
        })
        .collect();

    results.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(keys: &[&str]) -> Vec<GifItem> {
        keys.iter()
            .map(|k| GifItem { key: k.to_string(), size: 0, last_modified: String::new() })
            .collect()
    }

    #[test]
    fn empty_query_returns_all_items_in_order() {
        let matcher = SkimMatcherV2::default();
        let list = items(&["cat.gif", "dog.gif", "fish.gif"]);
        let results = fuzzy_search(&matcher, "", &list);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].index, 0);
        assert_eq!(results[1].index, 1);
        assert_eq!(results[2].index, 2);
        assert!(results[0].matched_indices.is_empty());
    }

    #[test]
    fn exact_match_returns_one_result() {
        let matcher = SkimMatcherV2::default();
        let list = items(&["cat.gif", "dog.gif"]);
        let results = fuzzy_search(&matcher, "dog", &list);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].index, 1);
    }

    #[test]
    fn results_sorted_by_score_descending() {
        let matcher = SkimMatcherV2::default();
        // "cat" should score higher for "cat" than "catch"
        let list = items(&["catch.gif", "cat.gif"]);
        let results = fuzzy_search(&matcher, "cat", &list);
        assert!(!results.is_empty());
        // Scores should be non-increasing
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn matched_indices_are_valid_char_positions() {
        let matcher = SkimMatcherV2::default();
        let list = items(&["happy-cat.gif"]);
        let results = fuzzy_search(&matcher, "cat", &list);
        assert_eq!(results.len(), 1);
        let key = &list[results[0].index].key;
        for &i in &results[0].matched_indices {
            assert!(i < key.chars().count(), "index {i} out of range for '{key}'");
        }
    }

    #[test]
    fn no_match_returns_empty() {
        let matcher = SkimMatcherV2::default();
        let list = items(&["cat.gif", "dog.gif"]);
        let results = fuzzy_search(&matcher, "zzz", &list);
        assert!(results.is_empty());
    }

    #[test]
    fn fuzzy_match_non_contiguous_chars() {
        let matcher = SkimMatcherV2::default();
        let list = items(&["dancing-flamingo.gif"]);
        // 'dg' should fuzzy-match "dancing"
        let results = fuzzy_search(&matcher, "dflg", &list);
        assert_eq!(results.len(), 1);
    }
}
