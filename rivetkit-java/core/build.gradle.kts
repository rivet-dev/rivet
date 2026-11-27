plugins {
    `java-library`
}

dependencies {
    implementation(project(":ffi"))
    implementation(libs.jackson.cbor)

    testImplementation(platform(libs.junit.bom))
    testImplementation(libs.junit.jupiter)
    testRuntimeOnly(libs.junit.launcher)
}

// --- Native library -------------------------------------------------------------
// Builds rivetkit-ffi with cargo and packages it at native/<os>-<arch>/<file> on
// the classpath, where NativeLoader looks for it. Release builds add one entry per
// target triple (see BRIEF.md, "Packaging").

val repoRoot: File = rootDir.parentFile
val cargoProfile: String = providers.gradleProperty("rivet.cargoProfile").getOrElse("debug")

val osName: String = System.getProperty("os.name").lowercase().let {
    when {
        it.contains("linux") -> "linux"
        it.contains("mac") -> "macos"
        it.contains("windows") -> "windows"
        else -> error("unsupported OS: $it")
    }
}
val archName: String = System.getProperty("os.arch").let {
    when (it) {
        "amd64", "x86_64" -> "x86_64"
        "aarch64", "arm64" -> "aarch64"
        else -> error("unsupported arch: $it")
    }
}
val libFile: String = when (osName) {
    "linux" -> "librivetkit_ffi.so"
    "macos" -> "librivetkit_ffi.dylib"
    else -> "rivetkit_ffi.dll"
}

val cargoBuild by tasks.registering(Exec::class) {
    description = "Builds the rivetkit-ffi native library."
    workingDir = repoRoot
    val args = mutableListOf("cargo", "build", "-p", "rivetkit-ffi")
    if (cargoProfile == "release") args += "--release"
    commandLine(args)
    // Cargo does its own change detection; let it decide every time.
    outputs.upToDateWhen { false }
}

val nativeResources = layout.buildDirectory.dir("native-resources")

val copyNative by tasks.registering(Copy::class) {
    dependsOn(cargoBuild)
    from(repoRoot.resolve("target/$cargoProfile/$libFile"))
    into(nativeResources.map { it.dir("native/$osName-$archName") })
}

sourceSets.main {
    resources.srcDir(files(nativeResources).builtBy(copyNative))
}

// --- Fixture drift test ---------------------------------------------------------
tasks.test {
    systemProperty(
        "rivet.fixtures",
        repoRoot.resolve("rivetkit-rust/packages/rivetkit-host-adapter/fixtures").absolutePath,
    )
}

dependencies {
    testImplementation(platform(libs.junit.bom))
}
