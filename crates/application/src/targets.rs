use domain::Target;

/// Two providers, so one slow or filtering target cannot decide the verdict alone.
#[must_use]
pub fn list_default_targets() -> Vec<Target> {
    vec![Target::parse("1.1.1.1"), Target::parse("8.8.8.8")]
}
