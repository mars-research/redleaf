"""
Named after the Vermilion Flycatcher (Bird). Helping you catch bugs with a bird's eye view of RedLeaf ;)
"""

from typing import Dict, List
from dataclasses import dataclass
import re


@dataclass
class GDBDomain:
    name: str
    offset: int
    entry_point: int

# Maps domain name (e.x. pci to text addr)
ACTIVE_DOMAINS: Dict[str, str] = {}
DOMAIN_TEXT_REGEX = r"cpu\(0\):domain/(\w+): .text starts at ([0-9a-fA-F]+)"
GDB_DOMAIN_REGEX = r'GDBDomain {\n\s*name: \"(?P<name>\w+)\",\n\s*offset: (?P<offset>\w+),\n\s*entry_point: (?P<entry_point>\w+),\n\s*},'


def print_greeting():
    print("--------------------------------------------")
    print("|                Vermilion                 |")
    print("|  For RedLeaf by the Mars Research Group  |")
    print("--------------------------------------------")

def parse_loaded_domain_str(s: str):
    """
    [
        GDBDomain {
            name: "init",
            offset: 0x1028c0000,
            entry_point: 0x1028c2b90,
        },
        GDBDomain {
            name: "dom_proxy",
            offset: 0x102d00000,
            entry_point: 0x102d00000,
        },
        GDBDomain {
            name: "tpm",
            offset: 0x102d80000,
            entry_point: 0x102d81cb0,
        },
    ]
    """
    matches = re.findall(GDB_DOMAIN_REGEX, s)

    return [
        GDBDomain(
            name=match[0],
            offset=int(match[1], 16),
            entry_point=int(match[2], 16)
        ) for match in matches
    ]


def get_loaded_domains():
    """
    Uses redleaf_kernel::domain::domain::get_loaded_domains_as_string to figure out which domains have been loaded by the Kernel.
    (I think calling this function leaks memory because the String object is never Dropped by GDB)
    """

    loaded_domains: str = gdb.execute(
        r'printf "%s\n", redleaf_kernel::domain::domain::get_loaded_domains_as_string().vec.buf.ptr.pointer',
        to_string=True,
    )

    # print("AS STR:", loaded_domains)
    # print("PARSED:", parse_loaded_domain_str(loaded_domains))

    return parse_loaded_domain_str(loaded_domains)


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
                    print(
                        f"WARNING: THE FOLLOWING LINE IS INCOMPATIBLE WITH DOMAIN LOADING REGEX: {line}"
                    )

    return found_domains


def add_symbol_file_for_domain(domain: str, text_start: str):
    print(f"Adding Domain {domain} @ {text_start}")
    file_path = f"domains/build/{domain}"

    gdb.execute(f"add-symbol-file {file_path} {text_start}")


def add_domain_symbol_files(domains_to_load: List[GDBDomain]):
    for domain in domains_to_load:
        add_symbol_file_for_domain(domain.name, domain.offset)

def get_gdb_loaded_domains():
    """
    Returns the names of the domains already loaded by gdb
    """

    DOMAIN_NAME_REGEX = r"\/domains\/build\/(\w+)"

    res = set()

    for objfile in gdb.selected_inferior().progspace.objfiles():
        if objfile.is_valid():
            match = re.search(DOMAIN_NAME_REGEX, objfile.filename)
            if match:
                res.add(match.groups()[0])

    return res

def add_load_domain_breakpoint():
    gdb.Breakpoint(r"redleaf_kernel::domain::domain::gdb_helper_new_domain_loaded", internal=True)

def break_point_handler(event):
    print(event)
    print(event.breakpoints)

    for bp in event.breakpoints:
        print(bp.location, bp.expression, bp.commands, bp.silent)

def event_handler(event):
    print("STOP EVENT:", event)

    if isinstance(event, gdb.StopEvent):
        pass
        # Run our helpers
        print("IS INSTANCE OF STOP EVENT")
        loaded_domains = get_loaded_domains()
        gdb_loaded_domains = get_gdb_loaded_domains()

        print(loaded_domains, gdb_loaded_domains)

        # Only load domains that haven't been loaded
        domains_to_load = [domain for domain in loaded_domains if domain.name not in gdb_loaded_domains]

        add_domain_symbol_files(domains_to_load)


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


def init():
    print_greeting()
    _add_all_event_listeners()
    add_load_domain_breakpoint()
    gdb.events.stop.connect(break_point_handler)
    # gdb.events.stop.connect(event_handler)


init()
