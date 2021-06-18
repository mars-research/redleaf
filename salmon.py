"""
RedLeaf => 
Color Red => 
Wikipedia Page for Red Colors => 
Salmon (as color) => 
Salmon Swim Upstream => 
Relatable while working on RedLeaf
"""

from typing import Dict
import re

# Maps domain name (e.x. pci to text addr)
ACTIVE_DOMAINS: Dict[str, str] = {}
DOMAIN_TEXT_REGEX = r"cpu\(0\):domain/(\w+): .text starts at ([0-9a-fA-F]+)"


def print_greeting():
    print("-------------------------------")
    print("|      Hello From Salmon      |")
    print("|  Always Swimming Up Stream  |")
    print("-------------------------------")


def print_frame_info():
    newest_frame = gdb.newest_frame()

    print(newest_frame)
    print(newest_frame.name())
    print(newest_frame.architecture())
    print(newest_frame.type())
    print(newest_frame.pc())
    print(newest_frame.function())

def parse_log_for_loaded_domains():
    found_domains = {}

    with open("serial.log", "r") as file:
        for line in file:
            # print(line)
            match = re.match(DOMAIN_TEXT_REGEX, line)

            if match != None:
                groups = match.groups()

                if len(groups) == 2:
                    domain_name = groups[0]
                    text_start = groups[1]

                    found_domains[domain_name] = text_start
                else:
                    print(f"WARNING: THE FOLLOWING LINE IS INCOMPATIBLE WITH DOMAIN LOADING REGEX: {line}")

    return found_domains

def add_symbol_file_for_domain(domain: str, text_start: str):
    print(f"Adding Domain {domain} @ {text_start}")
    file_path = f"domains/build/{domain}"

    gdb.execute(f"add-symbol-file {file_path} 0x{text_start}")

def add_domain_symbol_files():
    found_domains = parse_log_for_loaded_domains()
    print(f"{found_domains=}")
    domains_to_load = [domain for domain in found_domains if domain not in ACTIVE_DOMAINS]

    for domain in domains_to_load:
        add_symbol_file_for_domain(domain, found_domains[domain])

def event_handler(event):
    print("STOP EVENT:", event)

    if isinstance(event, gdb.StopEvent):
        pass
        # Run our helpers
        print("IS INSTANCE OF STOP EVENT")
        add_domain_symbol_files()

def init():
    print_greeting()

    print("OBJFILES:")
    for file in gdb.objfiles():
        print(file)

    # print(gdb.selected_thread())
    # print(gdb.current_progspace().__dict__)

    """
    These Symbols 100% exist because I can find them in gdb just fine. 

    >> (gdb) info address _binary_domains_build_redleaf_init_start
    >> Symbol "_binary_domains_build_redleaf_init_start" is at 0x18026c in a file compiled without debugging.

    Looks like we'll be doing lots of parsing :sigh:
    """

    # print(gdb.lookup_symbol(
    #     "_binary_sys_init_build_init_start"
    # ))
    # print(gdb.lookup_symbol(
    #     "_binary_domains_build_redleaf_init_start"
    # ))
    # print(gdb.lookup_global_symbol(
    #     "_binary_domains_build_redleaf_init_start"
    # ))
    # print(gdb.lookup_static_symbol(
    #     "_binary_domains_build_redleaf_init_start"
    # ))

    # print("Available Events:", gdb.events.__dict__.keys())

    EVENT_TYPES = (
        'stop', 
        'cont', 
        'exited', 
        'new_objfile', 
        'clear_objfiles', 
        'new_inferior', 
        'inferior_deleted', 
        'new_thread', 
        'inferior_call', 
        'memory_changed', 
        'register_changed', 
        'breakpoint_created', 
        'breakpoint_deleted', 
        'breakpoint_modified', 
        'before_prompt'
    )

    for event_type in EVENT_TYPES:
        # print(event_type)
        gdb.events.__dict__[event_type].connect(print)

    # Register our stop handler
    gdb.events.stop.connect(event_handler)

    # for event_type in EVENT_TYPES:
    #     print(event_type)
    #     gdb.events.__dict__[event_type].disconnect(event_handler)
    # gdb.events.exited.connect(event_handler)

init()