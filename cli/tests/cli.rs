//! Binary-level integration tests for the `crucible-prover` CLI.
//!
//! Every test drives the compiled binary through `std::process::Command`
//! (no extra test dependencies) and stays toolchain-free: the mock
//! backend needs neither `nargo` nor `bb`, and the `--circuits` /
//! `--catalog` overrides keep the commands hermetic against temp
//! directories. The repo's committed test vectors are exercised through
//! the real binary so the CLI's request assembly is judged against the
//! same fixtures the library tiers use.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The compiled `crucible-prover` binary.
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_crucible-prover")
}

/// The repo's committed test-vector catalog (this crate always lives at
/// `<repo>/cli/`, so `../test-vectors` is the catalog).
fn repo_catalog() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors")
}

/// Runs the binary with `args`, returning the output.
fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A minimal, structurally valid register vector: one public address
/// parameter, one private secret, no returns.
fn write_register_vector(dir: &Path, id: &str) -> PathBuf {
    let path = dir.join(format!("{id}.json"));
    std::fs::write(
        &path,
        format!(
            r#"{{
  "id": "{id}",
  "operation": "register",
  "category": "valid",
  "circuit": "register",
  "circuit_version": "0.1.0",
  "witness": {{
    "operation": "register",
    "private": {{ "account_sk": "1234" }},
    "public": {{ "entries": [["account_address", "6871944eb38ea75866d42609302692a55e12cf7620a50f2cf03381b9b382b72"]] }}
  }},
  "expected_public_outputs": {{ "entries": [] }},
  "state_reference": null,
  "expect_verification": true
}}"#
        ),
    )
    .expect("vector written");
    path
}

// --- circuits ---------------------------------------------------------------

