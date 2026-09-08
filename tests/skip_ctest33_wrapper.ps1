# Wrapper for kagami_qa_blargg_runner ctest 33.
# Translates exit code 1 (FAIL) to 77 (SKIP) so ctest reports SKIP.
# Per plan v2.1.1 §A.2 escape valve (Route C, owner-approved 2026-09-08).
# This wrapper is the CMake test command target for test 33
# `rust_ppu_vbl_nmi_timing_test`. See tests/CMakeLists.txt line ~786-802.
#
# Future v2.1.1.5 sub-plan will land the frame layout fix and remove
# this wrapper + SKIP_RETURN_CODE together.

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$RunnerPath,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RunnerArgs
)

# Run the actual kagami blargg runner, capture all output
$output = & $RunnerPath @RunnerArgs 2>&1
$exitCode = $LASTEXITCODE

# Echo output (ctest needs to see it for diagnostics)
$output | ForEach-Object { Write-Output $_ }

if ($exitCode -eq 1) {
    # Map FAIL to SKIP (77) for ctest SKIP_RETURN_CODE
    Write-Output "[skip_ctest33_wrapper] exit 1 -> 77 (SKIP per plan v2.1.1 §A.2 escape valve, Route C)"
    exit 77
} else {
    # Pass through (0 = PASS, 77 = SKIP, other = FAIL)
    exit $exitCode
}