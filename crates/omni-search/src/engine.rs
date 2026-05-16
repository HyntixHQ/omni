use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use omni_core::models::{AppEntry, SearchResult};

pub struct SearchEngine {
    matcher: SkimMatcherV2,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn search(&self, query: &str, apps: &[AppEntry]) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = apps
            .iter()
            .filter_map(|entry| self.score_entry(query, entry))
            .collect();

        results.sort_by_key(|b| std::cmp::Reverse(b.score));
        results.truncate(20);
        results
    }

    fn score_entry(&self, query: &str, entry: &AppEntry) -> Option<SearchResult> {
        let text = entry.searchable_text();
        let lower_text = text.to_lowercase();
        let lower_query = query.to_lowercase();

        // Strategy 1: Exact match on name (highest priority)
        if entry.name.to_lowercase() == lower_query {
            return Some(SearchResult {
                score: i64::MAX,
                matches: Vec::new(),
                entry: entry.clone(),
            });
        }

        // Strategy 2: Strong fuzzy on combined searchable text
        if let Some((score, matches)) = self.matcher.fuzzy_indices(&text, query) {
            // Bonus for matches in the name specifically
            let name_bonus = if self
                .matcher
                .fuzzy_match(&entry.name, query)
                .is_some()
            {
                500
            } else {
                0
            };
            return Some(SearchResult {
                score: score + 1000 + name_bonus,
                matches,
                entry: entry.clone(),
            });
        }

        // Strategy 3: Substring match in combined text
        if let Some(pos) = lower_text.find(&lower_query) {
            let matched_indices: Vec<usize> =
                (pos..pos + lower_query.len()).collect();
            let score = 500 + (lower_query.len() * 10) as i64;
            return Some(SearchResult {
                score,
                matches: matched_indices,
                entry: entry.clone(),
            });
        }

        // Strategy 4: Word prefix (query matches start of any word)
        if let Some(score) = self.word_prefix_score(&lower_query, &lower_text, &entry.name) {
            return Some(SearchResult {
                score,
                matches: Vec::new(),
                entry: entry.clone(),
            });
        }

        // Strategy 5: Relaxed fuzzy on combined text
        if let Some((score, matches)) = self.relaxed_fuzzy(&lower_query, &lower_text) {
            return Some(SearchResult {
                score,
                matches,
                entry: entry.clone(),
            });
        }

        None
    }

    fn word_prefix_score(&self, query: &str, lower_text: &str, name: &str) -> Option<i64> {
        let q_len = query.len();
        if q_len < 2 {
            return None;
        }

        for word in lower_text.split_whitespace() {
            if word.starts_with(query) {
                let name_bonus = if name.to_lowercase().contains(query) {
                    100
                } else {
                    0
                };
                let is_in_name = lower_text[..name.len()].contains(query);
                let position_bonus = if is_in_name { 50 } else { 0 };
                let score = 400 + (q_len * 20) as i64 + name_bonus + position_bonus;
                return Some(score);
            }
        }
        None
    }

    fn relaxed_fuzzy(&self, query: &str, lower_text: &str) -> Option<(i64, Vec<usize>)> {
        let q_chars: Vec<char> = query.chars().collect();
        let n_chars: Vec<char> = lower_text.chars().collect();
        let total = q_chars.len();

        if total < 3 {
            return None;
        }

        let mut matched_indices = Vec::new();
        let mut ni = 0;

        for &qc in &q_chars {
            while ni < n_chars.len() && n_chars[ni] != qc {
                ni += 1;
            }
            if ni < n_chars.len() {
                matched_indices.push(ni);
                ni += 1;
            }
        }

        let matched = matched_indices.len();
        let ratio = matched as f64 / total as f64;

        if matched == 0 || ratio < 0.5 {
            return None;
        }

        let mut score = (matched * 80) as i64;
        let missing = (total - matched) as i64;
        score -= missing * 25;

        let mut consecutive = 1;
        for i in 1..matched_indices.len() {
            if matched_indices[i] == matched_indices[i - 1] + 1 {
                consecutive += 1;
                score += consecutive * 10;
            } else {
                consecutive = 1;
            }
        }

        if let Some(&first) = matched_indices.first() {
            if first == 0 {
                score += 40;
            }
        }

        Some((score, matched_indices))
    }
}
