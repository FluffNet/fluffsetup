use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    let builder = CxxQtBuilder::new_qml_module(
        QmlModule::new("org.flufflinux.setup").qml_file("qml/Main.qml"),
    );

    // SAFETY: This only adds a supported compiler warning flag; it does not
    // replace the CXX Qt compiler, linker, include paths, or generated sources.
    let builder = unsafe {
        builder.cc_builder(|compiler| {
            // GCC 16 can emit this warning while parsing Qt 6's QChar headers.
            // Keep every other compiler warning visible.
            compiler.flag_if_supported("-Wno-sfinae-incomplete");
        })
    };

    builder
        .files(["src/backend.rs"])
        .cpp_file("native/session.cpp")
        .build();
}
