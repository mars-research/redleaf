#include <vector>
#include <cstdint>
#include <string>

namespace compiler {
    struct function {
        std::size_t name;
        std::size_t arg_list;
        std::size_t num_args;
    };

    struct trait {
        std::size_t name;
        std::size_t fn_list;
        std::size_t num_fns;
    };

    enum class rref_type {
        none,
        mut,
        immut,
        plain
    };

    struct argument {
        std::size_t name;
        rref_type rt;
        std::string type;
    };

    struct data {
        std::vector<std::string> identifiers;
        std::vector<argument> arguments;
        std::vector<function> free_functions;
        std::vector<function> member_functions;
        std::vector<trait> traits;
        std::vector<std::size_t> allowed_types;
        std::vector<std::size_t> type_refs;
    };
}