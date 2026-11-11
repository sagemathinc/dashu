#!/bin/bash
# Phase 0, second VM run: profiles of modsym and the imaginary quadratic class group, with perf sampling core 3
# for the workload's whole lifetime (-a -C 3), so threads started later are caught. Measurement only.
set -u -o pipefail
S=$HOME/sage2-p0b
exec > >(tee -a $S/run.log) 2>&1
echo "START $(date -u +%FT%TZ) pgid=$(ps -o pgid= $$ | tr -d ' ') host=$(hostname)"
cd $S || exit 2
export CARGO_TARGET_DIR=$S/tgt TMPDIR=$S/tmp
mkdir -p $S/tmp $S/out
CORE=3
echo "=== build-py $(date -u +%FT%TZ)"
(cd sagebrush/engine && CARGO_PROFILE_RELEASE_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_STRIP=none cargo build --release -p sagebrush-py --features pyo3/abi3-py310 2>&1 | tail -2); echo "build rc=$?"
rm -rf pkg && mkdir pkg && cp -r sagebrush/engine/py/python/sagebrush pkg/ && rm -f pkg/sagebrush/_native*.so && cp $S/tgt/release/libsagebrush_native.so pkg/sagebrush/_native.abi3.so
sha256sum pkg/sagebrush/_native.abi3.so > out/SHA256SUMS-binaries
# warm the bytecode cache as this user, so the profiled runs write nothing
env SAGEBRUSH_THREADS=1 PYTHONPATH=$S/pkg python3 -c "import sagebrush" ; echo "warm rc=$?"
for f in wl/*.sage; do
  w=$(basename $f .sage)
  echo "=== $w $(date -u +%FT%TZ)"
  env SAGEBRUSH_THREADS=1 PYTHONPATH=$S/pkg PYTHONDONTWRITEBYTECODE=1 taskset -c $CORE python3 -m sagebrush $f > out/D-$w.stdout 2>&1 &
  wpid=$!
  sudo -n perf record -a -C $CORE -F 997 -o out/D-$w.perf.data -- tail --pid=$wpid -f /dev/null > out/D-$w.perf.err 2>&1; prc=$?
  wait $wpid; echo "$w perf rc=$prc workload rc=$? wpid=$wpid"
  sudo -n perf report -i out/D-$w.perf.data --stdio --no-children --pid $wpid --sort dso,sym 2>/dev/null | grep -v '^#' | grep -v '^$' > out/D-$w.report.txt; echo "$w report(pid) rc=$? lines=$(wc -l < out/D-$w.report.txt)"
  sudo -n perf report -i out/D-$w.perf.data --stdio --no-children --sort comm,pid 2>/dev/null | grep -v '^#' | grep -v '^$' | head -20 > out/D-$w.comms.txt
  rm -f out/D-$w.perf.data out/D-$w.perf.data.old; echo "$w perf.data removed: $(ls out/D-$w.perf.data* 2>&1 | head -1)"
done
echo "ALL DONE $(date -u +%FT%TZ)"
