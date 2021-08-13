

# NEW_DOMAIN_LOADED_BREAKPOINT = "redleaf_kernel::domain::domain::gdb_notify_new_domain_loaded"
NEW_DOMAIN_LOADED_BREAKPOINT = "redleaf_kernel::domain::domain::gdb_notify_new_domain_loaded:1"

# gdb.events.stop.connect(print)
# gdb.events.breakpoint_created.connect(print)
# gdb.events.breakpoint_modified.connect(print)
# gdb.events.breakpoint_deleted.connect(print)

# Create the Breakpoints
bp = gdb.Breakpoint(NEW_DOMAIN_LOADED_BREAKPOINT, internal=True)
bp.silent = True


def load_domain_symbol_file(name: str, text_start: int):
    print(f"Adding Domain {name} @ {text_start}")
    file_path = f"domains/build/{name}"

    gdb.execute(f"add-symbol-file {file_path} {text_start}", to_string=True)


def breakpoint_event_handler(event):
    print(event)

    if isinstance(event, gdb.BreakpointEvent):
        print("Breakpoint Event", event.breakpoints)
        breakpoints = event.breakpoints

        for bp in breakpoints:
            if bp.location == NEW_DOMAIN_LOADED_BREAKPOINT:
                # Load the domain
                print("Load New Domain")
                frame = gdb.newest_frame()

                print(frame.name())
                caller_frame = frame.older()
                print(caller_frame.name(), caller_frame.read_var("name"))

                domain_name = caller_frame.read_var("name")
                domain_start = caller_frame.read_var("_domain_start")

                load_domain_symbol_file(domain_name, domain_start)


gdb.events.stop.connect(breakpoint_event_handler)


