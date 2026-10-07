#!/bin/sh
# Validate the XML parts of an IDML package against IDML RelaxNG compact
# schemas (the package schemas InDesign generates with generateIDMLSchema).
#
# Usage: tools/validate.sh <file.idml> <schema-dir> <jing-dir>
#   schema-dir: directory with designmap.rnc, Spreads/Spread.rnc, ...
#   jing-dir:   directory with jing.jar, isorelax.jar, saxon.jar
# The schemas and Jing are not part of this repository.
set -eu
idml=$1; schemas=$(cd "$2" && pwd); jing=$(cd "$3" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
unzip -qq "$idml" -d "$tmp"
# The generated schemas pin DOMVersion to the generating InDesign's version;
# validate structure regardless of the version written.
dom=$(grep -o 'DOMVersion="[^"]*"' "$schemas/designmap.rnc" | head -1)
[ -z "$dom" ] && dom=$(grep -o '"[0-9][0-9]*\.[0-9]"' "$schemas/designmap.rnc" | head -1 | sed 's/^/DOMVersion=/')
find "$tmp" -name '*.xml' -exec sed -i "s/DOMVersion=\"[^\"]*\"/$dom/" {} +
cp="$jing/jing.jar:$jing/isorelax.jar:$jing/saxon.jar"
status=0
check() { # schema file... (one Jing run per schema)
    schema=$1; shift
    [ $# -gt 0 ] || return 0
    if ! out=$(java -cp "$cp" com.thaiopensource.relaxng.util.Driver -c "$schemas/$schema" "$@" 2>&1); then
        status=1
    fi
    [ -n "$out" ] && printf '%s\n' "$out" | sed "s|$tmp/||"
    return 0
}
existing() { for f in "$@"; do [ -f "$f" ] && printf '%s\n' "$f"; done; }
cd "$tmp"
check designmap.rnc $(existing designmap.xml)
check Spreads/Spread.rnc $(existing Spreads/*.xml)
check MasterSpreads/MasterSpread.rnc $(existing MasterSpreads/*.xml)
check Stories/Story.rnc $(existing Stories/*.xml)
for p in Fonts Graphic Preferences Styles; do check "Resources/$p.rnc" $(existing "Resources/$p.xml"); done
check XML/BackingStory.rnc $(existing XML/BackingStory.xml)
check XML/Tags.rnc $(existing XML/Tags.xml)
exit $status
