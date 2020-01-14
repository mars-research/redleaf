#!/usr/bin/env python

class TestException(Exception):
    def __init__(self, message):
        Exception.__init__(self, message)

def erase(file):
    import os
    try:
        os.remove(file)
    except OSError:
        pass

def write(data, file):
    f = open(file, 'w')
    f.write(data)
    f.close()

def rootPath():
    return ".test"

file_count = 0
def newFile(suffix = ""):
    import os
    global file_count
    file_count += 1
    return "file%d%s" % (file_count, suffix)
    # return os.path.join(rootPath(), "file%d%s" % (file_count, suffix))

def get_peg_output(option, grammar):
    import subprocess
    peg_out = subprocess.Popen(['./peg.py', option, grammar], stdout = subprocess.PIPE)
    code = peg_out.wait()
    out, err = peg_out.communicate()
    if code != 0:
        raise TestException(out)
    return out

def do_bnf(name, grammar):
    print "[%s] Test bnf.." % name
    # peg_out = subprocess.Popen(['./peg.py', '--bnf', grammar], stdout = subprocess.PIPE)
    # out, err = peg_out.communicate()
    out = get_peg_output('--bnf', grammar)
    g2 = ".bnf2"
    write(out, g2)
    out2 = get_peg_output('--bnf', g2)
    # peg_out2 = subprocess.Popen(['./peg.py', '--bnf', g2], stdout = subprocess.PIPE)
    # out2, err2 = peg_out2.communicate()
    erase(g2)
    if out != out2:
        print "error with bnf generation!!"
        print out
        print "vs"
        print out2
        return False
    return True

def do_ruby(name, grammar, input):
    print "[%s] Test ruby.." % name
    out = get_peg_output('--ruby', grammar)
    file = newFile('.rb')
    write(out, file)
    import re
    import subprocess
    module = re.match(r"(\w+)\.rb", file).group(1)
    process = subprocess.Popen(['ruby', '-e', 'require "%s"; puts parse(ARGV[0])' % module, input], stdout = subprocess.PIPE)
    code = process.wait()
    out, err = process.communicate()
    # erase(file)
    if code != 0:
        raise TestException("Ruby failed: %s" % code)
    return out

def do_python(name, grammar, input):
    import re
    # import subprocess
    try:
        print "[%s] Test python.." % name
        out = get_peg_output('--python', grammar)
        # peg_out = subprocess.Popen(['./peg.py', '--python', grammar], stdout = subprocess.PIPE)
        # out, err = peg_out.communicate()
        file = newFile('.py')
        write(out, file)
        name = re.match(r"(\w+)\.py", file).group(1)
        x = __import__(name)
        result = x.parseFile(input)
        erase(file)
        erase(file + 'c')
        if result == None:
            raise TestException("Error with python parser")
        return result
    except Exception as e:
        import traceback
        traceback.print_exc()
        raise TestException(str(e))

def do_cpp(name, grammar, input):
    import subprocess
    print "[%s] Test c++.." % name
    out = get_peg_output("--cpp", grammar)
    cpp = '.test_cpp.cpp'
    write(out, cpp)
    driver = '.driver.cpp'
    driver_code = """
#include <string>
#include <vector>
#include <iostream>

using namespace std;

namespace Parser{
    struct Value;
    const void * parse(const std::string & filename, bool stats = false);
}

int main(int argc, char ** argv){
    if (argc >= 2){
        const void * result = Parser::parse(argv[1]);
        cout << (int) result << endl << endl;
        return 0;
    } else {
        cout << "Give an argument" << endl;
    }
    return 1;
}
"""
    write(driver_code, driver)

    exe = './.cpp-test'
    subprocess.call(["g++", "-g3", cpp, driver, "-o", exe])
    # out = subprocess.call([exe, input])
    cpp_out = subprocess.Popen([exe, input], stdout = subprocess.PIPE)
    code = cpp_out.wait()
    out, err = cpp_out.communicate()

    # erase(driver)
    # erase(cpp)
    return out

