# AHCI Driver

We enumare the PCI bus to [find an AHCI driver by its class](https://wiki.osdev.org/AHCI#Find_an_AHCI_controller). Then we find all the disks(ports) that don't have the [MBR magic number](https://en.wikibooks.org/wiki/X86_Assembly/Bootloaders#The_Bootsector) in it first sector so that we don't accidently override the disks that already has OSs on it. Since we only support one block device for now, our read/write operations will always go to the first disk.

# TODOs:

## Phase1: Synchronized R/W

* [X] Find an AHCI controller
* [ ] Determining what mode the controller is in
* [ ] Detect attached SATA devices
* [ ] AHCI port memory space initialization
* [ ] Read hard disk sectors
* [ ] Write hard disk sectors
