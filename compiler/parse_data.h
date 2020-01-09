#include <vector>
#include <cstdint>
#include <string>

namespace compiler {
    struct data {
        std::vector<std::string> identifiers;
        std::vector<std::size_t> allowed_types;
        std::vector<std::size_t> type_refs;
        std::vector<std::size_t> init_signature;
    };
}