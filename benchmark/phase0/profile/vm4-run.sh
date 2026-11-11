#!/bin/bash
# Phase 0 on VM4 (granted 07:35Z-09:35Z): build, time parts A-C on one core, profile part D with sudo -n perf.
# Measurement only. Usage: bash run.sh SLOT   (SLOT is the granted lock name; the launcher holds it)
set -u -o pipefail
S=$HOME/sage2-p0
exec > >(tee -a $S/run.log) 2>&1
echo "START $(date -u +%FT%TZ) pgid=$(ps -o pgid= $$ | tr -d ' ') host=$(hostname)"
cd $S || exit 2
export CARGO_TARGET_DIR=$S/tgt TMPDIR=$S/tmp
mkdir -p $S/tmp $S/out
CORE=3
rc_all=0
step() { echo "=== $1 $(date -u +%FT%TZ)"; }
step build-A; (cd dashu/benchmark/phase0/kernels && cargo build --release 2>&1 | tail -2); a=$?; echo "rc=$a"; rc_all=$((rc_all|a))
step build-B; (cd dashu/benchmark/phase0/batched && cargo build --release 2>&1 | tail -2); b=$?; echo "rc=$b"; rc_all=$((rc_all|b))
step build-C; (cd dashu/benchmark/phase0/boundary/dashu-radix && cargo build --release 2>&1 | tail -2); c=$?; echo "rc=$c"; rc_all=$((rc_all|c))
step build-py; (cd sagebrush/engine && CARGO_PROFILE_RELEASE_DEBUG=line-tables-only CARGO_PROFILE_RELEASE_STRIP=none cargo build --release -p sagebrush-py --features pyo3/abi3-py310 2>&1 | tail -2); d=$?; echo "rc=$d"; rc_all=$((rc_all|d))
rm -rf pkg && mkdir pkg && cp -r sagebrush/engine/py/python/sagebrush pkg/ && rm -f pkg/sagebrush/_native*.so && cp $S/tgt/release/libsagebrush_native.so pkg/sagebrush/_native.abi3.so
sha256sum $S/tgt/release/phase0-kernels $S/tgt/release/phase0-batched $S/tgt/release/phase0-radix pkg/sagebrush/_native.abi3.so bin/* > out/SHA256SUMS-binaries
step timings
taskset -c $CORE $S/tgt/release/phase0-kernels > out/A-native-dashu.txt; echo "A dashu rc=$?"
taskset -c $CORE bin/p0-gmp-asm > out/A-native-gmp-asm.txt; echo "A gmp-asm rc=$?"
taskset -c $CORE bin/p0-gmp-gen > out/A-native-gmp-generic.txt; echo "A gmp-gen rc=$?"
taskset -c $CORE $S/tgt/release/phase0-batched > out/B-native-sagebrush.txt; echo "B sagebrush rc=$?"
taskset -c $CORE bin/p0b-gmp-asm > out/B-native-gmp-asm.txt; echo "B gmp-asm rc=$?"
taskset -c $CORE bin/p0b-gmp-gen > out/B-native-gmp-generic.txt; echo "B gmp-gen rc=$?"
taskset -c $CORE $S/tgt/release/phase0-radix > out/C-native-dashu-radix.txt; echo "C dashu rc=$?"
taskset -c $CORE node dashu/benchmark/phase0/boundary/v8-bigint.mjs > out/C-node-v8-bigint.txt; echo "C v8 rc=$?"
step profiles
for f in wl/*.sage; do
  w=$(basename $f .sage)
  /usr/bin/time -f "$w wall %e s maxrss %M KB" env SAGEBRUSH_THREADS=1 PYTHONPATH=$S/pkg taskset -c $CORE python3 -m sagebrush $f > out/D-$w.stdout 2> out/D-$w.time; echo "$w plain rc=$? $(tail -1 out/D-$w.time)"
  # (the workload runs as this user; perf attaches to its pid, so nothing of the workload runs as root)
  env SAGEBRUSH_THREADS=1 PYTHONPATH=$S/pkg PYTHONDONTWRITEBYTECODE=1 taskset -c $CORE python3 -m sagebrush $f > /dev/null 2>&1 &
  wpid=$!
  sudo -n perf record -N -F 997 -p $wpid -o out/D-$w.perf.data > out/D-$w.perf.err 2>&1; prc=$?
  wait $wpid; echo "$w perf rc=$prc workload rc=$?"
  sudo -n perf report -i out/D-$w.perf.data --stdio --no-children --sort dso,sym 2>/dev/null | grep -v '^#' | grep -v '^$' > out/D-$w.report.txt; echo "$w report rc=$? lines=$(wc -l < out/D-$w.report.txt)"
  rm -f out/D-$w.perf.data out/D-$w.perf.data.old; echo "$w perf.data removed: $(ls out/D-$w.perf.data* 2>&1 | head -1)"
done
ls -la out | head -60
echo "ALL DONE $(date -u +%FT%TZ) rc_build=$rc_all"
