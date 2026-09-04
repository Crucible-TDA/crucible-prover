#!/usr/bin/env bash
# Regenerates the committed proof-envelope fixtures under proofs/fixtures/
# from the committed test-vector catalog, through the real CLI and the mock
# backend (deterministic: same vector + same mock key => same bytes).
#
# Requires no toolchain beyond cargo. After running, `git diff --stat
# proofs/` should be empty unless the envelope format, a vector, or the
# mock's envelope construction changed — in which case the fixture commit
# must land with that change.
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p proofs/fixtures/valid proofs/fixtures/invalid

operations=(register deposit merge transfer withdraw)
for op in "${operations[@]}"; do
    vector="test-vectors/$op/valid/$op-valid-001.json"
    out="proofs/fixtures/valid/$op-valid-001.mock.proof.json"
    echo "==> proving $op"
    cargo run -q -p crucible-cli -- prove "$op" \
        --vector "$vector" \
        --backend mock \
        --out "$out" >/dev/null
done

echo "==> writing the tampered invalid fixture"
python3 - <<'PY'
import json, pathlib
src = json.load(open("proofs/fixtures/valid/transfer-valid-001.mock.proof.json"))
hexbytes = src["proof"]["bytes"]
# Flip one hex digit of the proof bytes: verification MUST fail.
i = next((k for k, c in enumerate(hexbytes) if c not in "01"), 0)
flip = "0" if hexbytes[i] == "1" else "1"
src["proof"]["bytes"] = hexbytes[:i] + flip + hexbytes[i + 1:]
out = pathlib.Path("proofs/fixtures/invalid/transfer-tampered-proof-001.json")
out.write_text(json.dumps(src, indent=2) + "\n")
PY

echo "Fixtures regenerated (valid: ${#operations[@]} ops, invalid: 1)."
