#include <algorithm>
#include <array>
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
                    std::printf(
                        "\t\"%s\" of type %s, rt = %s\n",
                        arg_name,
                        arg_type,
                        [=] {
                            switch (arg.rt) {
                            case rref_type::none:
                                return "none";
                                
                            case rref_type::mut:
                                return "mut";

                            case rref_type::immut:
                                return "immut";
                            }
                        }()
                    );
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
                        std::printf(
                            "\t\t\"%s\" of type %s, rt = %s\n",
                            arg_name,
                            arg_type,
                            [=] {
                                switch (arg.rt) {
                                case rref_type::none:
                                    return "none";
                                    
                                case rref_type::mut:
                                    return "mut";

                                case rref_type::immut:
                                    return "immut";

                                case rref_type::plain:
                                    return "plain";
                                }
                            }()
                        );
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

        void generate_sys_call_args(const data& data, const std::vector<argument>& args, std::ofstream& file)
        {
            bool past_first_arg = false;
            for (const auto& arg : args) {
                if (past_first_arg)
                    file << ", ";
                file << data.identifiers.at(arg.name) << ": ";
                switch (arg.rt) {
                case rref_type::none:
                    break;
                    
                case rref_type::mut:
                    file << "&mut ";
                    break;

                case rref_type::immut:
                    file << "&";
                    break;
                }
                if (arg.rt == rref_type::none)
                    file << arg.type;
                else
                    file << "RRef<" << arg.type << ">";
                past_first_arg = true;
            }
        }

        void generate_real_call_args(const data& data, const std::vector<argument>& args, std::ofstream& file)
        {
            bool past_first_arg = false;
            for (const auto& arg : args) {
                if (past_first_arg)
                    file << ", ";
                file << data.identifiers.at(arg.name);
                past_first_arg = true;
            }
        }

        struct arg_range {
            std::vector<argument>::const_iterator begin;
            std::vector<argument>::const_iterator end;
        };

        arg_range get_func_args(const data& data, function func)
        {
            auto arg_i = std::next(data.arguments.begin(), func.arg_list);
            const auto args_end = std::next(arg_i, func.num_args - 1);
            return {arg_i, args_end};
        }

        void generate_sys_calls(const data& data, std::ofstream& file)
        {
            for (const auto& trait : data.traits) {
                const auto trait_name = data.identifiers.at(trait.name);
                auto fn_i = std::next(data.member_functions.begin(), trait.fn_list);
                const auto fns_end = std::next(fn_i, trait.num_fns);

                for (; fn_i < fns_end; ++fn_i) {
                    const auto& func = *fn_i;
                    const auto name = data.identifiers.at(func.name).c_str();
                    const auto arg_list = get_func_args(data, func);
                    const auto ret_val = *arg_list.end;
                    std::vector<argument> args(func.num_args - 1);
                    std::copy(arg_list.begin, arg_list.end, args.begin());

                    std::array<char, 256> buffer {};
                    std::transform(trait_name.begin(), trait_name.end(), buffer.begin(), [](auto c) {
                        return std::tolower(c); 
                    });

                    for (const auto& arg : args) {
                        if (arg.rt == rref_type::none)
                            continue;
                        const auto arg_name = data.identifiers.at(arg.name).c_str();

                        file << "pub fn sys_" << buffer.data() << "_" << name << "_new_" << arg_name << "(&self, ";
                        file << arg_name << ": " << arg.type << ") -> RRef<" << arg.type << "> {\n";
                        file << "\tlet proxy = PROXY.r#try().expect(\"Proxy interface is not initialized.\");\n";
                        file << "\tproxy." << buffer.data() << "_" << name << "_new_" << arg_name << "(" << arg_name << ");\n";
                        file << "}\n\n";

                        file << "pub fn sys_" << buffer.data() << "_" << name << "_drop_" << arg_name << "(&self, ";
                        file << arg_name << ": RRef<" << arg.type << ">) {\n";
                        file << "\tlet proxy = PROXY.r#try().expect(\"Proxy interface is not initialized.\");\n";
                        file << "\tproxy." << buffer.data() << "_" << name << "_drop_" << arg_name << "(" << arg_name << ");\n";
                        file << "}\n\n";
                    }

                    file << "pub fn sys_" << name << "(";
                    generate_sys_call_args(data, args, file);
                    file << ") ";

                    if (ret_val.type != "void")
                        file << "-> " << ret_val.type << " ";
                    file << "{\n";
                    
                    file << "\tlet proxy = PROXY.r#try().expect(\"Proxy interface is not initialized.\");\n";
                    file << "\tproxy." << buffer.data() << "_" << name << "(";
                    bool past_first_arg {false};
                    for (const auto& arg : args) {
                        if (past_first_arg)
                            file << ", ";
                        file << data.identifiers.at(arg.name);
                        past_first_arg = true;
                    }
                    file << ");\n";

                    file << "}\n\n";
                }
            }
        }

        void generate_proxy_calls(const data& data, std::ofstream& file)
        {
            file << "impl usr::proxy::Proxy for Proxy {\n";
            file << "\tfn proxy_clone(&self) -> Box<dyn usr::proxy::Proxy> {\n";
            file << "\t\tBox::new((*self).clone());\n";
            file << "\t}\n\n";
            for (const auto& trait : data.traits) {
                const auto trait_name = data.identifiers.at(trait.name);
                auto fn_i = std::next(data.member_functions.begin(), trait.fn_list);
                const auto fns_end = std::next(fn_i, trait.num_fns);

                for (; fn_i < fns_end; ++fn_i) {
                    const auto& func = *fn_i;
                    const auto name = data.identifiers.at(func.name).c_str();
                    const auto arg_list = get_func_args(data, func);
                    const auto ret_val = *arg_list.end;
                    std::vector<argument> args(func.num_args - 1);
                    std::copy(arg_list.begin, arg_list.end, args.begin());

                    std::array<char, 256> buffer {};
                    std::transform(trait_name.begin(), trait_name.end(), buffer.begin(), [](auto c) {
                        return std::tolower(c); 
                    });

                    for (const auto& arg : args) {
                        if (arg.rt == rref_type::none)
                            continue;
                        const auto arg_name = data.identifiers.at(arg.name).c_str();

                        file << "\tfn " << buffer.data() << "_" << name << "_new_" << arg_name << "(&self, ";
                        file << arg_name << ": " << arg.type << ") -> RRef<" << arg.type << "> {\n";
                        file << "\t\tlet rref = RRef::new(0, " << arg_name << ");\n";
                        file << "\t\trref;\n";
                        file << "\t}\n\n";

                        file << "\tfn " << buffer.data() << "_" << name << "_drop_" << arg_name << "(&self, ";
                        file << arg_name << ": RRef<" << arg.type << ">) {\n";
                        file << "\t\tRRef::drop(" << arg_name << ");\n";
                        file << "\t}\n\n";
                    }

                    file << "\tfn " << buffer.data() << "_" << name << "(&self, ";
                    generate_sys_call_args(data, args, file);
                    file << ") ";

                    if (ret_val.type != "void")
                        file << "-> " << ret_val.type << " ";
                    file << "{\n";
                    
                    file << "\t\tlet " << buffer.data() << " = self." << buffer.data() << ".as_deref().expect(\"";
                    file << trait_name << " interface not initialized.\");\n";
                    file << "\t\t" << buffer.data() << "." << name << "(";
                    bool past_first_arg {false};
                    for (const auto& arg : args) {
                        if (past_first_arg)
                            file << ", ";
                        file << data.identifiers.at(arg.name);
                        past_first_arg = true;
                    }
                    file << ");\n";

                    file << "\t}\n\n";
                }
            }
            file << "}\n";
        }

        void generate_proxy(const data& data, std::ofstream& file)
        {
            file << "pub trait Proxy {\n";
            file << "fn proxy_clone(&self) -> Box<dyn Proxy>;\n";
            for (const auto& trait : data.traits) {
                const auto trait_name = data.identifiers.at(trait.name);
                auto fn_i = std::next(data.member_functions.begin(), trait.fn_list);
                const auto fns_end = std::next(fn_i, trait.num_fns);

                for (; fn_i < fns_end; ++fn_i) {
                    const auto& func = *fn_i;
                    const auto name = data.identifiers.at(func.name).c_str();
                    auto arg_i = std::next(data.arguments.begin(), func.arg_list);
                    const auto args_end = std::next(arg_i, func.num_args - 1);
                    const auto ret_val = *args_end;
                    std::vector<argument> args(func.num_args - 1);
                    std::copy(arg_i, args_end, args.begin());

                    std::array<char, 256> buffer {};
                    std::transform(trait_name.begin(), trait_name.end(), buffer.begin(), [](auto c) {
                        return std::tolower(c); 
                    });

                    for (const auto& arg : args) {
                        if (arg.rt == rref_type::none)
                            continue;
                        const auto arg_name = data.identifiers.at(arg.name).c_str();
                        file << "\tfn " << buffer.data() << "_" << name << "_new_" << arg_name << "(&self, ";
                        file << arg_name << ": " << arg.type << ") -> RRef<" << arg.type << ">;\n";
                        file << "\tfn " << buffer.data() << "_" << name << "_drop_" << arg_name << "(&self, ";
                        file << arg_name << ": RRef<" << arg.type << ">);\n";
                    }

                    file << "\tfn " << buffer.data() << "_" << name << "(&self, ";

                    generate_sys_call_args(data, args, file);

                    file << ")";
                    if (ret_val.type != "void")
                        file << " -> " << ret_val.type;
                    file << ";\n";
                }
            }
            file << "}\n\n";
        }

        void generate_proxies(const data& data, const char* path)
        {
            std::ofstream file {path};
            generate_sys_calls(data, file);
            generate_proxy(data, file);
            generate_proxy_calls(data, file);
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