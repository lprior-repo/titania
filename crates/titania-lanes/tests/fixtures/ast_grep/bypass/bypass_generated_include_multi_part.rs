// Fixture: generated include with multiple concat path segments.
// The detector must inspect all concat arguments after env!("OUT_DIR").

fn generated_module() {
    include!(concat!(env!("OUT_DIR"), "/generated", ".rs"));
}
