#include <algorithm>
#include <iterator>
#include <cstring>
#include <fstream>

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
                    std::printf("\t\"%s\" of type %s, is_rref = %s\n", arg_name, arg_type, arg.is_rref ? "true" : "false");
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
                        std::printf("\t\t\"%s\" of type %s, is_rref = %s\n", arg_name, arg_type, arg.is_rref ? "true" : "false");
                    }
                }
            }
        }

        auto get_proxy_name(const std::string& trait_name)
        {
            const auto trait_root = trait_name.substr(0, trait_name.find("Interface"));
            std::vector<char> buffer;
            buffer.reserve(256);
            bool past_first {false};
            for (const char c : trait_root) {
                if (std::isupper(c)) {
                    if (past_first)
                        buffer.push_back('_');
                    past_first = true;
                }
                buffer.push_back(std::toupper(c));
            }

            return std::string {buffer.begin(), buffer.end()};
        }

        void generate_proxies(const data& data, const char* path)
        {
            std::ofstream file {path};
            for (const auto& func : data.free_functions) {
                const auto name = data.identifiers.at(func.name).c_str();
                auto arg_i = std::next(data.arguments.begin(), func.arg_list);
                const auto args_end = std::next(arg_i, func.num_args - 1);
                const auto ret_val = *args_end;
                std::vector<argument> args(func.num_args - 1);
                std::copy(arg_i, args_end, args.begin());

                file << "fn " << name << "(";

                bool past_first_arg = false;
                for (const auto& arg : args) {
                    if (past_first_arg)
                        file << ", ";
                    file << data.identifiers.at(arg.name) << ": " << arg.type;
                    past_first_arg = true;
                }

                file << ") ";
                if (ret_val.type != "void")
                    file << "-> " << ret_val.type << " ";
                file << "{\n";
                file << "\tlet old_domain_id = GET_CALLER_DOMAIN_ID();\n";
                file << "\tlet new_domain_id = GET_CALLEE_DOMAIN_ID();\n";
                file << "\tRECORD_IP();\n\tRECORD_SP();\n";

                for (const auto& arg : args) {
                    if (arg.is_rref)
                        file << "\t" << data.identifiers.at(arg.name) << ".move_to(new_domain_id);\n";
                }

                file << "\tlet ret = " << name << "(";

                past_first_arg = false;
                for (const auto& arg : args) {
                    if (past_first_arg)
                        file << ", ";
                    file << data.identifiers.at(arg.name);
                    past_first_arg = true;
                }

                file << ");\n";

                if (ret_val.is_rref)
                    file << "\tret.move_to(new_domain_id);\n";

                file << "\treturn ret;\n}\n\n";
            }
            
            for (const auto& trait : data.traits) {
                const auto trait_name = data.identifiers.at(trait.name).c_str();
                auto fn_i = std::next(data.member_functions.begin(), trait.fn_list);
                const auto fns_end = std::next(fn_i, trait.num_fns);
                
                file << "impl " << trait_name << " for Proxy {\n";

                for (; fn_i < fns_end; ++fn_i) {
                    const auto& func = *fn_i;
                    const auto name = data.identifiers.at(func.name).c_str();
                    auto arg_i = std::next(data.arguments.begin(), func.arg_list);
                    const auto args_end = std::next(arg_i, func.num_args - 1);
                    const auto ret_val = *args_end;
                    std::vector<argument> args(func.num_args - 1);
                    std::copy(arg_i, args_end, args.begin());

                    file << "fn " << name << "(self&";

                    for (const auto& arg : args) {
                        file << ", ";
                        file << data.identifiers.at(arg.name) << ": " << arg.type;
                    }

                    file << ") ";
                    if (ret_val.type != "void")
                        file << "-> " << ret_val.type << " ";
                    file << "{\n";
                    file << "\tlet old_domain_id = GET_CALLER_DOMAIN_ID();\n";
                    file << "\tlet new_domain_id = GET_CALLEE_DOMAIN_ID();\n";
                    file << "\tRECORD_IP();\n\tRECORD_SP();\n";

                    for (const auto& arg : args) {
                        if (arg.is_rref)
                            file << "\t" << data.identifiers.at(arg.name) << ".move_to(new_domain_id);\n";
                    }

                    file << "\tlet ret = " << get_proxy_name(trait_name) << "." << name << "(";

                    bool past_first_arg = false;
                    for (const auto& arg : args) {
                        if (past_first_arg)
                            file << ", ";
                        file << data.identifiers.at(arg.name);
                        past_first_arg = true;
                    }

                    file << ");\n";

                    if (ret_val.is_rref)
                        file << "\tret.move_to(new_domain_id);\n";

                    file << "\treturn ret;\n}\n\n";
                }

                file << "}\n";
            }
        }
    }
}

int main(int argc, const char** argv)
{
    if (argc < 3)
        return -1;
    
    bool dbg_enabled {false};
    auto path {argv[1]};
    auto proxy_path {argv[2]};

    if (argc == 4) {
        if (std::strcmp(argv[1], "dbg") != 0)
            return -1;
        dbg_enabled = true;
        path = argv[2];
        proxy_path = argv[3];
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

    compiler::generate_proxies(*data, proxy_path);

    if (!dbg_enabled)
        return 0;

    compiler::dump_info(*data);

    return 0;
}