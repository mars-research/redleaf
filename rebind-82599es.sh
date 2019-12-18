#!/bin/sh

TARGET_DRIVER=vfio-pci
LOCATION=$(lspci | grep 82599ES | head -1 | awk '{print $1}')

modprobe ${TARGET_DRIVER}
if [ $? -ne 0 ]; then
	>&2 echo "Failed to probe ${TARGET_DRIVER}"
	exit 2
fi

if [ -z ${LOCATION} ]; then
	>&2 echo "No 82599ES device found! Exiting..."
	exit 1
fi

SYSFS_DEVICE="0000:${LOCATION}"
SYSFS_DEVICE_PATH="/sys/bus/pci/devices/${SYSFS_DEVICE}"
VENDOR=$(cat "${SYSFS_DEVICE_PATH}/vendor" | sed 's/\x/ /' | awk '{print $2}')
DEVICE=$(cat "${SYSFS_DEVICE_PATH}/device" | sed 's/\x/ /' | awk '{print $2}')
OLDDRV=$(basename $(realpath ${SYSFS_DEVICE_PATH}/driver))

>&2 echo "Found 82599ES at ${LOCATION} (${VENDOR} ${DEVICE}) bound to ${OLDDRV}"

if [ "${OLDDRV}" = "${TARGET_DRIVER}" ]; then
	>&2 echo "Device already bound to ${TARGET_DRIVER}!"
else
	>&2 echo "Rebinding the device..."
	echo "${VENDOR} ${DEVICE}" > /sys/bus/pci/drivers/${TARGET_DRIVER}/new_id
	echo "${SYSFS_DEVICE}" > /sys/bus/pci/drivers/${OLDDRV}/unbind
	echo "${SYSFS_DEVICE}" > /sys/bus/pci/drivers/${TARGET_DRIVER}/bind

	NEWDRV=$(basename $(realpath ${SYSFS_DEVICE_PATH}/driver))
	if [ "${NEWDRV}" != "${TARGET_DRIVER}" ]; then
		>&2 echo "Failed to rebind device. See errors above."
		exit 3
	fi
	>&2 echo "Success!"
fi

echo "-device vfio-pci,romfile=,host=${LOCATION}"
