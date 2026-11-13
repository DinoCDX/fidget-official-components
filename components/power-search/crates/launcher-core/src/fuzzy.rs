use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};

pub fn score(needle: &str, haystack: &str) -> Option<u32> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(needle, CaseMatching::Ignore, Normalization::Smart);
    let mut buffer = Vec::new();
    let text = Utf32Str::new(haystack, &mut buffer);
    pattern.score(text, &mut matcher)
}
