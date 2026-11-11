#!/bin/bash
# Phase 0, third VM run: profiles of modsym and the imaginary quadratic class group with a start barrier.
# Each workload starts as this user, pinned to core 3, blocked on a FIFO; perf (system-wide on core 3) starts
# recording first, and only its child command writes the FIFO that releases the workload, then waits for it
# to exit. So sampling covers the workload's whole lifetime, every thread included. Measurement only.
set -u -o pipefail
S=$HOME/sage2-p0c
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
env SAGEBRUSH_THREADS=1 PYTHONPATH=$S/pkg python3 -c "import sagebrush"; echo "warm rc=$?"
for f in wl/*.sage; do
  w=$(basename $f .sage)
  echo "=== $w $(date -u +%FT%TZ)"
  fifo=$S/tmp/go-$w; rm -f $fifo; mkfifo $fifo
  # the workload: blocked until perf releases it (exec keeps the same pid)
  env SAGEBRUSH_THREADS=1 PYTHONPATH=$S/pkg PYTHONDONTWRITEBYTECODE=1 taskset -c $CORE bash -c 'read go < "$1"; echo "released $(date -u +%T.%N)"; exec python3 -m sagebrush "$2"' _ $fifo $f > out/D-$w.stdout 2>&1 &
  wpid=$!
  sleep 1; echo "$w blocked before perf: $(ps -o stat=,wchan= -p $wpid | tr -s ' ')"
  sudo -n perf record -a -C $CORE -F 997 -o out/D-$w.perf.data -- bash -c 'echo "perf child start $(date -u +%T.%N)"; echo go > "$1"; tail --pid="$2" -f /dev/null' _ $fifo $wpid > out/D-$w.perf.err 2>&1; prc=$?
  wait $wpid; echo "$w perf rc=$prc workload rc=$? wpid=$wpid"
  sudo -n perf report -i out/D-$w.perf.data --stdio --no-children --pid $wpid --sort dso,sym 2>/dev/null | grep -v '^#' | grep -v '^$' > out/D-$w.report.txt; echo "$w report(pid) rc=$? lines=$(wc -l < out/D-$w.report.txt)"
  sudo -n perf report -i out/D-$w.perf.data --stdio --no-children --sort comm,pid 2>/dev/null | grep -v '^#' | grep -v '^$' | head -20 > out/D-$w.comms.txt
  rm -f out/D-$w.perf.data out/D-$w.perf.data.old $fifo; echo "$w perf.data removed: $(ls out/D-$w.perf.data* 2>&1 | head -1)"
done
echo "ALL DONE $(date -u +%FT%TZ)"
