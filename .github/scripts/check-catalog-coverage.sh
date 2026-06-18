#!/usr/bin/env bash
set -euo pipefail

MIN_COVERAGE=85

# Compute aggregate line coverage for catalogue source files from
# `cargo llvm-cov report --summary-only`.

report=$(cargo llvm-cov report --summary-only)

summary=$(awk -v min="$MIN_COVERAGE" '
    # Match catalogue source files:
    # - anything under a /catalog/ directory
    # - api/src/handlers/deals/catalog.rs
    # - filenames: catalog.rs, resource.rs, need.rs, enhancement.rs,
    #   catalog_repository.rs, postgres_catalog_repository.rs
    /\/catalog\// || /\/deals\/catalog\.rs[[:space:]]/ || /\/(catalog|resource|need|enhancement|catalog_repository|postgres_catalog_repository)\.rs[[:space:]]/ {
        total += $2
        missed += $3
    }
    END {
        covered = total - missed
        if (total == 0) {
            print "0 0 0.00 FAIL"
            exit 0
        }
        coverage = (covered / total) * 100
        status = (coverage >= min) ? "PASS" : "FAIL"
        printf "%d %d %.2f %s\n", covered, total, coverage, status
    }
' <<<"$report")

read -r covered total coverage status <<<"$summary"

echo "Catalogue coverage: $covered / $total = $coverage%"

if [[ "$status" != "PASS" ]]; then
    echo "Catalogue coverage $coverage% is below the required $MIN_COVERAGE%"
    exit 1
fi
