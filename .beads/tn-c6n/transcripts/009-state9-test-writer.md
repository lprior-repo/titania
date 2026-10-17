# Transcript 009 — state 9 test-writer (tn-c6n-inv-009)

Wrote 14 behavior checks + 6 properties per the frozen bridge names, all
against unmodified production. Failing-first demonstrated with a temporary
red control (14-1 red, then 14-0/6-0 green; control removed). Fixed 4 test-
side compile errors without touching production or weakening assertions.
Both files under 300 lines. No production change was needed — the suite
confirms enforcement already lives in the constructors.