def test_all(name, grammar, input):
    grammar_file = newFile()
    input_file = newFile()
    
    write(grammar, grammar_file)
    write(input, input_file)

    do_bnf(name, grammar_file)
    do_python(name, grammar_file, input_file)
    do_cpp(name, grammar_file, input_file)
    do_ruby(name, grammar_file, input_file)

    erase(grammar_file)
    erase(input_file)

def test_something(name, grammar, input, func):
    grammar_file = newFile()
    input_file = newFile()
    
    write(grammar, grammar_file)
    write(input, input_file)

    out = func(name, grammar_file, input_file)

    erase(grammar_file)
    erase(input_file)
    return out

def test_cpp(name, grammar, input):
    return test_something(name, grammar, input, do_cpp)

def test_python(name, grammar, input):
    return test_something(name, grammar, input, do_python)

def test_ruby(name, grammar, input):
    return test_something(name, grammar, input, do_ruby)

def test1():
    grammar = """
start-symbol: start
rules:
    start = "a"* "b"* "\\n"* <eof>
"""
    input = """aaaaaaabbbbbb"""

    test_all('test1', grammar, input)

def test2():
    grammar = """
start-symbol: start
rules:
    start = "a"* &"b" "b"+ "\\n"* <eof>
"""
    input = """aaaaaaabbbbbb"""

    test_all('test2', grammar, input)

def test3():
    grammar = """
start-symbol: start
include: {{
#include <iostream>
static void got_a(){
    std::cout << "Got an 'aa'!" << std::endl;
}
}}
rules:
    start = a* b "\\n"* <eof>
    a = "aa" {{
        got_a();
    }}
    b = "b"
"""
    input = """aaaab"""

    test_cpp('test3', grammar, input)
    
import sys
# add rootPath to sys path

