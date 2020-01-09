#include <algorithm>

#include "parse_data.h"
#include "parser.h"

int main(int argc, char** argv)
{
    try {
        bool is_good {true};
        auto data = static_cast<const compiler::data*>(Parser::parse(std::string {argv[1]}));
        for (const std::size_t id : data->type_refs) {
            const auto type_i = std::find(data->allowed_types.begin(), data->allowed_types.end(), id);
            if (type_i == data->allowed_types.end()) {
                std::printf("Could not resolve referenced type \"%s\"\n", data->identifiers.at(id).c_str());
                is_good = false;
            }
        }
        if (is_good)
            std::printf("Everything looks good!\n");

        std::printf("init() used these traits (return type last):\n");
        for (const std::size_t id : data->init_signature) {
            std::printf("%s\n", data->identifiers.at(id).c_str());
        }
    }
    catch (const Parser::ParseException& e) {
        std::printf("%s", e.getReason().c_str());
    }

    return 0;
}