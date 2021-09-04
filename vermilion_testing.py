import os

NEW_DOMAIN_LOADED_BREAKPOINT = (
    "redleaf_kernel::domain::domain::gdb_notify_new_domain_loaded"
)
REDLEAF_KERNEL_ENTRY = "redleaf_kernel::rust_main"


class DelayedExecute:
    def __init__(self, command: str) -> None:
        self.command: str = command

    def __call__(self):
        gdb.execute(self.command)


class DelayedWrite:
    def __init__(self, string: str) -> None:
        self.string: str = string

    def __call__(self):
        gdb.write(self.string)
        gdb.flush()


# DelayedFlush = lambda : gdb.flush()


def load_domain_symbol_file(name: str, text_start: int):
    file_path = f"domains/build/{name}"
    # print(f"Adding Domain {file_path} @ {text_start}")

    # Command of interest: "add-symbol-file-from-memory address"
    gdb.execute(f"add-symbol-file {file_path} {text_start}", to_string=True)


def handle_stop_event(event):
    print(event, type(event), isinstance(event, gdb.BreakpointEvent))
    print(event.breakpoints)

    if isinstance(event, gdb.BreakpointEvent):
        breakpoints = event.breakpoints

        for bp in breakpoints:
            if bp.location == NEW_DOMAIN_LOADED_BREAKPOINT:
                try:
                    # Load the domain
                    frame = gdb.newest_frame()

                    caller_frame = frame.older()
                    print(
                        frame.name(), caller_frame.name(), caller_frame.read_var("name")
                    )

                    domain_name = caller_frame.read_var("name")
                    domain_start = caller_frame.read_var("_domain_start")

                    load_domain_symbol_file(domain_name, domain_start)
                except Exception as e:
                    print(e)

    gdb.post_event(DelayedExecute("continue"))
    # gdb.post_event(DelayedWrite("(gdc) "))


# for k in gdb.__dict__["breakpoints"]:
#     print(k)

# Create the Domain Load Breakpoint
bp = gdb.Breakpoint(NEW_DOMAIN_LOADED_BREAKPOINT, internal=True)
bp.silent = True

# Necessary for breakpoints to work with kvm enabled
print(
    "*** Vermilion ***: Breakpoint 1 is necessary for domain loading to function with KVM Enabled"
)
gdb.execute(f"hbreak {NEW_DOMAIN_LOADED_BREAKPOINT}", to_string=True)


gdb.events.stop.connect(handle_stop_event)

# Add in symbols with no location
# gdb.execute("add-symbol-file domains/build/ixgbe")
# gdb.execute("add-symbol-file domains/build/virtio_net")
# gdb.execute("add-symbol-file domains/build/pci")


def _add_all_event_listeners():
    """
    Helpful for debugging purposes
    """

    EVENT_TYPES = (
        "stop",
        "cont",
        "exited",
        "new_objfile",
        "clear_objfiles",
        "new_inferior",
        "inferior_deleted",
        "new_thread",
        "inferior_call",
        "memory_changed",
        "register_changed",
        "breakpoint_created",
        "breakpoint_deleted",
        "breakpoint_modified",
        "before_prompt",
    )

    for event_type in EVENT_TYPES:
        # print(event_type)
        gdb.events.__dict__[event_type].connect(print)
