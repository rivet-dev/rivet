import java.net.InetAddress
import java.net.ServerSocket
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
        // A free port per run, so the suite never reuses (or fights over) an engine that a
        // developer or another run left on the default 6420. The engine also binds +1 and +10.
        val loopback = InetAddress.getLoopbackAddress()
        fun bindable(p: Int) = p <= 65535 && runCatching { ServerSocket(p, 1, loopback).close() }.isSuccess
        val port = generateSequence { ServerSocket(0, 1, loopback).use { it.localPort } }
            .take(50)
            .firstOrNull { bindable(it) && bindable(it + 1) && bindable(it + 10) }
            ?: error("no free port block for the test engine")
        (this as Test).systemProperty("rivet.test.endpoint", "http://127.0.0.1:$port")
    }
    environment("RIVETKIT_STORAGE_PATH", engineStore.get().asFile.absolutePath)
    // End-to-end tests spawn or reuse a local engine; give them room.
    timeout = Duration.ofMinutes(3)
    providers.gradleProperty("rivet.bench").orNull?.let { systemProperty("rivet.bench", it) }
    // -Privet.log=info forwards Rust logs at that level.
    providers.gradleProperty("rivet.log").orNull?.let { systemProperty("rivet.log", it) }
}

