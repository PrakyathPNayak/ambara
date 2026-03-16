#!/usr/bin/env bash
set -euo pipefail

REPORT="build/env_report.json"
mkdir -p build screenshots logs

echo "{ " > $REPORT
echo "  \"rust_version\": \"$(rustc --version 2>/dev/null || echo MISSING)\"," >> $REPORT
echo "  \"cargo_version\": \"$(cargo --version 2>/dev/null || echo MISSING)\"," >> $REPORT
echo "  \"python_version\": \"$(python3 --version 2>/dev/null || echo MISSING)\"," >> $REPORT
echo "  \"node_version\": \"$(node --version 2>/dev/null || echo MISSING)\"," >> $REPORT
echo "  \"npm_version\": \"$(npm --version 2>/dev/null || echo MISSING)\"," >> $REPORT
echo "  \"scrot_available\": \"$(which scrot 2>/dev/null && echo YES || echo NO)\"," >> $REPORT
echo "  \"gnome_screenshot\": \"$(which gnome-screenshot 2>/dev/null && echo YES || echo NO)\"," >> $REPORT
echo "  \"import_available\": \"$(which import 2>/dev/null && echo YES || echo NO)\"," >> $REPORT
echo "  \"display\": \"${DISPLAY:-NONE}\"," >> $REPORT
echo "  \"xvfb\": \"$(which Xvfb 2>/dev/null && echo YES || echo NO)\"," >> $REPORT
echo "  \"pip3\": \"$(pip3 --version 2>/dev/null || echo MISSING)\"," >> $REPORT
echo "  \"curl\": \"$(which curl 2>/dev/null && echo YES || echo NO)\"," >> $REPORT
echo "  \"git_hash\": \"$(git rev-parse --short HEAD 2>/dev/null || echo NO_GIT)\"," >> $REPORT
echo "  \"os\": \"$(uname -s)\"," >> $REPORT
echo "  \"arch\": \"$(uname -m)\"" >> $REPORT
echo "}" >> $REPORT

cat $REPORT
