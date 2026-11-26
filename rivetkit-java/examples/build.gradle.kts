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
    // End-to-end tests spawn or reuse a local engine; give them room.
    timeout = Duration.ofMinutes(10)
    providers.gradleProperty("rivet.bench").orNull?.let { systemProperty("rivet.bench", it) }
}
