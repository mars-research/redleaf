# for k in gdb.__dict__:
#     print(k)

bp = gdb.Breakpoint("0x1", type=gdb.BP_HARDWARE_WATCHPOINT)
