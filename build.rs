extern crate protoc_rust;

use protoc_rust::Customize;
use std::fmt;
use std::io;
use std::process;

struct MissingDependency {
    binary: String,
    additional_information: String,
}

enum DependencyCheckError {
    Unexpected(io::Error),
    MissingDependencies(Vec<MissingDependency>),
}

impl fmt::Debug for DependencyCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyCheckError::Unexpected(error) => write!(f, "{:?}", error),
            DependencyCheckError::MissingDependencies(missing_dependencies) => {
                let message = missing_dependencies
                    .iter()
                    .map(
                        |MissingDependency {
                             binary,
                             additional_information,
                         }| {
                            format!(
                                "\"which {}\" did not find a version of {} on your machine.  {}",
                                binary, binary, additional_information
                            )
                        },
                    )
                    .collect::<Vec<String>>()
                    .join("\n");
                write!(f, "{:?}", message)
            }
        }
    }
}

fn check_for_dependencies() -> Result<(), DependencyCheckError> {
    let mut maybe_missing_dependencies = vec![];
    maybe_missing_dependencies.push(MissingDependency {
        binary: "protoc".to_owned(),
        additional_information: "Follow the instructions to install the compiler here (https://developers.google.com/protocol-buffers) or ensure that it is on your PATH.".to_owned(),
    });
    maybe_missing_dependencies.push(MissingDependency {
        binary: "npm".to_owned(),
        additional_information:
            "Follow the instructions at https://nodejs.org/ to install node and NPM".to_owned(),
    });
    maybe_missing_dependencies.push(MissingDependency {
        binary: "node".to_owned(),
        additional_information:
            "Follow the instructions at https://nodejs.org/ to install node and NPM".to_owned(),
    });

    let mut missing_dependencies = vec![];

    for maybe_missing_dependency in maybe_missing_dependencies {
        let status = process::Command::new("which")
            .arg(&maybe_missing_dependency.binary)
            .status()
            .map_err(|error| DependencyCheckError::Unexpected(error))?;

        if !status.success() {
            missing_dependencies.push(maybe_missing_dependency);
        }
    }

    if missing_dependencies.len() > 0 {
        return Err(DependencyCheckError::MissingDependencies(
            missing_dependencies,
        ));
    }

    Ok(())
}

fn generate_server_protocol_buffers() -> io::Result<()> {
    protoc_rust::Codegen::new()
        .out_dir("src/protos")
        .includes(&["./"])
        .inputs(&["./definitions.proto"])
        .include("protos")
        .customize(Customize {
            serde_derive: Some(true),
            ..Default::default()
        })
        .run()
}

fn generate_client_protocol_buffers() -> io::Result<()> {
    let status = process::Command::new("protoc")
        .arg("--js_out=import_style=commonjs,binary:ui")
        .arg("./definitions.proto")
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Got a nonzero exit code generating client protobuf definitions",
        ));
    }

    Ok(())
}

fn install_javascript_dependencies() -> io::Result<()> {
    let status = process::Command::new("npm")
        .arg("install")
        .current_dir("ui")
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Got a nonzero exit code running \"npm install\"",
        ));
    }

    Ok(())
}

fn build_javascript() -> io::Result<()> {
    let status = process::Command::new("node")
        .arg("build.js")
        .current_dir("ui")
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Got a nonzero exit code running \"node build.js\"",
        ));
    }

    Ok(())
}

fn main() {
    check_for_dependencies().unwrap();
    generate_server_protocol_buffers().unwrap();
    generate_client_protocol_buffers().unwrap();
    install_javascript_dependencies().unwrap();
    build_javascript().unwrap();
}
