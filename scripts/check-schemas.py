#!/usr/bin/env python3
"""Validate the JSON schemas in schemas/ against representative documents.

Every schema file must be a well-formed draft-07 schema, and a set of
representative documents (captured from the Rust wire types) must validate
against the schema that describes them, while deliberately-broken documents
must be rejected.

Usage: python3 scripts/check-schemas.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from jsonschema import Draft7Validator, ValidationError
from referencing import Registry
from referencing.jsonschema import DRAFT7

ROOT = Path(__file__).resolve().parent.parent
SCHEMAS = ROOT / "schemas"
# Live test-vector catalog: every file must validate against
# test-vector.schema.json (the cross-language contract).
TEST_VECTORS = ROOT / "test-vectors"

DOCS = {
    # A proof envelope exactly as crucible_proof_types::ProofEnvelope
    # serializes it (captured from the mock backend fixtures).
    "proof.schema.json": {
        "version": 1,
        "operation": "transfer",
        "circuit": "transfer",
        "circuit_version": "0.1.0",
        "backend": "mock",
        "proof": {
            "format": "mock-envelope-v1",
            "bytes": "deadbeef",
        },
        "public_outputs": {
            "entries": [
                ["sender_address", "aa11"],
                ["recipient_address", "bb22"],
            ]
        },
        "verification_key_id": "mock-vk/transfer/0.1.0",
        "artifact_checksum": "ab" * 32,
        "state_reference": {
            "root": "ab" * 32,
            "sequence": 1,
            "label": "transfer-1",
        },
        "metadata": {"request_id": "req-transfer-1", "produced_by": "mock/0.1.0"},
    },
    "proof-response.schema.json": {
        "request_id": "req-transfer-1",
        "circuit": "transfer",
        "circuit_version": "0.1.0",
        "backend": "mock",
        "proof": {"format": "mock-envelope-v1", "bytes": "deadbeef"},
        "public_outputs": {"entries": [["old_sender_commitment", "cc33"]]},
        "verification_key_id": "mock-vk/transfer/0.1.0",
        "artifact_checksum": "ab" * 32,
        "state_reference": None,
    },
    # The redacted view of a proof request: names/count only, never values.
    "proof-request.schema.json": {
        "request_id": "req-transfer-1",
        "operation": "transfer",
        "circuit": "transfer",
        "circuit_version": "0.1.0",
        "artifact_version": "0.1.0",
        "backend": "mock",
        "private_witness": {"names": ["sender_sk", "amount"], "count": 2},
        "public_inputs": {"entries": [["sender_address", "aa11"]]},
        "state_reference": None,
    },
    "witness.schema.json": {
        "operation": "transfer",
        "private": {"sender_sk": "deadbeef", "amount": "7f"},
        "public": {"entries": [["sender_address", "aa11"]]},
    },
    "circuit.schema.json": {
        "id": "transfer",
        "operation": "transfer",
        "version": "0.1.0",
    },
    "artifact.schema.json": {
        "manifest_version": 1,
        "circuit": "transfer",
        "circuit_version": "0.1.0",
        "artifact_version": "0.1.0",
        "backend": "ultrahonk",
        "verification_key_id": "uhk/transfer/0.1.0/" + "ab" * 32,
        "files": [
            {
                "path": "acir.bin",
                "sha256": "ab" * 32,
                "kind": "acir",
            },
            {
                "path": "keys/vk.bin",
                "sha256": "cd" * 32,
                "kind": "verification-key",
            },
        ],
        "backend_metadata": {"bb": "1.0.0"},
    },
    "verification-key.schema.json": {
        "id": "uhk/transfer/0.1.0/" + "ab" * 32,
        "checksum": "cd" * 32,
    },
    "test-vector.schema.json": {
        "id": "transfer-valid-001",
        "operation": "transfer",
        "category": "valid",
        "description": "simple transfer",
        "circuit": "transfer",
        "circuit_version": "0.1.0",
        "witness": {
            "operation": "transfer",
            "private": {"sender_sk": "deadbeef", "amount": "7f", "blinding": "0102030405"},
            "public": {"entries": [["sender_address", "aa11"]]},
        },
        "expected_public_outputs": {"entries": [["new_commitment", "c0ffee"]]},
        "state_reference": {"root": "ab" * 32, "sequence": 1, "label": None},
        "expect_verification": True,
    },
}

# Documents that must FAIL validation of the named schema.
INVALID_DOCS = {
    "proof.schema.json": [
        # Artifact checksum not 64 hex chars.
        {"version": 1, "operation": "transfer", "circuit": "transfer",
         "circuit_version": "0.1.0", "backend": "mock",
         "proof": {"format": "mock-envelope-v1", "bytes": "deadbeef"},
         "public_outputs": {"entries": []},
         "verification_key_id": "mock-vk/transfer/0.1.0",
         "artifact_checksum": "zz", "state_reference": None,
         "metadata": {"request_id": "r", "produced_by": "p"}},
        # Unknown operation.
        {"version": 1, "operation": "burn", "circuit": "transfer",
         "circuit_version": "0.1.0", "backend": "mock",
         "proof": {"format": "mock-envelope-v1", "bytes": "deadbeef"},
         "public_outputs": {"entries": []},
         "verification_key_id": "mock-vk", "artifact_checksum": "ab" * 32,
         "state_reference": None, "metadata": {"request_id": "r", "produced_by": "p"}},
    ],
    "proof-request.schema.json": [
        # Private witness with a value must never appear in a redacted view.
        {"request_id": "r", "operation": "transfer", "circuit": "transfer",
         "circuit_version": "0.1.0", "artifact_version": "0.1.0", "backend": "mock",
         "private_witness": {"names": ["amount"], "count": 1, "values": ["7f"]},
         "public_inputs": {"entries": []}, "state_reference": None},
    ],
    "witness.schema.json": [
        # Missing operation.
        {"private": {}, "public": {"entries": []}},
    ],
    "artifact.schema.json": [
        # Empty file list is not an artifact.
        {"manifest_version": 1, "circuit": "transfer",
         "circuit_version": "0.1.0", "artifact_version": "0.1.0",
         "backend": "ultrahonk", "verification_key_id": None,
         "files": [], "backend_metadata": {}},
    ],
}


def build_registry() -> Registry:
    """Register every schema file keyed by its $id."""
    resources = {}
    for path in sorted(SCHEMAS.glob("*.schema.json")):
        doc = json.loads(path.read_text())
        if "$id" not in doc:
            raise SystemExit(f"{path.name}: missing $id")
        # Draft-07 detection matters: from_contents on a schema without an
        # explicit dialect keyword assumes the latest dialect.
        resources[doc["$id"]] = DRAFT7.create_resource(doc)
    return Registry().with_resources(resources.items())


def main() -> int:
    registry = build_registry()
    failures = 0

    # 1. Every schema is itself a valid draft-07 schema.
    for path in sorted(SCHEMAS.glob("*.schema.json")):
        doc = json.loads(path.read_text())
        errors = list(Draft7Validator.check_schema(doc) or [])
        if errors:
            failures += 1
            print(f"FAIL {path.name}: not a valid schema: {errors[0]}")
        else:
            print(f"ok   {path.name}: schema is well-formed")

    # 2. Representative documents validate.
    for schema_name, doc in sorted(DOCS.items()):
        schema = json.loads((SCHEMAS / schema_name).read_text())
        validator = Draft7Validator(schema, registry=registry)
        errors = sorted(validator.iter_errors(doc), key=lambda e: list(e.path))
        if errors:
            failures += 1
            print(f"FAIL {schema_name}: expected document to validate: {errors[0].message}")
        else:
            print(f"ok   {schema_name}: representative document validates")

    # 3. Broken documents are rejected.
    for schema_name, docs in sorted(INVALID_DOCS.items()):
        schema = json.loads((SCHEMAS / schema_name).read_text())
        validator = Draft7Validator(schema, registry=registry)
        for doc in docs:
            errors = list(validator.iter_errors(doc))
            if not errors:
                failures += 1
                print(f"FAIL {schema_name}: broken document was accepted")
            else:
                print(f"ok   {schema_name}: broken document rejected ({errors[0].message[:60]})")

    # 4. Every live test vector validates against test-vector.schema.json.
    if TEST_VECTORS.is_dir():
        schema = json.loads((SCHEMAS / "test-vector.schema.json").read_text())
        validator = Draft7Validator(schema, registry=registry)
        for path in sorted(TEST_VECTORS.rglob("*.json")):
            doc = json.loads(path.read_text())
            errors = sorted(validator.iter_errors(doc), key=lambda e: list(e.path))
            if errors:
                failures += 1
                print(f"FAIL {path.relative_to(ROOT)}: {errors[0].message}")
            else:
                print(f"ok   {path.relative_to(ROOT)}: validates against test-vector schema")
    else:
        print(f"skip test-vectors sweep: {TEST_VECTORS} not present")

    if failures:
        print(f"\n{failures} failure(s)")
        return 1
    print("\nall schema checks passed")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except ValidationError as e:  # pragma: no cover
        print(f"unexpected validation error: {e}")
        sys.exit(1)
