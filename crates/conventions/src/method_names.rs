//! A function is an action: its name starts with a verb from `scripts/method-verbs.txt`, the list
//! the TypeScript check reads too, with the `from_`, `to_`, `is_` and `has_` idioms. Exempt: `new`, `main`,
//! methods whose name a trait imposes, and tests (their names read as sentences).

use std::collections::HashSet;
use std::sync::LazyLock;

use syn::visit::Visit;
use syn::{Attribute, ImplItemFn, ItemFn, ItemImpl, TraitItemFn};

const VERBS: &str = include_str!("../../../scripts/method-verbs.txt");
/// `main` is the entry point Rust imposes.
const EXEMPT_NAMES: [&str; 2] = ["new", "main"];
const TEST_ATTRIBUTES: [&str; 2] = ["test", "tokio::test"];

/// One verb a line; blank lines and `#` comments are skipped. Read once.
static ALLOWED_VERBS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    VERBS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
});

/// The names of the functions in `source` that do not start with an allowed verb.
/// O(n) for n syntax nodes.
///
/// # Errors
///
/// The source is not valid Rust.
pub fn find_misnamed_functions(source: &str) -> Result<Vec<String>, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut finder = MisnamedFinder {
        misnamed: Vec::new(),
    };
    finder.visit_file(&file);
    Ok(finder.misnamed)
}

struct MisnamedFinder {
    misnamed: Vec<String>,
}

impl MisnamedFinder {
    /// O(a) for a attributes.
    fn check(&mut self, name: String, attributes: &[Attribute]) {
        if is_test(attributes) || Self::is_well_named(&name) {
            return;
        }
        self.misnamed.push(name);
    }

    /// O(1).
    fn is_well_named(name: &str) -> bool {
        let first_word = name
            .trim_start_matches('_')
            .split('_')
            .next()
            .unwrap_or_default();
        EXEMPT_NAMES.contains(&name) || ALLOWED_VERBS.contains(first_word)
    }
}

impl<'ast> Visit<'ast> for MisnamedFinder {
    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        self.check(function.sig.ident.to_string(), &function.attrs);
        syn::visit::visit_item_fn(self, function);
    }

    /// A trait's implementation keeps the trait's names: only inherent methods are checked.
    fn visit_item_impl(&mut self, block: &'ast ItemImpl) {
        if block.trait_.is_some() {
            return;
        }
        syn::visit::visit_item_impl(self, block);
    }

    fn visit_impl_item_fn(&mut self, method: &'ast ImplItemFn) {
        self.check(method.sig.ident.to_string(), &method.attrs);
        syn::visit::visit_impl_item_fn(self, method);
    }

    fn visit_trait_item_fn(&mut self, method: &'ast TraitItemFn) {
        self.check(method.sig.ident.to_string(), &method.attrs);
        syn::visit::visit_trait_item_fn(self, method);
    }
}

/// `#[test]`, `#[tokio::test]` and their variants with arguments. O(a) for a attributes.
fn is_test(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        let path = attribute.path();
        let name = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        TEST_ATTRIBUTES.contains(&name.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(source: &str) -> Vec<String> {
        find_misnamed_functions(source).expect("valid Rust")
    }

    #[test]
    fn rejects_a_function_named_like_a_thing() {
        // Act
        let misnamed = find("fn total_price() -> u32 { 0 } fn compute_total_price() -> u32 { 0 }");

        // Assert
        assert_eq!(misnamed, ["total_price"]);
    }

    #[test]
    fn checks_inherent_and_trait_methods_but_not_trait_implementations() {
        // Arrange
        let source = "
            struct Order;
            impl Order { fn shipping_city(&self) {} fn read_city(&self) {} }
            trait Repository { fn candidates(&self); }
            impl Default for Order { fn default() -> Self { Order } }
        ";

        // Act
        let misnamed = find(source);

        // Assert
        assert_eq!(misnamed, ["shipping_city", "candidates"]);
    }

    #[test]
    fn exempts_constructors_conversions_predicates_and_tests() {
        // Arrange
        let source = "
            fn new() {} fn from_octets() {} fn to_text() {} fn is_up() {} fn has_route() {}
            #[test] fn keeps_the_peak() {}
            #[tokio::test] async fn asks_the_box() {}
        ";

        // Act & Assert
        assert!(find(source).is_empty());
    }
}
