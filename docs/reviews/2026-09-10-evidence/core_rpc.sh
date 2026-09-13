#!/bin/bash
# usage: rpc.sh <wallet|-> <method> [json params...]
W="$1"; M="$2"; shift 2
P="[$(IFS=,; echo "$*")]"
URL="http://127.0.0.1:18899/"
[ "$W" != "-" ] && URL="${URL}wallet/$W"
curl -s -u u:p --data-binary "{\"jsonrpc\":\"1.0\",\"id\":\"probe\",\"method\":\"$M\",\"params\":$P}" -H 'content-type: text/plain;' "$URL"
echo