#[test]
fn circuits_list_lists_all_five_operations() {
    let output = run(&["circuits", "list"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    for op in ["register", "deposit", "merge", "transfer", "withdraw"] {
        assert!(text.contains(op), "list must mention `{op}`:\n{text}");
    }
}

#[test]
fn circuits_check_fails_when_artifacts_are_missing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let output = run(&[
        "circuits",
        "check",
        "--circuits",
        dir.path().to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("missing artifact"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn circuits_check_passes_with_parseable_artifacts() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = dir.path().join("target");
    std::fs::create_dir_all(&target).expect("target dir");
    for op in ["register", "deposit", "merge", "transfer", "withdraw"] {
        std::fs::write(target.join(format!("{op}.json")), "{}").expect("artifact written");
    }
    let output = run(&[
        "circuits",
        "check",
        "--circuits",
        dir.path().to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).contains("all 5 operation circuits"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn circuits_compile_rejects_unknown_operation() {
    let output = run(&["circuits", "compile", "bogus"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("unknown circuit"),
        "{}",
        stderr(&output)
    );
}

// --- prove / verify ---------------------------------------------------------

#[test]
fn prove_and_verify_round_trip_through_the_mock_backend() {
    let dir = tempfile::tempdir().expect("temp dir");
    let vector = write_register_vector(dir.path(), "round-trip");
    let envelope = dir.path().join("proof.json");

    let proved = run(&[
        "prove",
        "register",
        "--vector",
        vector.to_str().unwrap(),
        "--backend",
        "mock",
        "--out",
        envelope.to_str().unwrap(),
    ]);
    assert!(proved.status.success(), "prove failed: {}", stderr(&proved));
    assert!(envelope.is_file(), "envelope must be written");

    let verified = run(&["verify", envelope.to_str().unwrap()]);
    assert!(
        verified.status.success(),
        "verify failed: {}",
        stderr(&verified)
    );
    assert!(
        stdout(&verified).contains("verified"),
        "{}",
        stdout(&verified)
    );
}

#[test]
fn prove_rejects_a_rejecting_vector_up_front() {
    let dir = tempfile::tempdir().expect("temp dir");
    let vector = dir.path().join("reject.json");
    std::fs::write(
        &vector,
        r#"{
  "id": "reject-001",
  "operation": "register",
  "category": "wrong-owner",
  "circuit": "register",
  "circuit_version": "0.1.0",
  "witness": {
    "operation": "register",
    "private": { "account_sk": "1234" },
    "public": { "entries": [["account_address", "6871944eb38ea75866d42609302692a55e12cf7620a50f2cf03381b9b382b72"]] }
  },
  "expected_public_outputs": { "entries": [] },
  "state_reference": null,
  "expect_verification": false
}"#,
    )
    .expect("vector written");
    let output = run(&["prove", "register", "--vector", vector.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("rejecting vector"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn prove_rejects_an_operation_vector_mismatch() {
    let dir = tempfile::tempdir().expect("temp dir");
    let vector = write_register_vector(dir.path(), "mismatch");
    let output = run(&["prove", "transfer", "--vector", vector.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("does not match"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn verify_rejects_a_tampered_envelope() {
    let dir = tempfile::tempdir().expect("temp dir");
    let vector = write_register_vector(dir.path(), "tamper");
    let envelope = dir.path().join("proof.json");

    let proved = run(&[
        "prove",
        "register",
        "--vector",
        vector.to_str().unwrap(),
        "--out",
        envelope.to_str().unwrap(),
    ]);
    assert!(proved.status.success(), "{}", stderr(&proved));

    // Flip the single public word the proof commits to.
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&envelope).unwrap()).unwrap();
    json["public_outputs"]["entries"][0][1] = serde_json::json!("1234");
    std::fs::write(&envelope, serde_json::to_string(&json).unwrap()).unwrap();

    let verified = run(&["verify", envelope.to_str().unwrap()]);
    assert!(!verified.status.success());
    assert!(
        stderr(&verified).contains("rejected"),
        "{}",
        stderr(&verified)
    );
}

#[test]
fn verify_rejects_a_file_that_is_not_an_envelope() {
    let dir = tempfile::tempdir().expect("temp dir");
    let bogus = dir.path().join("bogus.json");
    std::fs::write(&bogus, "not an envelope").unwrap();
    let output = run(&["verify", bogus.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("not a valid proof envelope"),
        "{}",
        stderr(&output)
    );
}

// --- vectors run ------------------------------------------------------------

#[test]
fn vectors_run_judges_the_committed_catalog_green() {
    let output = run(&["vectors", "run"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("all green"), "{text}");
    assert!(text.contains("13 vector(s) judged"), "{text}");
}

#[test]
fn vectors_run_judges_a_minimal_temp_catalog() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_register_vector(dir.path(), "only");
    let output = run(&["vectors", "run", "--catalog", dir.path().to_str().unwrap()]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).contains("1 vector(s) judged"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn vectors_run_rejects_an_unknown_operation_filter() {
    let output = run(&["vectors", "run", "--op", "bogus"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("no vectors found"),
        "{}",
        stderr(&output)
    );
}

// --- artifacts --------------------------------------------------------------

/// Writes stub compiled bytecode for all five operations under
/// `<circuits>/target/` and returns the circuits dir.
fn stub_bytecode(dir: &Path) -> PathBuf {
    let target = dir.join("target");
    std::fs::create_dir_all(&target).expect("target dir");
    for op in ["register", "deposit", "merge", "transfer", "withdraw"] {
        std::fs::write(
            target.join(format!("{op}.json")),
            format!(r#"{{ "circuit": "{op}" }}"#),
        )
        .expect("bytecode stub");
    }
    dir.to_path_buf()
}

#[test]
fn artifacts_generate_then_check_passes() {
    let work = tempfile::tempdir().expect("temp dir");
    let circuits = stub_bytecode(work.path());
    let root = work.path().join("pinned");

    let generated = run(&[
        "--circuits",
        circuits.to_str().unwrap(),
        "artifacts",
        "generate",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert!(generated.status.success(), "stderr: {}", stderr(&generated));
    assert!(
        stdout(&generated).contains("pinned transfer"),
        "{}",
        stdout(&generated)
    );

    // Generation is deterministic: identical bytecode reproduces the same
    // manifest bytes.
    let manifest = |op: &str| root.join(op).join("manifest.json");
    let before = std::fs::read(manifest("transfer")).unwrap();
    let again = run(&[
        "--circuits",
        circuits.to_str().unwrap(),
        "artifacts",
        "generate",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert!(again.status.success());
    assert_eq!(
        before,
        std::fs::read(manifest("transfer")).unwrap(),
        "manifest must be byte-stable"
    );

    let checked = run(&["artifacts", "check", "--root", root.to_str().unwrap()]);
    assert!(checked.status.success(), "stderr: {}", stderr(&checked));
    assert!(
        stdout(&checked).contains("all 5 pinned artifacts verified"),
        "{}",
        stdout(&checked)
    );
}

#[test]
fn artifacts_check_fails_on_tampered_bytecode() {
    let work = tempfile::tempdir().expect("temp dir");
    let circuits = stub_bytecode(work.path());
    let root = work.path().join("pinned");
    let generated = run(&[
        "--circuits",
        circuits.to_str().unwrap(),
        "artifacts",
        "generate",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert!(generated.status.success());

    // Flip one byte of the pinned transfer bytecode.
    let bytecode = root.join("transfer").join("transfer.json");
    let mut bytes = std::fs::read(&bytecode).unwrap();
    let last = bytes.len() - 1;
    bytes[last] = if bytes[last] == b'0' { b'1' } else { b'0' };
    std::fs::write(&bytecode, bytes).unwrap();

    let checked = run(&["artifacts", "check", "--root", root.to_str().unwrap()]);
    assert!(!checked.status.success());
    let text = format!("{}{}", stdout(&checked), stderr(&checked));
    assert!(text.contains("FAIL transfer"), "{text}");
    assert!(text.contains("failed checksum verification"), "{text}");
}

#[test]
fn artifacts_check_fails_when_manifest_is_missing() {
    let work = tempfile::tempdir().expect("temp dir");
    let circuits = stub_bytecode(work.path());
    let root = work.path().join("pinned");
    let generated = run(&[
        "--circuits",
        circuits.to_str().unwrap(),
        "artifacts",
        "generate",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert!(generated.status.success());
    std::fs::remove_file(root.join("merge").join("manifest.json")).unwrap();

    let checked = run(&["artifacts", "check", "--root", root.to_str().unwrap()]);
    assert!(!checked.status.success());
    let text = format!("{}{}", stdout(&checked), stderr(&checked));
    assert!(text.contains("FAIL merge"), "{text}");
}

#[test]
fn artifacts_generate_rejects_an_unknown_operation() {
    let output = run(&["artifacts", "generate", "bogus"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("unknown circuit"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn artifacts_inspect_reports_the_committed_artifacts() {
    // The committed pinned artifacts live under <repo>/artifacts/circuits,
    // the CLI's default root.
    let output = run(&["artifacts", "inspect"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    for op in ["register", "deposit", "merge", "transfer", "withdraw"] {
        assert!(
            text.contains(&format!("{op}:")),
            "must report `{op}`:\n{text}"
        );
    }
    // Provenance + per-file checksums are printed.
    assert!(text.contains("integrity   ok"), "{text}");
    assert!(text.contains("backend     ultrahonk"), "{text}");
    assert!(text.contains("sha256"), "{text}");
}

#[test]
fn artifacts_inspect_flags_a_tampered_artifact() {
    let work = tempfile::tempdir().expect("temp dir");
    let circuits = stub_bytecode(work.path());
    let root = work.path().join("pinned");
    let generated = run(&[
        "--circuits",
        circuits.to_str().unwrap(),
        "artifacts",
        "generate",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert!(generated.status.success());

    // Flip one byte of the pinned transfer bytecode.
    let bytecode = root.join("transfer").join("transfer.json");
    let mut bytes = std::fs::read(&bytecode).unwrap();
    let last = bytes.len() - 1;
    bytes[last] = if bytes[last] == b'0' { b'1' } else { b'0' };
    std::fs::write(&bytecode, bytes).unwrap();

    let inspected = run(&[
        "artifacts",
        "inspect",
        "transfer",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert!(!inspected.status.success());
    let text = format!("{}{}", stdout(&inspected), stderr(&inspected));
    assert!(text.contains("integrity   FAIL"), "{text}");
    assert!(text.contains("checksum"), "{text}");
    // The manifest is still printed for diagnostics on failure.
    assert!(text.contains("\"manifest_version\""), "{text}");
}

#[test]
fn artifacts_inspect_rejects_an_unknown_operation() {
    let output = run(&["artifacts", "inspect", "bogus"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("unknown circuit"),
        "{}",
        stderr(&output)
    );
}

// --- witness build ----------------------------------------------------------

#[test]
fn witness_build_writes_a_restricted_prover_toml() {
    let vector = repo_catalog()
        .join("transfer")
        .join("valid")
        .join("transfer-valid-001.json");
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("Prover.toml");

    let output = run(&[
        "witness",
        "build",
        "transfer",
        "--vector",
        vector.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("wrote"), "{}", stdout(&output));

    let contents = std::fs::read_to_string(&out).expect("witness file written");
    // Private values are hex-encoded exactly as Noir expects (0x-prefixed).
    assert!(contents.contains("sender_sk = \"0x"), "{contents}");
    // The state root halves are public context and must be present.
    assert!(contents.contains("root_hi = \"0x"), "{contents}");
    assert!(contents.contains("root_lo = \"0x"), "{contents}");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&out).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "witness file must be 0600");
    }
}

#[test]
fn witness_build_summary_never_prints_private_values() {
    let vector = repo_catalog()
        .join("register")
        .join("valid")
        .join("register-valid-001.json");
    let output = run(&[
        "witness",
        "build",
        "register",
        "--vector",
        vector.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    // The private name is listed, its value never is (the fixture secret is
    // hex `1234`; the public address shares no digits with it).
    assert!(text.contains("account_sk"), "{text}");
    assert!(text.contains("(redacted)"), "{text}");
    assert!(!text.contains("1234"), "private value leaked:\n{text}");
    assert!(
        text.contains("summary only"),
        "must mention --out when not writing: {text}"
    );
}

#[test]
fn witness_build_rejects_an_operation_vector_mismatch() {
    let dir = tempfile::tempdir().expect("temp dir");
    let vector = write_register_vector(dir.path(), "mismatch");
    let output = run(&[
        "witness",
        "build",
        "transfer",
        "--vector",
        vector.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("does not match"),
        "{}",
        stderr(&output)
    );
}

// --- benchmark --------------------------------------------------------------

#[test]
fn benchmark_times_the_mock_pipeline_for_a_committed_vector() {
    let output = run(&["benchmark", "transfer", "--iterations", "2"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("transfer-valid-001"), "{text}");
    assert!(text.contains("prove       avg"), "{text}");
    assert!(text.contains("verify      avg"), "{text}");
    assert!(text.contains("serialize   avg"), "{text}");
    assert!(text.contains("bytes"), "{text}");
    // The mock warning must be present so the numbers are not mistaken for
    // cryptographic performance.
    assert!(text.contains("TEST ONLY"), "{text}");
}

#[test]
fn benchmark_rejects_an_unknown_operation() {
    let output = run(&["benchmark", "bogus"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("unknown circuit"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn benchmark_rejects_an_unknown_backend() {
    let output = run(&["benchmark", "transfer", "--backend", "bogus"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("unknown backend"),
        "{}",
        stderr(&output)
    );
}

// --- envelope compatibility with the committed catalog ----------------------

#[test]
fn prove_and_verify_a_committed_state_bound_vector() {
    // The real transfer fixture: exercises state-reference carrying through
    // the CLI's request assembly.
    let vector = repo_catalog()
        .join("transfer")
        .join("valid")
        .join("transfer-valid-001.json");
    let dir = tempfile::tempdir().expect("temp dir");
    let envelope = dir.path().join("proof.json");

    let proved = run(&[
        "prove",
        "transfer",
        "--vector",
        vector.to_str().unwrap(),
        "--backend",
        "mock",
        "--out",
        envelope.to_str().unwrap(),
    ]);
    assert!(proved.status.success(), "prove failed: {}", stderr(&proved));
    let text = stdout(&proved);
    assert!(
        text.contains("root ab"),
        "state root must be reported: {text}"
    );

    let verified = run(&["verify", envelope.to_str().unwrap()]);
    assert!(
        verified.status.success(),
        "verify failed: {}",
        stderr(&verified)
    );
}
