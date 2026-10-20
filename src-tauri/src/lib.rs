//! Tauri shell: wires the adapters to the use cases and exposes typed commands to the front end.
#![forbid(unsafe_code)]

pub mod commands;

use tauri_specta::{Builder, collect_commands};

/// Single list of the commands exposed to the front end, shared by the runtime and the bindings export.
#[must_use]
pub fn create_specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![commands::read_app_info])
}

/// Starts the desktop application.
///
/// # Panics
///
/// Panics when the Tauri runtime cannot start, which leaves nothing to recover.
pub fn run() {
    let builder = create_specta_builder();
    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("the Tauri runtime failed to start");
}

#[cfg(test)]
mod tests {
    use specta_typescript::Typescript;

    // Regenerates the front-end bindings; CI then fails if the committed file differs.
    #[test]
    fn exports_up_to_date_typescript_bindings() {
        // Arrange
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");

        // Act
        let result = super::create_specta_builder().export(Typescript::default(), path);

        // Assert
        assert!(result.is_ok(), "bindings export failed: {result:?}");
    }
}
