#!/bin/sh
# Validate the XML parts of IDML packages against IDML RelaxNG compact
# schemas (the package schemas InDesign generates with generateIDMLSchema).
#
# Usage: tools/validate.sh <file.idml>... <schema-dir> <jing-dir>
#   schema-dir: directory with designmap.rnc, Spreads/Spread.rnc, ...
#   jing-dir:   directory with jing.jar, isorelax.jar, saxon.jar
# The schemas and Jing are not part of this repository.
#
# Prints one line per schema error: the IDML path, a tab, then Jing's
# message (part:line:column: error: ...). Exits 1 if there are errors.
#
# Starting a JVM costs far more than validating one part, so the parts of
# all packages are validated together: one Jing run per schema and batch of
# up to 400 parts, with VALIDATE_JOBS runs in parallel (default: the
# number of processors).
set -eu
if [ $# -lt 3 ]; then
    echo "usage: $0 <file.idml>... <schema-dir> <jing-dir>" >&2
    exit 2
fi
n=$(($# - 2))
i=0
for a; do
    i=$((i + 1))
    if [ $i -eq $((n + 1)) ]; then schemas=$(cd "$a" && pwd); fi
    if [ $i -eq $((n + 2)) ]; then jing=$(cd "$a" && pwd); fi
done
jobs=${VALIDATE_JOBS:-$(nproc 2>/dev/null || echo 4)}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# Package i (1-based) is unpacked into $tmp/x/i; names maps i to its path.
i=0
for f; do
    i=$((i + 1))
    [ $i -le $n ] || break
    printf '%s\t%s\n' "$i" "$f"
done > "$tmp/names"
mkdir "$tmp/x" "$tmp/out"
# shellcheck disable=SC2016
xargs -d '\n' -P "$jobs" -n 50 sh -c '
    x=$1; shift
    tab=$(printf "\t")
    for line; do
        i=${line%%"$tab"*}; f=${line#*"$tab"}
        mkdir "$x/$i"
        unzip -qq "$f" "*.xml" -d "$x/$i" || echo "$f: cannot unpack" >&2
    done' sh "$tmp/x" < "$tmp/names"

# The generated schemas pin DOMVersion to the generating InDesign's version;
# validate structure regardless of the version written.
dom=$(grep -o 'DOMVersion="[^"]*"' "$schemas/designmap.rnc" | head -1)
[ -z "$dom" ] && dom=$(grep -o '"[0-9][0-9]*\.[0-9]"' "$schemas/designmap.rnc" | head -1 | sed 's/^/DOMVersion=/')
find "$tmp/x" -name '*.xml' -print0 |
    xargs -0 -P "$jobs" -n 500 sed -i "s/DOMVersion=\"[^\"]*\"/$dom/"

cp="$jing/jing.jar:$jing/isorelax.jar:$jing/saxon.jar"
# Lines of "schema<TAB>part" for every part that has a schema.
cd "$tmp/x"
{
    find . -mindepth 2 -maxdepth 2 -name designmap.xml | sed 's|^|designmap.rnc\t|'
    for d in Spreads:Spread MasterSpreads:MasterSpread Stories:Story; do
        find . -mindepth 3 -maxdepth 3 -path "./*/${d%%:*}/*.xml" | sed "s|^|${d%%:*}/${d#*:}.rnc\t|"
    done
    for p in Resources/Fonts Resources/Graphic Resources/Preferences Resources/Styles \
        XML/BackingStory XML/Tags; do
        find . -mindepth 3 -maxdepth 3 -path "./*/$p.xml" | sed "s|^|$p.rnc\t|"
    done
} | sed 's|\t\./|\t|' | sort > "$tmp/parts"
# One file of up to 400 parts per Jing run, all for one schema.
awk -F '\t' -v dir="$tmp/out" '
    $1 != schema || k == 400 {
        if (b) { close(dir "/batch" b ".schema"); close(dir "/batch" b ".list") }
        schema = $1; k = 0; b++; print schema > (dir "/batch" b ".schema")
    }
    { print $2 > (dir "/batch" b ".list"); k++ }' "$tmp/parts"
# shellcheck disable=SC2016
find "$tmp/out" -name '*.schema' | xargs -P "$jobs" -n 1 sh -c '
    cp=$1; schemas=$2; b=${3%.schema}
    xargs -d "\n" java -cp "$cp" com.thaiopensource.relaxng.util.Driver -c \
        "$schemas/$(cat "$3")" < "$b.list" > "$b.err" 2>&1 || true' sh "$cp" "$schemas"

# Jing names the part by its absolute path, $tmp/x/<i>/<part>; print the
# package path instead.
status=0
cat "$tmp/out"/*.err 2>/dev/null | awk -F '\t' -v names="$tmp/names" -v x="$tmp/x/" '
    BEGIN { while ((getline line < names) > 0) { split(line, f, "\t"); path[f[1]] = f[2] } }
    index($0, x) == 1 {
        rest = substr($0, length(x) + 1); i = substr(rest, 1, index(rest, "/") - 1)
        print path[i] "\t" substr(rest, length(i) + 2); next
    }
    NF { print "jing\t" $0 }' | sort -t "$(printf '\t')" -k1,1 -s > "$tmp/errors"
if [ -s "$tmp/errors" ]; then
    cat "$tmp/errors"
    status=1
fi
exit $status
