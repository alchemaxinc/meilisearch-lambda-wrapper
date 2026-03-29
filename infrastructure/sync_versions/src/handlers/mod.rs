use std::error;

mod get_latest_meilisearch_version;

type Handler = fn() -> Result<String, Box<dyn error::Error>>;

pub const HANDLERS: &[(&str, Handler)] = &[(
    "get-latest-meilisearch-version",
    get_latest_meilisearch_version::handle,
)];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections;

    #[test]
    fn test_all_handlers_have_unique_names() {
        let names: Vec<&str> = HANDLERS.iter().map(|(name, _)| return *name).collect();
        let unique: collections::HashSet<&&str> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }
}
