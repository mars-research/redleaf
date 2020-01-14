#include <algorithm>
#include <iterator>
#include <cstring>

#include "parse_data.h"
#include "parser.h"

namespace compiler {
    namespace {
        void dump_info(const data& data)
        {
            for (const auto& func : data.free_functions) {
                const auto name = data.identifiers.at(func.name).c_str();
                auto arg_i = std::next(data.arguments.begin(), func.arg_list);
                const auto args_end = std::next(arg_i, func.num_args);
                std::printf("Free function \"%s\" with arguments:\n", name);
                for (; arg_i < args_end; ++arg_i) {
                    const auto& arg = *arg_i;
                    const auto arg_name = data.identifiers.at(arg.name).c_str();
                    const auto arg_type = arg.type.c_str();
                    std::printf("\t\"%s\" of type %s\n", arg_name, arg_type);
                }
            }

            for (const auto& trait : data.traits) {
                const auto name = data.identifiers.at(trait.name).c_str();
                auto fn_i = std::next(data.member_functions.begin(), trait.fn_list);
                const auto fns_end = std::next(fn_i, trait.num_fns);
                std::printf("Trait \"%s\" with member functions:\n", name);
                for (; fn_i < fns_end; ++fn_i) {
                    const auto& func = *fn_i;
                    const auto name = data.identifiers.at(func.name).c_str();
                    auto arg_i = std::next(data.arguments.begin(), func.arg_list);
                    const auto args_end = std::next(arg_i, func.num_args);
                    std::printf("\t\"%s\" with arguments:\n", name);
                    for (; arg_i < args_end; ++arg_i) {
                        const auto& arg = *arg_i;
                        const auto arg_name = data.identifiers.at(arg.name).c_str();
                        const auto arg_type = arg.type.c_str();
                        std::printf("\t\t\"%s\" of type %s\n", arg_name, arg_type);
                    }
                }
            }
        }
    }
}

int main(int argc, const char** argv)
{
    if (argc < 2)
        return -1;
    
    bool dbg_enabled {false};
    auto path {argv[1]};

    if (argc == 3) {
        if (std::strcmp(argv[1], "dbg") != 0)
            return -1;
        dbg_enabled = true;
        path = argv[2];
    }

    const compiler::data* data;
    try {
        data =static_cast<const compiler::data*>(Parser::parse(std::string {path}));
    } catch (const Parser::ParseException& e) {
        std::printf("%s\n", e.getReason().c_str());
        return -1;
    }

    bool is_good {true};
    for (const std::size_t id : data->type_refs) {
        const auto type_i = std::find(data->allowed_types.begin(), data->allowed_types.end(), id);
        if (type_i == data->allowed_types.end()) {
            std::printf("Could not resolve referenced type \"%s\"\n", data->identifiers.at(id).c_str());
            is_good = false;
        }
    }

    if (!is_good)
        return -1;

    std::printf("Everything looks good!\n");

    if (!dbg_enabled)
        return 0;

    compiler::dump_info(*data);

    return 0;
}