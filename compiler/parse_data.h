#include <vector>
#include <cstdint>
#include <string>

namespace compiler {
    struct free_function {
        std::size_t name;
        std::size_t arg_list;
        std::size_t num_args;
    };

    struct argument {
        std::size_t name;
        std::string type;
    };

    struct data {
        std::vector<std::string> identifiers;
        std::vector<argument> arguments;
        std::vector<std::size_t> allowed_types;
        std::vector<std::size_t> type_refs;        
        std::vector<std::size_t> init_signature;
    };
}