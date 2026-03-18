#!/bin/bash
# Verify all unsafe blocks have preceding // SAFETY: comments.
# Exits with 1 if any unsafe block is missing a safety comment.

set -e

ERRORS=0

while IFS= read -r file; do
    while IFS= read -r line_info; do
        LINE_NUM=$(echo "$line_info" | cut -d: -f1)

        # Look up to 5 lines back for a // SAFETY: comment
        FOUND=0
        for OFFSET in 1 2 3 4 5; do
            CHECK_LINE=$((LINE_NUM - OFFSET))
            [ $CHECK_LINE -lt 1 ] && break
            CONTENT=$(sed -n "${CHECK_LINE}p" "$file")
            if echo "$CONTENT" | grep -q "// SAFETY:"; then
                FOUND=1
                break
            fi
            # Stop if we hit a non-comment, non-empty line that isn't whitespace or another comment
            TRIMMED=$(echo "$CONTENT" | sed 's/^[[:space:]]*//')
            if [ -n "$TRIMMED" ] && ! echo "$TRIMMED" | grep -q "^//"; then
                break
            fi
        done

        if [ $FOUND -eq 0 ]; then
            echo "WARNING: $file:$LINE_NUM - unsafe block without // SAFETY: comment"
            ERRORS=$((ERRORS + 1))
        fi
    done < <(grep -n "^\s*unsafe\s*{" "$file" 2>/dev/null || true)
done < <(find kernel/src -name "*.rs" -type f 2>/dev/null)

if [ $ERRORS -gt 0 ]; then
    echo ""
    echo "Found $ERRORS unsafe blocks without // SAFETY: comments"
    exit 1
fi

echo "All unsafe blocks have // SAFETY: comments"
exit 0
