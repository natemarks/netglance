#!/bin/bash
# Fetch PR check failures and save to log

set -e

PR_NUMBER=${1:-8}
OUTPUT_FILE=${2:-pr-${PR_NUMBER}-errors.log}

echo "Fetching errors for PR #${PR_NUMBER}..."
echo "Output file: ${OUTPUT_FILE}"
echo ""

# Clear/create output file
> "${OUTPUT_FILE}"

{
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║              PR #${PR_NUMBER} Check Errors                            "
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Generated: $(date)"
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo ""

    # Get PR details
    echo "## PR Details"
    echo ""
    gh pr view ${PR_NUMBER} --json title,url,state,author,headRefName
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo ""

    # Get check runs
    echo "## Check Status Summary"
    echo ""
    gh pr checks ${PR_NUMBER}
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo ""

    # Get failed workflow runs
    echo "## Failed Workflow Details"
    echo ""

    # Get the SHA for this PR
    PR_SHA=$(gh pr view ${PR_NUMBER} --json headRefOid --jq '.headRefOid')
    echo "Commit SHA: ${PR_SHA}"
    echo ""

    # List all workflow runs for this commit
    echo "### Workflow Runs"
    gh run list --commit ${PR_SHA} --json databaseId,name,conclusion,status,url
    echo ""

    # Get failed runs
    FAILED_RUNS=$(gh run list --commit ${PR_SHA} --json databaseId,conclusion --jq '.[] | select(.conclusion=="failure") | .databaseId')

    if [ -z "${FAILED_RUNS}" ]; then
        echo "No failed runs found (runs may still be in progress or cancelled)"
        echo ""
    else
        for RUN_ID in ${FAILED_RUNS}; do
            echo "───────────────────────────────────────────────────────────────"
            echo "Run ID: ${RUN_ID}"
            echo ""

            # Get run details
            gh run view ${RUN_ID}
            echo ""

            echo "### Failed Jobs:"
            echo ""

            # Get failed jobs
            gh run view ${RUN_ID} --json jobs --jq '.jobs[] | select(.conclusion=="failure") | "Job: \(.name)\nSteps:\n" + (.steps[] | select(.conclusion=="failure") | "  - \(.name): \(.conclusion)\n")'
            echo ""

            echo "### Full Log:"
            echo ""
            gh run view ${RUN_ID} --log-failed || echo "Could not fetch logs (may need repo permissions)"
            echo ""
            echo "───────────────────────────────────────────────────────────────"
            echo ""
        done
    fi

    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    echo "## Summary"
    echo ""
    echo "To view PR in browser:"
    echo "  gh pr view ${PR_NUMBER} --web"
    echo ""
    echo "To view specific workflow run:"
    echo "  gh run view <RUN_ID>"
    echo ""
    echo "To view workflow logs in browser:"
    echo "  gh run view <RUN_ID> --web"
    echo ""
    echo "To re-run failed checks:"
    echo "  gh run rerun <RUN_ID> --failed"
    echo ""

} | tee "${OUTPUT_FILE}"

echo ""
echo "✓ Errors saved to: ${OUTPUT_FILE}"
echo ""
echo "View the file:"
echo "  cat ${OUTPUT_FILE}"
echo "  less ${OUTPUT_FILE}"
