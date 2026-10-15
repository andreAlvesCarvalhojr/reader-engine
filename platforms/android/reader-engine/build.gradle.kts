plugins {
    id("com.android.library")
    id("maven-publish")
}

val libraryGroup = "com.github.andreAlvesCarvalhojr"
val libraryArtifact = "reader-engine"
val libraryVersion = project.findProperty("VERSION_NAME")?.toString()
    ?: System.getenv("VERSION_NAME")
    ?: "0.1.0-SNAPSHOT"

// Public Maven layout (no credentials for consumers). CI pushes this to the `mvn-repo` branch.
val publicMavenDir = rootProject.layout.projectDirectory.dir("public-maven")

android {
    namespace = "com.readerengine"
    compileSdk = 36

    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.directories += "src/main/jniLibs"
        }
    }
}

dependencies {
    api("net.java.dev.jna:jna:5.15.0@aar")
    implementation("androidx.annotation:annotation:1.9.1")
}

afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("release") {
                from(components["release"])
                groupId = libraryGroup
                artifactId = libraryArtifact
                version = libraryVersion

                pom {
                    name.set("reader-engine")
                    description.set("UniFFI Android bindings for reader-engine (EPUB/HPUB/JSON)")
                    url.set("https://github.com/andreAlvesCarvalhojr/reader-engine")
                    licenses {
                        license {
                            name.set("MIT License")
                            url.set("https://opensource.org/licenses/MIT")
                        }
                    }
                    developers {
                        developer {
                            id.set("andreAlvesCarvalhojr")
                            name.set("Andre Alves C. Jr.")
                        }
                    }
                    scm {
                        connection.set("scm:git:git://github.com/andreAlvesCarvalhojr/reader-engine.git")
                        developerConnection.set("scm:git:ssh://github.com/andreAlvesCarvalhojr/reader-engine.git")
                        url.set("https://github.com/andreAlvesCarvalhojr/reader-engine")
                    }
                }
            }
        }

        repositories {
            // Primary: public file Maven repo (published to GitHub branch `mvn-repo`)
            maven {
                name = "PublicMaven"
                url = publicMavenDir.asFile.toURI()
            }
            // Optional: GitHub Packages (requires token; useful for CI-only)
            maven {
                name = "GitHubPackages"
                url = uri("https://maven.pkg.github.com/andreAlvesCarvalhojr/reader-engine")
                credentials {
                    username = project.findProperty("gpr.user") as String?
                        ?: System.getenv("GITHUB_ACTOR")
                    password = project.findProperty("gpr.key") as String?
                        ?: System.getenv("GITHUB_TOKEN")
                }
            }
        }
    }
}
