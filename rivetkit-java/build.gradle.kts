plugins {
    base
    jacoco
}

repositories {
    mavenCentral()
}

subprojects {
    group = "dev.rivet"
    version = "0.1.0-SNAPSHOT"

    plugins.withType<JavaPlugin> {
        extensions.configure<JavaPluginExtension> {
            toolchain.languageVersion = JavaLanguageVersion.of(25)
        }
        tasks.withType<JavaCompile>().configureEach {
            options.release = 25
            // -parameters keeps action parameter names, so by-name arguments (the Rust
            // client's struct form) can be matched.
            options.compilerArgs.addAll(listOf("-Xlint:all", "-Werror", "-parameters"))
        }
        tasks.withType<Test>().configureEach {
            useJUnitPlatform()
            // FFM restricted methods (downcalls, upcalls, reinterpret) are allowed
            // only for code on the classpath, which is where the SDK lives in tests.
            jvmArgs("--enable-native-access=ALL-UNNAMED")
            testLogging {
                events("passed", "failed", "skipped")
                exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
                showStandardStreams = true
            }
        }
    }

    repositories {
        mavenCentral()
    }
}

// --- Coverage ---------------------------------------------------------------------
// `gradle coverage`: one JaCoCo report for the SDK (core) from its unit tests and the
// end-to-end tests in examples. Generated FFM bindings are excluded.
subprojects {
    if (name == "core" || name == "examples") {
        apply(plugin = "jacoco")
    }
}

tasks.register<JacocoReport>("coverage") {
    val core = project(":core")
    val tests = listOf(":core:test", ":examples:test")
    dependsOn(tests)
    executionData(tests.map { files(project(it.substringBeforeLast(":")).layout.buildDirectory.file("jacoco/test.exec")) })
    sourceDirectories.from(core.files("src/main/java"))
    classDirectories.from(core.layout.buildDirectory.dir("classes/java/main"))
    reports {
        html.required = true
        xml.required = true
        html.outputLocation = layout.buildDirectory.dir("reports/coverage")
        xml.outputLocation = layout.buildDirectory.file("reports/coverage.xml")
    }
}
