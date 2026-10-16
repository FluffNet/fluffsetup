use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    println!("cargo:rerun-if-changed=native/clock.h");
    let builder = CxxQtBuilder::new_qml_module(QmlModule::new("org.flufflinux.setup").qml_files([
        "qml/Main.qml",
        "qml/LanguagePicker.qml",
        "qml/KeyboardPicker.qml",
        "qml/KeyboardIndicator.qml",
        "qml/TimezonePicker.qml",
        "qml/KdeTimezoneMap.qml",
        "qml/TimezoneMapSurface.qml",
    ]))
    .qrc_resources([
        "assets/translate.svg",
        "assets/input-keyboard.svg",
        "assets/keyboard-layout.svg",
        "assets/globe.svg",
        "assets/edit-delete.svg",
        "assets/preferences-system-time.svg",
    ]);

    // SAFETY: This only adds a supported compiler warning flag; it does not
    // replace the CXX Qt compiler, linker, include paths, or generated sources.
    let builder = unsafe {
        builder.cc_builder(|compiler| {
            // GCC 16 can emit this warning while parsing Qt 6's QChar headers.
            // Keep every other compiler warning visible.
            compiler.flag_if_supported("-Wno-sfinae-incomplete");
            compiler.include(".");
        })
    };

    builder
        .files(["src/backend.rs", "src/clock.rs"])
        .cpp_file("native/session.cpp")
        .build();
}
