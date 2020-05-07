#!/bin/bash
typeset -i core_id
typeset -i sibling_id
typeset -i state

for i in /sys/devices/system/cpu/cpu[0-$(( $(nproc) - 1 ))]*; do
  core_id="${i##*cpu}"
  sibling_id="-1"

  if [ -f ${i}/topology/thread_siblings_list ]; then
    sibling_id="$(cut -d',' -f1 ${i}/topology/thread_siblings_list)"
  fi

  if [ $core_id -ne $sibling_id ]; then
    state="$(<${i}/online)"
    echo -n "$((1-state))" > "${i}/online"
    echo "switched ${i}/online to $((1-state))"
  fi
done

