# tn-ebn9 Refinement-Verification Report

STATUS: PASS

Bridge (rust-refinement-obligation) verification, 4/4 rows `verified`:

- tn-ebn9.R-P1..R-P4: source refs resolve to edited production symbols;
  behavior-test refs (killer-demo E2E, lane outcome tests) and
  refinement-harness refs (detector fixture suites) are disjoint sets;
  evidence commands re-executed at closure with exit 0; evidence
  artifacts (`raw/*.log`) retained in this directory.
- `proof-test-source-alignment.jsonl` + `.md` join every requirement to
  its proof, refinement, sources, tests, harness, commands, and ledger
  PASS rows.
