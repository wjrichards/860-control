//! Compiles the app's resources (its own icons) into the binary.

fn main() {
    glib_build_tools::compile_resources(&["data"], "data/resources.gresource.xml", "resources.gresource");
}
