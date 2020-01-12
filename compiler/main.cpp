#include <algorithm>

#include "parse_data.h"
#include "parser.h"

int main(int argc, char** argv)
{
    bool is_good {true};
    const compiler::data* data;
    try {
        data =static_cast<const compiler::data*>(Parser::parse(std::string {argv[1]}));
    } catch (const Parser::ParseException& e) {
        std::printf("%s", e.getReason().c_str());
        return -1;
    }
    for (const std::size_t id : data->type_refs) {
        const auto type_i = std::find(data->allowed_types.begin(), data->allowed_types.end(), id);
        if (type_i == data->allowed_types.end()) {
            std::printf("Could not resolve referenced type \"%s\"\n", data->identifiers.at(id).c_str());
            is_good = false;
        }
    }
    if (is_good)
        std::printf("Everything looks good!\n");

    for (const auto arg : data->arguments) {
        std::printf("Argument \"%s\" of type %s\n", data->identifiers.at(arg.name).c_str(), arg.type.c_str());
    }

    std::printf("init() used these traits (return type last):\n");
    for (const std::size_t id : data->init_signature) {
        std::printf("%s\n", data->identifiers.at(id).c_str());
    }

    return 0;
}