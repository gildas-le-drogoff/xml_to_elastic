#!/usr/bin/env bash

set -euo pipefail

DOSSIER_XML="${1:-TA}"
INDEX="decisions"
BULK="bulk_all.json"
PREFIX="bulk_part_"
ES="http://localhost:9200"

command -v cargo > /dev/null || exit 1
command -v split > /dev/null || exit 1
command -v curl  > /dev/null || exit 1
command -v jq    > /dev/null || exit 1
command -v od    > /dev/null || exit 1

echo "BUILD"
cargo fmt
cargo build --release

echo "GENERATION BULK"
./target/release/xml_to_elastic "$DOSSIER_XML" "$BULK"
[ -s "$BULK" ] || exit 1

echo "SPLIT"
rm -f ${PREFIX}*
split -l 20000 "$BULK" "$PREFIX"
ls -lh ${PREFIX}*

echo "VERIFY NEWLINE"
for f in ${PREFIX}*; do
    BYTE=$(
        tail -c 1 "$f" |
        od -An -t x1 |
        tr -d ' '
    )

    [ "$BYTE" = "0a" ] || exit 1
done

echo "DELETE INDEX"
curl -s -X DELETE "$ES/$INDEX" > /dev/null || true

echo "BULK INDEX"

TOTAL=0

for f in ${PREFIX}*; do
    echo "$f"

    curl -s \
        -H "Content-Type: application/x-ndjson" \
        -X POST "$ES/_bulk" \
        --data-binary "@$f" |
        jq -e '.errors == false' > /dev/null

    COUNT=$(wc -l < "$f")
    COUNT=$((COUNT / 2))

    TOTAL=$((TOTAL + COUNT))
done

echo "VERIFY COUNT"

ES_COUNT=$(curl -s "$ES/$INDEX/_count" | jq '.count')

echo "EXPECTED $TOTAL"
echo "REAL     $ES_COUNT"

[ "$TOTAL" = "$ES_COUNT" ] || exit 1

echo
echo "OK"
echo "$ES/$INDEX"
echo
