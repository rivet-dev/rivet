// jextract output for rivetkit-ffi/include/rivetkit.h. Committed; regenerate with
// `mise run bindings`. Internal: only :core uses it.
plugins {
    `java-library`
}

tasks.withType<JavaCompile>().configureEach {
    // Generated code: keep lint output for our code only.
    options.compilerArgs.removeAll(listOf("-Xlint:all", "-Werror"))
}
