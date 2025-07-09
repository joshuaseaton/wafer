// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use std::error::Error;
use std::path::PathBuf;
use std::{env, fs, path::Path, process::Command as ProcessCommand};

use wast2json::{Command, ModuleType, TestFile};

const WAST_TEST_DIR: &str = "../third-party/github.com/WebAssembly/spec/test/core";

fn process_wast_file(wast_path: &Path, out_dir: &Path) -> Result<Vec<Command>, Box<dyn Error>> {
    // Generate output JSON file path
    let file_stem = wast_path.file_stem().unwrap().to_str().unwrap();
    let json_file = out_dir.join(format!("{file_stem}.json"));

    // Run wast2json with working directory set to our output directory
    // This prevents .wasm/.wat files from being created in the workspace
    let output = ProcessCommand::new("wast2json")
        .arg(wast_path)
        .arg("--output")
        .arg(&json_file)
        .current_dir(out_dir) // Set working directory to output directory
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "wast2json failed for {}: {}",
            wast_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let json_str = fs::read_to_string(&json_file)?;
    let TestFile { mut commands, .. } = serde_json::from_str(&json_str)?;

    // Since we're not testing any .wat parsing functionality, we filter out all
    // tests involving malformed wat-encoded modules. We also throw away any
    // commands not yet supported.
    commands.retain(|command| {
        if let Command::AssertMalformed(malformed) = command {
            malformed.module_type == ModuleType::Binary
        } else {
            true
        }
    });
    Ok(commands)
}

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);

    // Path to the WAST files directory
    let wast_dir = manifest_dir.join(WAST_TEST_DIR);

    // Read the list of WAST files from wast.json
    let wast_json_path = manifest_dir.join("wast.json");
    println!("cargo:rerun-if-changed={}", wast_json_path.display());

    let wast_json_content = fs::read_to_string(&wast_json_path)?;
    let wast_filenames: Vec<String> = serde_json::from_str(&wast_json_content)?;

    fs::create_dir_all(&out_dir)?;

    // Collect all commands from specified WAST files
    let mut all_commands = Vec::new();

    // Process each WAST file listed in wast.json
    for filename in wast_filenames {
        let path = wast_dir.join(&filename);
        println!("cargo:rerun-if-changed={}", path.display());

        let mut commands = process_wast_file(&path, &out_dir)?;
        all_commands.append(&mut commands);
    }

    // Write the amalgamated JSON file at the root of the build directory
    let amalgamated_file = out_dir.join("spec-tests.json");
    let json_output = serde_json::to_string_pretty(&all_commands)?;
    fs::write(&amalgamated_file, json_output)?;

    Ok(())
}
