#!/bin/sh

export NOMAD_ADDR=http://localhost:8086

echo
echo "> Pre-start GC"
nomad system gc
sleep 2

echo "> Purging old jobs"
curl http://localhost:8086/v1/jobs | jq -r '. | map(select(.Name | startswith("test-param")).ID) | .[]' | xargs -L 1 -d'\n' nomad job stop -yes -purge -detach

echo
echo "> GCing purged jobs"
nomad system gc


echo
echo "> Submitting job"
nomad run job-param.nomad

# Continuous load
# echo
# echo "> Running continuous"
# while true; do
#     nomad job dispatch -detach -meta X=abc test-param &
#     sleep 1
# done

# Run many at once
# NOTE: We don't run GC on purpose to see how that increases the time
for iter in {1..500}; do
    echo
    echo "> Running batch $iter"
    date
    for i in {1..500}; do
        nomad job dispatch -detach -meta X=$i test-param > /dev/null &
    done
    wait

    # Wait for jobs to finish
    echo
    echo "> Waiting for completion"
    time sh << EOF
    until curl -s "http://127.0.0.1:8086/v1/jobs?prefix=test-param/dispatch-" | jq -e '. | all(.Status == "dead")' > /dev/null; do
        sleep 2
    done
EOF
done

echo 'Done'

