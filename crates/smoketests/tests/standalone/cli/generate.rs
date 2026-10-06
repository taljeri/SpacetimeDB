use predicates::prelude::*;
use spacetimedb_guard::ensure_binaries_built;
use spacetimedb_smoketests::patch_module_cargo_to_local_bindings;
use std::process::Command;

fn cli_cmd() -> Command {
    Command::new(ensure_binaries_built())
}

/// Standalone-only: this builds a local module and generates bindings without contacting a server.
#[test]
fn cli_generate_with_config_but_no_match_uses_cli_args() {
    // Test that when config exists but doesn't match CLI args, we use CLI args
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

    // Initialize a new project (creates <project-path>/spacetimedb/)
    let output = cli_cmd()
        .args([
            "init",
            "--non-interactive",
            "--lang",
            "rust",
            "--project-path",
            temp_dir.path().to_str().unwrap(),
            "test-project",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to execute");
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let project_dir = temp_dir.path().to_path_buf();
    let module_dir = project_dir.join("spacetimedb");
    patch_module_cargo_to_local_bindings(&module_dir).expect("failed to patch module Cargo.toml");

    // Create a config with a different module-path filter
    let config_content = r#"{
  "generate": [
    {
      "language": "typescript",
      "out-dir": "./config-output",
      "module-path": "config-module-path"
    }
  ]
}"#;
    std::fs::write(module_dir.join("spacetime.json"), config_content).expect("failed to write config");

    // Build the module first
    let output = cli_cmd()
        .args(["build", "--module-path", module_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute");
    assert!(
        output.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_dir = module_dir.join("cli-output");
    std::fs::create_dir(&output_dir).expect("failed to create output dir");

    // Generate with different module-path from CLI - should use CLI args, not config
    let output = cli_cmd()
        .args([
            "generate",
            "--lang",
            "rust",
            "--out-dir",
            output_dir.to_str().unwrap(),
            "--module-path",
            module_dir.to_str().unwrap(),
        ])
        .current_dir(&module_dir)
        .output()
        .expect("failed to execute");
    assert!(
        output.status.success(),
        "generate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify files were generated in the CLI-specified output directory
    assert!(
        predicate::path::exists().eval(&output_dir.join("lib.rs"))
            || predicate::path::exists().eval(&output_dir.join("mod.rs")),
        "Generated files should exist in CLI-specified output directory"
    );
}

/// Standalone-only: receiver/module bindings are generated from a local Rust module without contacting a server.
#[test]
fn cli_generate_rust_module_bindings() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

    let output = cli_cmd()
        .args([
            "init",
            "--non-interactive",
            "--lang",
            "rust",
            "--project-path",
            temp_dir.path().to_str().unwrap(),
            "test-project",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to execute");
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let module_dir = temp_dir.path().join("spacetimedb");
    patch_module_cargo_to_local_bindings(&module_dir).expect("failed to patch module Cargo.toml");

    let output = cli_cmd()
        .args(["build", "--module-path", module_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute");
    assert!(
        output.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_dir = module_dir.join("receiver-bindings");
    let output = cli_cmd()
        .args([
            "generate",
            "--lang",
            "rust",
            "--bindings",
            "module",
            "--out-dir",
            output_dir.to_str().unwrap(),
            "--module-path",
            module_dir.to_str().unwrap(),
        ])
        .current_dir(&module_dir)
        .output()
        .expect("failed to execute");
    assert!(
        output.status.success(),
        "generate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let root = std::fs::read_to_string(output_dir.join("lib.rs")).expect("generated lib.rs should exist");
    assert!(root.contains("pub struct Identity(pub spacetimedb::Identity);"));
    assert!(root.contains("impl spacetimedb::rt::FnInfo for add"));
    assert!(root.contains("const NAME: &'static str = \"add\";"));
    assert!(!root.contains("DbConnection"));
    assert!(!output_dir.join("add_reducer.rs").exists());
}

/// Standalone-only: a sender module can use generated receiver bindings from a spacetime.json dependency.
#[test]
fn cli_generate_receiver_bindings_from_dependency_compile_in_sender() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let receiver_project = temp_dir.path().join("receiver");
    let sender_project = temp_dir.path().join("sender");

    for (project_path, project_name) in [
        (&receiver_project, "receiver-project"),
        (&sender_project, "sender-project"),
    ] {
        let output = cli_cmd()
            .args([
                "init",
                "--non-interactive",
                "--lang",
                "rust",
                "--project-path",
                project_path.to_str().unwrap(),
                project_name,
            ])
            .current_dir(temp_dir.path())
            .output()
            .expect("failed to execute");
        assert!(
            output.status.success(),
            "init failed for {project_name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let receiver_dir = receiver_project.join("spacetimedb");
    let sender_dir = sender_project.join("spacetimedb");
    patch_module_cargo_to_local_bindings(&receiver_dir).expect("failed to patch receiver Cargo.toml");
    patch_module_cargo_to_local_bindings(&sender_dir).expect("failed to patch sender Cargo.toml");

    std::fs::write(
        receiver_dir.join("src/lib.rs"),
        r#"#[spacetimedb::reducer]
pub fn receive_ping(_ctx: &spacetimedb::ReducerContext, _target: spacetimedb::Identity, _body: String) {}
"#,
    )
    .expect("failed to write receiver module");

    std::fs::write(
        sender_dir.join("src/lib.rs"),
        r#"#[path = "receiver_bindings/game_world/lib.rs"]
mod game_world;

use spacetimedb::Table;

#[spacetimedb::table(accessor = outbound_ping, outbox(game_world::receive_ping))]
pub struct OutboundPing {
    #[primary_key]
    #[auto_inc]
    id: u64,
    target: game_world::Identity,
    body: String,
}

#[spacetimedb::reducer]
pub fn enqueue_ping(ctx: &spacetimedb::ReducerContext, target: spacetimedb::Identity, body: String) {
    ctx.db.outbound_ping().insert(OutboundPing {
        id: 0,
        target: game_world::Identity(target),
        body,
    });
}
"#,
    )
    .expect("failed to write sender module");

    std::fs::write(
        temp_dir.path().join("spacetime.json"),
        r#"{
  "server": "local",
  "children": [
    {
      "database": "game-world",
      "module-path": "./receiver/spacetimedb"
    },
    {
      "database": "lobby",
      "module-path": "./sender/spacetimedb",
      "dependencies": [
        { "name": "game_world", "database": "game-world" }
      ]
    }
  ]
}
"#,
    )
    .expect("failed to write spacetime.json");

    let output = cli_cmd()
        .args(["generate", "lobby"])
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to execute");
    assert!(
        output.status.success(),
        "generate failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = sender_dir.join("src/receiver_bindings/game_world/lib.rs");
    let root = std::fs::read_to_string(&generated).expect("generated receiver binding should exist");
    assert!(root.contains("pub struct Identity(pub spacetimedb::Identity);"));
    assert!(root.contains("impl spacetimedb::rt::FnInfo for receive_ping"));
    assert!(root.contains("receive_ping"));

    let output = cli_cmd()
        .args(["build", "--module-path", sender_dir.to_str().unwrap()])
        .output()
        .expect("failed to execute");
    assert!(
        output.status.success(),
        "sender build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
