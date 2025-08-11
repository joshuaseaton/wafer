// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use std::collections::HashSet;
use std::path::PathBuf;
use std::string::ToString;
use std::{env, fs, path::Path};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::Ident;

use wast2json::{AssertMalformedCommand, Command, ModuleCommand, ModuleType};

// TODO: Burn this list down!
const DISABLED_TESTS: &[&str] = &[
    "align_109_assert_malformed_892",
    "align_110_assert_malformed_911",
    "align_111_assert_malformed_930",
    "align_112_assert_malformed_949",
    "align_113_assert_malformed_968",
    "binary_66_assert_malformed_494",
    "binary_67_assert_malformed_517",
    "custom_5_assert_malformed_77",
    "custom_6_assert_malformed_85",
];

fn load_spec_tests() -> Vec<Command> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let spec_tests_file = PathBuf::from(out_dir).join("spec-tests.json");

    let json_str = fs::read_to_string(&spec_tests_file)
        .expect("Failed to read `spec-tests.json` - make sure build.rs ran successfully");

    serde_json::from_str(&json_str).expect("Failed to parse `spec-tests.json`")
}

fn generate_tests() -> Vec<TokenStream2> {
    let mut all_test_functions = Vec::new();
    let commands = load_spec_tests();

    // Convert disabled tests to HashSet for efficient lookup
    let mut disabled_tests: HashSet<String> =
        DISABLED_TESTS.iter().map(ToString::to_string).collect();

    let test_name = |filename: &str| {
        Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap()
            .replace(['-', '.'], "_")
    };

    let mut current_module: Option<&ModuleCommand> = None;
    let mut trailing_commands: Vec<&Command> = Vec::new();

    macro_rules! generate_module_test_case {
        () => {
            if let Some(module) = &current_module {
                let name = test_name(&module.filename);
                let is_disabled = disabled_tests.remove(&name);
                let test_case = module_test_case(&name, module, &trailing_commands, is_disabled);
                all_test_functions.push(test_case);

                current_module = None;
                trailing_commands.clear();

                let _ = current_module;
            }
        };
    }

    for command in &commands {
        match command {
            Command::Module(module) => {
                generate_module_test_case!();
                current_module = Some(module);
            }
            Command::AssertMalformed(malformed) => {
                generate_module_test_case!();

                //
                assert_eq!(malformed.module_type, ModuleType::Binary);

                let name = test_name(&malformed.filename);
                let name = format!("{name}_assert_malformed_{}", malformed.line);
                let is_disabled = disabled_tests.remove(&name);
                let test_case = assert_malformed_test_case(&name, malformed, is_disabled);
                all_test_functions.push(test_case);
            }
            _ => {
                trailing_commands.push(command);
            }
        }
    }
    generate_module_test_case!();

    // Validate that all disabled test names were found
    assert!(
        disabled_tests.is_empty(),
        "Disabled tests list contains invalid test names: {disabled_tests:?}",
    );

    all_test_functions
}

fn assert_malformed_test_case(
    name: &str,
    malformed: &AssertMalformedCommand,
    is_disabled: bool,
) -> TokenStream2 {
    if malformed.module_type != ModuleType::Binary {
        return quote! {};
    }

    // Convert relative path to absolute path
    let out_dir = env::var("OUT_DIR").unwrap();
    let wasm_file = PathBuf::from(out_dir).join(&malformed.filename);
    let wasm_file = wasm_file.to_string_lossy();

    let name = Ident::new(name, Span::call_site());

    let wasm_file = wasm_file.as_ref();
    let ignore_attr = if is_disabled {
        quote! { #[ignore] }
    } else {
        quote! {}
    };
    let error_variant = {
        let variant = format!("wast2json::Error::{:?}", malformed.text);
        let tokens: TokenStream2 = variant.parse().expect("Failed to parse error variant");
        tokens
    };

    quote! {
        #[test]
        #ignore_attr
        fn #name() {
            assert_malformed(#wasm_file, &#error_variant);
        }
    }
}

fn module_test_case(
    name: &str,
    module: &ModuleCommand,
    _commands: &[&Command],
    is_disabled: bool,
) -> TokenStream2 {
    // Convert relative path to absolute path
    let out_dir = env::var("OUT_DIR").unwrap();
    let wasm_file = PathBuf::from(out_dir).join(&module.filename);
    let wasm_file = wasm_file.to_string_lossy();

    let name = Ident::new(name, Span::call_site());
    let wasm_file_str = wasm_file.as_ref();
    let ignore_attr = if is_disabled {
        quote! { #[ignore] }
    } else {
        quote! {}
    };

    // For now, just load the module - later we can process the commands vector
    // to generate additional test logic for assert_return, assert_trap, etc.
    quote! {
        #ignore_attr
        #[test]
        fn #name() {
            check_module(#wasm_file_str);
        }
    }
}

/// Generate WebAssembly spec tests from the amalgamated JSON file
///
/// Usage: `wasm_spec_tests!()`
///
/// This macro will:
/// 1. Read the amalgamated `spec-tests.json` file created by build.rs
/// 2. Generate individual test functions for each assertion
///
#[proc_macro]
pub fn wasm_spec_tests(_input: TokenStream) -> TokenStream {
    let all_test_functions = generate_tests();

    let result = quote! {
        #(#all_test_functions)*
    };

    result.into()
}
