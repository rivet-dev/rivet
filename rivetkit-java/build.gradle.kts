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
