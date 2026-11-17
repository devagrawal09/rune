#!/bin/sh
# per-op instructions: (Ir at 2N - Ir at N) / N, so mount and teardown cancel
bin=$1; w=$2; n=$3
a=$(valgrind --tool=callgrind --callgrind-out-file=/dev/null $bin $w $n 2>&1 | sed -n 's/.*Collected : //p')
b=$(valgrind --tool=callgrind --callgrind-out-file=/dev/null $bin $w $((2*n)) 2>&1 | sed -n 's/.*Collected : //p')
echo $(( (b - a) / n ))
