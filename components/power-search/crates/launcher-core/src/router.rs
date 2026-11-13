use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Targets {
    All,
    Only(String),
}

impl Targets {
    pub fn matches(&self, id: &str) -> bool {
        match self {
            Targets::All => true,
            Targets::Only(module) => module == id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routed<'a> {
    pub targets: Targets,
    pub body: &'a str,
    pub prefixed: bool,
}

#[derive(Debug, Clone)]
pub struct Router {
    table: Vec<(String, String)>,
}

impl Router {
    pub fn new(prefixes: &BTreeMap<String, String>) -> Self {
        let mut table: Vec<(String, String)> = prefixes
            .iter()
            .map(|(prefix, module)| (prefix.clone(), module.clone()))
            .collect();
        table.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Self { table }
    }

    pub fn route<'a>(&self, raw: &'a str) -> Routed<'a> {
        let trimmed = raw.trim_start();
        match self
            .table
            .iter()
            .find(|(prefix, _)| trimmed.starts_with(prefix.as_str()))
        {
            Some((prefix, module)) => Routed {
                targets: Targets::Only(module.clone()),
                body: trimmed[prefix.len()..].trim(),
                prefixed: true,
            },
            None => Routed {
                targets: Targets::All,
                body: trimmed.trim(),
                prefixed: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Router, Targets};
    use std::collections::BTreeMap;

    fn router() -> Router {
        let mut prefixes = BTreeMap::new();
        prefixes.insert(String::from("f "), String::from("files"));
        prefixes.insert(String::from("f"), String::from("windows"));
        prefixes.insert(String::from("="), String::from("calc"));
        Router::new(&prefixes)
    }

    #[test]
    fn prefix_routes_to_its_module_and_strips_the_prefix() {
        let routed = router().route("f  readme");
        assert_eq!(routed.targets, Targets::Only(String::from("files")));
        assert_eq!(routed.body, "readme");
        assert!(routed.prefixed);
    }

    #[test]
    fn longest_prefix_wins() {
        let routed = router().route("f x");
        assert_eq!(routed.targets, Targets::Only(String::from("files")));
    }

    #[test]
    fn unprefixed_query_searches_every_module() {
        let routed = router().route("chrome");
        assert_eq!(routed.targets, Targets::All);
        assert_eq!(routed.body, "chrome");
        assert!(!routed.prefixed);
    }

    #[test]
    fn calculator_prefix() {
        let routed = router().route("=2+2");
        assert_eq!(routed.targets, Targets::Only(String::from("calc")));
        assert_eq!(routed.body, "2+2");
    }
}
