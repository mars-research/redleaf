"""
Named after the Vermilion Flycatcher (Bird). Helping you catch bugs with a bird's eye view of RedLeaf ;)
"""

import traceback
import os
import pathlib

# *** DEFINITIONS & CONSTANTS ***


NEW_DOMAIN_LOADED = "redleaf_kernel::domain::load_domain::gdb_notify_new_domain_loaded"
LOAD_DOMAIN = "redleaf_kernel::domain::load_domain::load_domain"
REDLEAF_KERNEL_ENTRY = "redleaf_kernel::rust_main"


class DelayedExecute:
    def __init__(self, command: str, to_string=False) -> None:
        self.command: str = command
        self.to_string = to_string

    def __call__(self):
        gdb.execute(self.command, to_string=self.to_string)


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


def remove_domain_symbol_file(name: str):
    """
    Removing the symbol file will set breakpoints that have been set with incorrect addresses into a pending state.
    Adding the symbol file at the correct location afterwards will fix this.
    """
    full_path = pathlib.Path(f"./domains/build/{name}").resolve()

    gdb.post_event(DelayedExecute(f"remove-symbol-file {full_path}", to_string=True))


def load_domain_symbol_file(name: str, text_start: int):
    if isinstance(text_start, gdb.Value):
        text_start = int(text_start)

    file_path = f"domains/build/{name}"
    print(f"Adding Domain {file_path} @ 0x{text_start:02X}")

    remove_domain_symbol_file(name)

    # Command of interest: "add-symbol-file-from-memory address"
    gdb.post_event(
        DelayedExecute(f"add-symbol-file {file_path} {text_start}", to_string=True)
    )


def handle_stop_event(event):
    # print("handle_stop_event", event)

    if isinstance(event, gdb.BreakpointEvent):
        breakpoints = event.breakpoints

        for bp in (b for b in breakpoints if b.is_valid()):
            # print(
            #     f"{bp.enabled=}, {bp.silent=}, {bp.pending=}, {bp.number=}, {bp.type=}, {bp.temporary=}, {bp.location=}"
            # )

            if bp.location == NEW_DOMAIN_LOADED:
                try:
                    # Load the domain
                    frame = gdb.newest_frame()

                    caller_frame = frame.older()

                    if not caller_frame.function():
                        print(
                            f"Vermilion Error: Caller Frame does not have a function! {caller_frame}"
                        )
                        gdb.post_event(DelayedExecute("bt"))

                    if caller_frame.function().name == LOAD_DOMAIN:
                        print(
                            frame.name(),
                            caller_frame.name(),
                        )
                        print(caller_frame.read_var("name"))

                        domain_name = caller_frame.read_var("name")
                        domain_start = caller_frame.read_var("_domain_start")

                        load_domain_symbol_file(domain_name, domain_start)
                        gdb.post_event(DelayedExecute("continue"))
                    else:
                        print(
                            f"Vermilion Error: {NEW_DOMAIN_LOADED} should only be called from {LOAD_DOMAIN} but was called from {caller_frame.function().name}"
                        )
                except Exception as e:
                    print("Vermilion Error:", e)
                    traceback.print_exc()

    # gdb.post_event(DelayedExecute("continue"))
    # gdb.post_event(DelayedWrite("(gdc) "))


def setup_automatic_domain_loading():
    # Create the Domain Load Breakpoint
    bp = gdb.Breakpoint(NEW_DOMAIN_LOADED, internal=True)
    bp.silent = True

    gdb.events.stop.connect(handle_stop_event)


def load_all_domains_for_autocomplete():
    """
    Adds the symbol files for every domain for autocomplete purposed. It is done without an address so any breakpoint set is incorrect.
    """
    for file in pathlib.Path("./domains/build/").iterdir():
        if file.suffix == "":
            # Add the domain
            gdb.post_event(DelayedExecute(f"add-symbol-file {file}"))


# *** INITIALIZATION ***


def init():
    print_greeting()
    setup_automatic_domain_loading()
    load_all_domains_for_autocomplete()


init()
