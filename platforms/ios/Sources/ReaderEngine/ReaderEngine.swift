/// Thin product target that re-exports the UniFFI-generated Swift API.
///
/// Build the native XCFramework first:
/// ```bash
/// ./scripts/ios-build-xcframework.sh
/// ```
/// Then in your app: `import ReaderEngine` / use `ReaderEngine()` from `ReaderFfi`.

@_exported import ReaderFfi
