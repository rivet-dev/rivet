import java.time.Duration

plugins {
    application
}

dependencies {
    implementation(project(":core"))

    testImplementation(project(":testing"))
    testImplementation(platform(libs.junit.bom))
    testImplementation(libs.junit.jupiter)
    testRuntimeOnly(libs.junit.launcher)
}

application {
    mainClass = "dev.rivet.examples.Main"
    applicationDefaultJvmArgs = listOf("--enable-native-access=ALL-UNNAMED")
}

tasks.test {
    // A fresh engine store per run: an engine started on old data replays every actor
    // earlier runs left behind before it serves new ones (tens of seconds).
    val engineStore = layout.buildDirectory.dir("engine-store")
    doFirst {
        val dir = engineStore.get().asFile
        dir.deleteRecursively()
        dir.mkdirs()
    }
    environment("RIVETKIT_STORAGE_PATH", engineStore.get().asFile.absolutePath)
    // End-to-end tests spawn or reuse a local engine; give them room.
    timeout = Duration.ofMinutes(3)
    providers.gradleProperty("rivet.bench").orNull?.let { systemProperty("rivet.bench", it) }
    // -Privet.log=info forwards Rust logs at that level.
    providers.gradleProperty("rivet.log").orNull?.let { systemProperty("rivet.log", it) }
}