def test4():
    def cpp():
        grammar = """
start-symbol: start
code: {{
static Value add(const Value & a, const Value & b){
    return Value((void*)((int) a.getValue() + (int) b.getValue()));
}

static Value sub(const Value & a, const Value & b){
    return Value((void*)((int) a.getValue() - (int) b.getValue()));
}

static Value multiply(const Value & a, const Value & b){
    return Value((void*)((int) a.getValue() * (int) b.getValue()));
}

static Value divide(const Value & a, const Value & b){
    return Value((void*)((int) a.getValue() / (int) b.getValue()));
}

}}

rules:
        start = expression sw <eof> {{ value = $1; }}
        expression = expression2 expression1_rest($1)
        expression1_rest(a) = "+" expression2 e:{{value = add(a,$2);}} expression1_rest(e)
                            | "-" expression2 e:{{value = sub(a,$2);}} expression1_rest(e)
                            | <void> {{ value = a; }}

        expression2 = expression3 expression2_rest($1)
        expression2_rest(a) = "*" expression3 e:{{value = multiply(a,$2);}} expression2_rest(e)
                            | "/" expression3 e:{{value = divide(a,$2);}} expression2_rest(e)
                            | <void> {{ value = a; }}

        expression3 = number
                    | "(" expression ")" {{ value = $2; }}

        inline number = digit+ {{
            int total = 0;
            for (Value::iterator it = $1.getValues().begin(); it != $1.getValues().end(); it++){
                const Value & v = *it;
                char letter = (char) (int) v.getValue();
                total = (total * 10) + letter - '0';
            }
            value = (void*) total;
        }}
        inline sw = "\\n"*
        inline digit = [0123456789]
"""

        input = """1+(3-2)*9/(2+2*32)-3232342+91"""
        out = test_cpp('test4', grammar, input).strip()
        expected = "-3232250"
        if out != expected:
            raise TestException("Expected %s but got %s" % (expected, out))

    def python():
        grammar = """
start-symbol: start
options: debug9
rules:
        start = expression sw <eof> {{ value = $1; }}
        expression = expression2 expression1_rest($1)
        expression1_rest(a) = "+" expression2 e:{{value = a + $2;}} expression1_rest(e)
                            | "-" expression2 e:{{value = a - $2;}} expression1_rest(e)
                            | <void> {{ value = a; }}

        expression2 = expression3 expression2_rest($1)
        expression2_rest(a) = "*" expression3 e:{{value = a * $2;}} expression2_rest(e)
                            | "/" expression3 e:{{value = a / $2;}} expression2_rest(e)
                            | <void> {{ value = a; }}

        expression3 = number
                    | "(" expression ")" {{ value = $2; }}

        inline number = digit+ {{
            value = int(''.join($1))
        }}
        inline sw = "\\n"*
        inline digit = [0123456789]
"""


        expected = "-3232250"
        input = """1+(3-2)*9/(2+2*32)-3232342+91"""
        out = test_python('test4', grammar, input)
        if str(out) != str(expected):
            raise TestException("Expected %s but got %s" % (expected, out))

    def ruby():
        grammar = """
start-symbol: start
rules:
        start = expression sw <eof> {{ value = $1; }}
        expression = expression2 expression1_rest($1)
        expression1_rest(a) = "+" expression2 e:{{value = a + $2;}} expression1_rest(e)
                            | "-" expression2 e:{{value = a - $2;}} expression1_rest(e)
                            | <void> {{ value = a; }}

        expression2 = expression3 expression2_rest($1)
        expression2_rest(a) = "*" expression3 e:{{value = a * $2;}} expression2_rest(e)
                            | "/" expression3 e:{{value = a / $2;}} expression2_rest(e)
                            | <void> {{ value = a; }}

        expression3 = number
                    | "(" expression ")" {{ value = $2; }}

        inline number = digit+ {{
            value = $1.join('').to_i
        }}
        inline sw = "\\n"*
        inline digit = [0123456789]
"""

        expected = "-3232250"
        input = """1+(3-2)*9/(2+2*32)-3232342+91"""
        out = test_ruby('test4', grammar, input).strip()
        if str(out) != str(expected):
            raise TestException("Expected '%s' but got '%s'" % (expected, out))

    cpp()
    python()
    ruby()

def test5():
    grammar = """
start-symbol: start
rules:
    start = a:a b:b {{value = a;}}
    a = "a"
    b = "b"
"""

    input = "ab"

    test_all('test5', grammar, input)

def test6():
    grammar = """
start-symbol: start
rules:
    start = x:x sum[a](x) sum[b](x) sum[c](x) sum2[d,e](x)
    sum[what](arg) = @what(arg)
    sum2[what,who](arg) = @what[@who](arg)
    a(x) = "a"
    b(x) = "b"
    c(arg) = "c"
    d[fuz](more) = @fuz(more)
    e(more) = "e"
    x = "x"
"""

    input = "xabce"
    test_all('test6', grammar, input)

def test7():
    grammar = """
start-symbol: start
rules:
    start = x
    x = "a" <predicate b>{{ b = false; }} {{ value = (void*) 1; }}
      | "a" <predicate b>{{ b = true; }} {{ value = (void*) 2; }}
"""
    expected = "2"
    input = """a"""
    out = test_cpp('test7', grammar, input).strip()
    if str(out) != str(expected):
        raise TestException("Expected '%s' but got '%s'" % (expected, out))

def run():
    tests = [test1, test2, test3, test4, test5, test6, test7]
    import sys
    failures = 0
    run = 0
    if len(sys.argv) > 1:
        for num in sys.argv[1:]:
            try:
                run += 1
                tests[int(num) - 1]()
            except TestException as t:
                failures += 1
                print t
    else:
        for test in tests:
            try:
                run += 1
                test()
            except TestException as t:
                failures += 1
                print t
    print
    print "Tests run %d. Failures %d" % (run, failures)

run()
