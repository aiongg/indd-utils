#!/bin/sh
# Validate the XML parts of an IDML package against IDML RelaxNG compact
# schemas (the package schemas InDesign generates with generateIDMLSchema).
#
# Usage: tools/validate.sh <file.idml> <schema-dir> <jing-dir>
#   schema-dir: directory with designmap.rnc, Spreads/Spread.rnc, ...
#   jing-dir:   directory with jing.jar, isorelax.jar, saxon.jar
# The schemas and Jing are not part of this repository.
set -eu
idml=$1; schemas=$2; jing=$3
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
check() { # file schema
    [ -f "$tmp/$1" ] || return 0
    if ! out=$(java -cp "$cp" com.thaiopensource.relaxng.util.Driver -c "$schemas/$2" "$tmp/$1" 2>&1); then
        status=1
    fi
    [ -n "$out" ] && printf '%s\n' "$out" | sed "s|$tmp/||"
    return 0
}
check designmap.xml designmap.rnc
for f in "$tmp"/Spreads/*.xml; do [ -e "$f" ] && check "Spreads/$(basename "$f")" Spreads/Spread.rnc; done
for f in "$tmp"/MasterSpreads/*.xml; do [ -e "$f" ] && check "MasterSpreads/$(basename "$f")" MasterSpreads/MasterSpread.rnc; done
for f in "$tmp"/Stories/*.xml; do [ -e "$f" ] && check "Stories/$(basename "$f")" Stories/Story.rnc; done
for p in Fonts Graphic Preferences Styles; do check "Resources/$p.xml" "Resources/$p.rnc"; done
check XML/BackingStory.xml XML/BackingStory.rnc
check XML/Tags.xml XML/Tags.rnc
exit $status
