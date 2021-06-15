"""
RedLeaf => Color Red => Wikipedia Page for Red Colors => Salmon (as color) => Salmon Swim Upstream
"""

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
    print(gdb.lookup_symbol(
        "_binary_domains_build_redleaf_init_start"
    ))
    print(gdb.lookup_global_symbol(
        "_binary_domains_build_redleaf_init_start"
    ))
    print(gdb.lookup_static_symbol(
        "_binary_domains_build_redleaf_init_start"
    ))

init()