"""
Named after the Vermilion Flycatcher (Bird). Helping you catch bugs with a bird's eye view of RedLeaf ;)
"""

# *** DEFINITIONS & CONSTANTS ***

USING_HARDWARE_BREAKPOINTS = True


NEW_DOMAIN_LOADED = "redleaf_kernel::domain::domain::gdb_notify_new_domain_loaded"
LOAD_DOMAIN = "redleaf_kernel::domain::load_domain::load_domain"
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


# *** PRINTERS ***


def print_greeting():
    print("--------------------------------------------")
    print("|                Vermilion                 |")
    print("|  For RedLeaf by the Mars Research Group  |")
    print("--------------------------------------------")


def print_frame_info():
    newest_frame = gdb.newest_frame()

    print(newest_frame)
    print(newest_frame.name())
    print(newest_frame.architecture())
    print(newest_frame.type())
    print(newest_frame.pc())
    print(newest_frame.function())


# *** AUTOMATIC DOMAIN LOADING ***


def load_domain_symbol_file(name: str, text_start: int):
    file_path = f"domains/build/{name}"
    print(f"Adding Domain {file_path} @ {text_start}")

    # Command of interest: "add-symbol-file-from-memory address"
    gdb.execute(f"add-symbol-file {file_path} {text_start}", to_string=True)


def handle_stop_event(event):
    print("handle_stop_event", event)

    if isinstance(event, gdb.SignalEvent):
        # Ignore Signal Events
        return

    if USING_HARDWARE_BREAKPOINTS:
        # Look at the current frame
        frame = gdb.newest_frame()

        if frame.function().name == NEW_DOMAIN_LOADED:
            print("Load New Domain!")
            caller_frame = frame.older()

            if caller_frame.function().name == LOAD_DOMAIN:
                print(caller_frame.read_var("name").format_string())
                domain_name = caller_frame.read_var("name")
                domain_start = caller_frame.read_var("_domain_start")

                load_domain_symbol_file(domain_name, domain_start)
                gdb.post_event(DelayedExecute("continue"))
            else:
                print(
                    f"ERROR: {NEW_DOMAIN_LOADED} should not be called by {caller_frame.function()}"
                )
    else:
        if isinstance(event, gdb.BreakpointEvent):
            breakpoints = event.breakpoints

            for bp in breakpoints:
                # print(
                #     f"{bp.enabled=}, {bp.silent=}, {bp.pending=}, {bp.number=}, {bp.type=}, {bp.temporary=}, {bp.location=}"
                # )
                print(f"{bp.number=}")

                if bp.location == NEW_DOMAIN_LOADED:
                    try:
                        # Load the domain
                        frame = gdb.newest_frame()

                        caller_frame = frame.older()
                        print(
                            frame.name(),
                            caller_frame.name(),
                            caller_frame.read_var("name"),
                        )

                        domain_name = caller_frame.read_var("name")
                        domain_start = caller_frame.read_var("_domain_start")

                        load_domain_symbol_file(domain_name, domain_start)
                    except Exception as e:
                        print("Vermilion Error:", e)

        # gdb.post_event(DelayedExecute("continue"))
        # gdb.post_event(DelayedWrite("(gdc) "))


def setup_automatic_domain_loading():
    # Create the Domain Load Breakpoint
    if USING_HARDWARE_BREAKPOINTS:
        # The docs Say that I can create Hardware Breakpoints using the API, but it doesn't work
        gdb.execute(f"hbreak {NEW_DOMAIN_LOADED}", to_string=True)
        gdb.execute("commands\nsilent\nend", to_string=True)
    else:
        bp = gdb.Breakpoint(NEW_DOMAIN_LOADED, internal=True)
        bp.silent = True

    gdb.events.stop.connect(handle_stop_event)


# *** INITIALIZATION ***


def init():
    print_greeting()
    setup_automatic_domain_loading()


init()
