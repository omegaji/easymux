// Cargo runs this before compiling easymux.
// It packs the files listed in resources/resources.gresource.xml into one
// bundle ("easymux.gresource"), which main.rs then includes in the program.

fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "easymux.gresource",
    );
}
