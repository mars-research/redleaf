#!/usr/bin/env python

# Packrat PEG (parsing expression grammar) generator
#   http://pdos.csail.mit.edu/~baford/packrat/
# Optimizations (like chunks) and other inspiration: Rats!
#   http://cs.nyu.edu/rgrimm/xtc/rats.html
# By Jon Rafkind
# License: GPL 2

# Python BNF parser:
# 1. 171397b / 45.216s = 3790.62721160651 b/s
# 2. 171397b / 36.751s = 4663.73704116895 b/s
# 3. 171397b / 8.630s = 19860.6025492468 b/s
# 4. 171397b / 10.539s = 16263.1179428788 b/s

# Todo (finished items at bottom)
# inline rules + semantic actions are broken (in C++ at least)
# add header (.h) generator for C/C++
# add generator for scheme, haskell, java, scala, ocaml, erlang, javascript, php, pascal, perl, C
# fix error message reporting (whats wrong with it?)
# Don't memoize if a rule accepts parameters (ruby, python)
# Robert Grimm suggested "His Elkhound-generated C++ parsers do not free memory nor do they integrate GC. Instead, you just allocate from a dedicated region, copy out the AST after parsing, and kill the entire region in one operation."
#

from core import CodeGenerator, newResult, gensym, resetGensym, indent, special_char, newOut
from cpp_generator import CppGenerator
from python_generator import PythonGenerator
from ruby_generator import RubyGenerator
from cpp_interpreter_generator import CppInterpreterGenerator
import cpp_generator, python_generator, ruby_generator, cpp_header_generator

# substitute variables in a string named by $foo
# "$foo + $bar - $foo" with {foo:1, bar:2} => "1 + 2 - 1"
# this is orders of magnitude slower than normal python string
# interpolation like "%s %s %s" % (str1, str2, str3)
# so only use it if it makes life much easier (and speed isn't an issue)
def template(now, dict):
    import re
    for key in dict:
        now = re.sub(r'\$%s' % key, str(dict[key]), now)
    return now

def flatten(lst):
    if isinstance(lst, list):
        out = []
        for x in lst:
            out.extend(flatten(x))
        return out
    return [lst]

def reverse(lst):
    # reversed only seems to be in python 2.4+, but I need to support 2.3
    # list(reversed(lst))
    return lst[::-1]

def special_escape(s):
    return s.replace("\\n", "\\\\n").replace("\\t", "\\\\t").replace("\"", "\\\"").replace("\\r", "\\\\r")

# unique elements of a list
def unique(lst):
    x = []
    for item in lst:
        if not item in x:
            x.append(item)
    return x

# Thrown when an eof rule is encountered
class DoneGenerating(Exception):
    pass

# Generates random input based on the Peg
class TestGenerator(CodeGenerator):
    def generate_sequence(self, pattern, peg):
        def make(p):
            def doit(work):
                return p.generate_v2(self, peg)
            return doit
        return [make(p) for p in pattern.patterns]
        # return "".join([p.generate_v2(self, peg) for p in pattern.patterns])

    def generate_verbatim(self, pattern, peg):
        def make2(work):
            if pattern.letters in ["\\r"]:
                return "\n"
            if type(pattern.letters) == type(0):
                return chr(pattern.letters)
            else:
                return pattern.letters
        return [make2]

    def generate_eof(self, pattern, peg):
        def blah(work):
            raise DoneGenerating()
        return [blah]
        # return ""

    def generate_any(self, pattern, peg):
        return [lambda work: 'x']
        # return 'x'

    def generate_ensure(self, pattern, peg):
        def blah(work):
            # Throw the result out, but generate it for eof
            pattern.generate_v2(self, peg)
            return ""
        return [blah]
        #return ""

    def generate_rule(self, pattern, peg):
        # print "Generating rule %s" % pattern.rule
        def make3(work):
            rule = peg.getRule(pattern.rule)
            if work > 10 and rule.hasEmptyRule(peg):
                # print "Skipping rule %s" % rule.name
                return ""
            else:
                return rule.generate_test(self, peg)
        return [make3]
        # return peg.getRule(pattern.rule).generate_test(self, peg)

    def generate_bind(self, pattern, peg):
        def make4(work):
            return pattern.pattern.generate_v2(self, peg)
        return [make4]
        # return pattern.pattern.generate_v2(self, peg)

    def generate_range(self, pattern, peg):
        def make5(work):
            import random
            return random.choice(pattern.range)
        return [make5]

    def generate_line(self, pattern, peg):
        return [lambda work: ""]

    def generate_void(self, pattern, peg):
        return [lambda work: ""]

    def generate_maybe(self, pattern, peg):
        def make(work):
            import random
            if random.randint(0, 1) == 0:
                return pattern.pattern.generate_v2(self, peg)
            else:
                return ""
        return [make]

    def generate_repeat_many(self, pattern, peg):
        import random
        def make(p):
            def doit(work):
                return p.generate_v2(self, peg)
            return doit
        return [make(pattern.next) for x in [1] * random.randint(0, 4)]

        # return "".join([pattern.next.generate_v2(self, peg) for x in [1] * random.randint(0, 4)])

    def generate_repeat_once(self, pattern, peg):
        import random
        def make(p):
            def doit(work):
                return p.generate_v2(self, peg)
            return doit
        return [make(pattern.next) for x in [1] * random.randint(1, 4)]
        # return "".join([pattern.next.generate_v2(self, peg) for x in [1] * random.randint(1, 4)])

    def generate_code(self, pattern, peg):
        return [lambda work: ""]

    def generate_not(self, pattern, peg):
        return [lambda work: ""]

class Pattern:
    def __init__(self):
        pass

    # only true for the failure class
    def isFail(self):
        return False
    
    # only true for PatternLine
    def isLineInfo(self):
        return False
    
    # true if this pattern is at the end of a sequence and calls a rule
    def tailRecursive(self, rule):
        return False

    def generate_v1(self, generator, result, previous_result, stream, failure):
        raise Exception("%s must override the `generate_v1' method to generate code" % (self.__class__))

    # generic code generation method. visitor should be a subclass of
    # CodeGenerator
    def generate(self, visitor, peg, result, stream, failure, tail, peg_args):
        raise Exception("Sub-classes must override the `generate' method to generate code")
    # utility method, probably move it elsewhere
    def parens(self, pattern, str):
        if pattern.contains() > 1:
            return "(%s)" % str
        else:
            return str

# Continues to parse only if the sub-pattern can be parsed, but no input is consumed
class PatternEnsure(Pattern):
    def __init__(self, next):
        Pattern.__init__(self)
        self.next = next

    def ensureRules(self, find):
        self.next.ensureRules(find)

    def find(self, proc):
        def me():
            if proc(self):
                return [self]
            return []
        return me() + self.next.find(proc)

    def isFixed(self):
        return self.next.isFixed()

    def generate_bnf(self):
        return "&" + self.next.generate_bnf()

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_ensure(self, result, previous_result, stream, failure)

    # Takes some arguments
    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_ensure(self, result, previous_result, stream, failure)

    # Takes no arguments other than the generator
    def generate_v2(self, generator, peg):
        return generator.generate_ensure(self, peg)

    def generate_v3(self, generator, rule, peg):
        return generator.generate_ensure(self, rule, peg)

    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_ensure(self, peg, result, stream, failure, tail, peg_args)
        
class PatternNot(Pattern):
    def __init__(self, next):
        Pattern.__init__(self)
        self.next = next

    def ensureRules(self, find):
        self.next.ensureRules(find)

    def find(self, proc):
        def me():
            if proc(self):
                return [self]
            return []
        return me() + self.next.find(proc)

    def isFixed(self):
        return self.next.isFixed()

    def canBeEmpty(self, peg):
        return True

    def generate_bnf(self):
        return "!" + self.next.generate_bnf()

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_not(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_not(self, peg, result, stream, failure, tail, peg_args)

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_not(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_not(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_not(self, rule, peg)
        
class PatternRule(Pattern):
    def __init__(self, rule, rules = None, parameters = None):
        Pattern.__init__(self)
        self.rule = rule
        self.rules = rules
        self.parameters = parameters

    def contains(self):
        return 1

    def tailRecursive(self, rule):
        return self.rule == rule.name

    def canBeEmpty(self, peg):
        return peg.getRule(self.rule).hasEmptyRule(peg)

    def find(self, proc):
        if proc(self):
            return [self]
        return []

    def isFixed(self):
        return False

    def generate_v2(self, generator, peg):
        return generator.generate_rule(self, peg)
        # return peg.getRule(self.rule).generate_test(generator, peg)

    def generate_v3(self, generator, rule, peg):
        return generator.generate_rule(self, rule, peg)

    def ensureRules(self, find):
        if not find(self.rule):
            print "*warning* could not find rule " + self.rule

    def generate_bnf(self):
        rules = ""
        values = ""
        if self.rules != None:
            rules = '[%s]' % ', '.join(self.rules)
        if self.parameters != None:
            values = '(%s)' % ', '.join(self.parameters)
        return '%s%s%s' % (self.rule, rules, values)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_rule(self, result, previous_result, stream, failure)
    
    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_rule(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_rule(self, peg, result, stream, failure, tail, peg_args)
    
class PatternVoid(Pattern):
    def __init__(self):
        Pattern.__init__(self)

    def ensureRules(self, find):
        pass

    def find(self, proc):
        if proc(self):
            return [self]
        return []
    
    def canBeEmpty(self, peg):
        return True

    def isFixed(self):
        return True

    def generate_bnf(self):
        return "<void>"

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_void(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_void(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_void(self, rule, peg)

    def generate_python(self, result, previous_result, stream, failure):
        return ""
    
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_void(self, peg, result, stream, failure, tail, peg_args)

class PatternEof(Pattern):
    def __init__(self):
        Pattern.__init__(self)

    def ensureRules(self, find):
        pass

    def find(self, proc):
        if proc(self):
            return [self]
        return []
    
    def canBeEmpty(self, peg):
        return True

    def generate_bnf(self):
        return "<eof>"

    def isFixed(self):
        return True

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_eof(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_eof(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_eof(self, rule, peg)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_eof(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_eof(self, peg, result, stream, failure, tail, peg_args)
        
class PatternSequence(Pattern):
    def __init__(self, patterns):
        Pattern.__init__(self)
        self.patterns = patterns

    def contains(self):
        return len(self.patterns)

    def tailRecursive(self, rule):
        return self.patterns[-1].tailRecursive(rule)

    def isFixed(self):
        return reduce(lambda ok, pattern: ok and pattern.isFixed(), self.patterns, True)

    def canBeEmpty(self, peg):
        for pattern in self.patterns:
            if not pattern.canBeEmpty(peg):
                return False
        return True

    def find(self, proc):
        def me():
            if proc(self):
                return [self]
            return []
        return flatten([p.find(proc) for p in self.patterns]) + me()

    def ensureRules(self, find):
        for pattern in self.patterns:
            pattern.ensureRules(find)

    def generate_bnf(self):
        return "%s" % " ".join([p.generate_bnf() for p in self.patterns])

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_sequence(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_sequence(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_sequence(self, rule, peg)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_sequence(self, result, previous_result, stream, failure)

    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_sequence(self, peg, result, stream, failure, tail, peg_args)

class PatternCallRule(Pattern):
    def __init__(self, name, rules, values):
        Pattern.__init__(self)
        self.name = name
        self.rules = rules
        self.values = values
    
    def ensureRules(self, find):
        pass

    def find(self, proc):
        if proc(self):
            return [self]
        return []

    def isFixed(self):
        return False
    
    def generate_bnf(self):
        rules = ""
        values = ""
        if self.rules != None:
            rules = '[%s]' % ', '.join(self.rules)
        if self.values != None:
            values = '(%s)' % ', '.join(self.values)
        return '@%s%s%s' % (self.name, rules, values)
    
    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_call_rule(self, result, previous_result, stream, failure)
    
    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_call_rule(self, result, previous_result, stream, failure)
    
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_call_rule(self, peg, result, stream, failure, tail, peg_args)
        
class PatternRepeatOnce(Pattern):
    def __init__(self, next):
        Pattern.__init__(self)
        self.next = next

    def ensureRules(self, find):
        self.next.ensureRules(find)

    def find(self, proc):
        def me():
            if proc(self):
                return [self]
            return []
        return me() + self.next.find(proc)

    def isFixed(self):
        return self.next.isFixed()

    def canBeEmpty(self, peg):
        return False

    def generate_bnf(self):
        return self.parens(self.next, self.next.generate_bnf()) + "+"

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_repeat_once(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_repeat_once(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_repeat_once(self, rule, peg)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_repeat_once(self, result, previous_result, stream, failure)

    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_repeat_once(self, peg, result, stream, failure, tail, peg_args)
        
class PatternCode(Pattern):
    def __init__(self, code):
        Pattern.__init__(self)
        self.code = code

    def contains(self):
        return 1

    def find(self, proc):
        if proc(self):
            return [self]
        return []

    def isFixed(self):
        return True

    def canBeEmpty(self, peg):
        return True

    def ensureRules(self, find):
        pass

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_code(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_code(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_code(self, rule, peg)

    def generate_bnf(self):
        return """{{%s}}""" % (self.code)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_code(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_code(self, peg, result, stream, failure, tail, peg_args)

class PatternRepeatMany(Pattern):
    def __init__(self, next):
        Pattern.__init__(self)
        self.next = next

    def ensureRules(self, find):
        self.next.ensureRules(find)

    def find(self, proc):
        def me():
            if proc(self):
                return [self]
            return []
        return me() + self.next.find(proc)

    def isFixed(self):
        return self.next.isFixed()

    def canBeEmpty(self, peg):
        return True
    
    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_repeat_many(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_repeat_many(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_repeat_many(self, rule, peg)

    def generate_bnf(self):
        return self.parens(self.next, self.next.generate_bnf()) + "*"

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_repeat_many(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_repeat_many(self, peg, result, stream, failure, tail, peg_args)
        
class PatternAny(Pattern):
    def __init__(self):
        Pattern.__init__(self)

    def find(self, proc):
        if proc(self):
            return [self]
        return []

    def generate_bnf(self):
        return "."

    def ensureRules(self, find):
        pass

    def isFixed(self):
        return True

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_any(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_any(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_any(self, rule, peg)

    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_any(self, peg, result, stream, failure, tail, peg_args)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_any(self, result, previous_result, stream, failure)
        
class PatternMaybe(Pattern):
    def __init__(self, pattern):
        Pattern.__init__(self)
        self.pattern = pattern

    def ensureRules(self, find):
        self.pattern.ensureRules(find)

    def isFixed(self):
        return self.pattern.isFixed()

    def find(self, proc):
        def me():
            if proc(self):
                return [self]
            return []
        return me() + self.pattern.find(proc)

    def canBeEmpty(self, peg):
        return True
    
    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_maybe(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_maybe(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_maybe(self, rule, peg)

    def generate_bnf(self):
        return self.parens(self.pattern, self.pattern.generate_bnf()) + "?"

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_maybe(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_maybe(self, peg, result, stream, failure, tail, peg_args)
        
class PatternOr(Pattern):
    def __init__(self, patterns):
        Pattern.__init__(self)
        self.patterns = patterns

    def contains(self):
        return 1

    def ensureRules(self, find):
        for pattern in self.patterns:
            pattern.ensureRules(find)

    def isFixed(self):
        return reduce(lambda ok, pattern: ok and pattern.isFixed(), self.patterns, True)

    def generate_bnf(self):
        return "or"

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_or(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_or(self, peg, result, stream, failure, tail, peg_args)

    def generate_v3(self, generator, rule, peg):
        return generator.generate_or(self, rule, peg)
        
class PatternBind(Pattern):
    def __init__(self, variable, pattern):
        Pattern.__init__(self)
        self.variable = variable
        self.pattern = pattern
        if self.variable == 'value':
            raise Exception("Cannot bind a pattern with the name 'value' because it is a reserved variable name used in the implementation of the peg")

    def ensureRules(self, find):
        self.pattern.ensureRules(find)

    def isFixed(self):
        return self.pattern.isFixed()

    def find(self, proc):
        def me():
            if proc(self):
                return [self]
            return []
        return me() + self.pattern.find(proc)

    def canBeEmpty(self, peg):
        return self.pattern.canBeEmpty(peg)

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_bind(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_bind(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_bind(self, rule, peg)

    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_bind(self, peg, result, stream, failure, tail, peg_args)
        
    def generate_bnf(self):
        return "%s:%s" % (self.variable, self.pattern.generate_bnf())

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_bind(self, result, previous_result, stream, failure)
        
def PatternUntil(pattern):
    return PatternRepeatMany(PatternSequence([
        PatternNot(pattern),
        PatternAny()
        ]))

class PatternRange(Pattern):
    def __init__(self, range):
        Pattern.__init__(self)
        self.range = range

    def find(self, proc):
        if proc(self):
            return [self]
        return []

    def ensureRules(self, find):
        pass

    def isFixed(self):
        return True

    def canBeEmpty(self, peg):
        return False

    def generate_bnf(self):
        return "[%s]" % self.range

    def generate_v2(self, generator, peg):
        return generator.generate_range(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_range(self, rule, peg)

    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_range(self, peg, result, stream, failure, tail, peg_args)
        
    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_range(self, result, previous_result, stream, failure)

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_range(self, result, previous_result, stream, failure)

class PatternLine(Pattern):
    def __init__(self):
        Pattern.__init__(self)

    def ensureRules(self, find):
        pass

    def find(self, proc):
        if proc(self):
            return [self]
        return []
    
    def contains(self):
        return 1

    def isFixed(self):
        return True

    def isLineInfo(self):
        return True

    def canBeEmpty(self, peg):
        return True

    def generate_bnf(self):
        return '<item>'

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_line(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_line(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_line(self, rule, peg)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_line(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_line(self, peg, result, stream, failure, tail, peg_args)

class PatternPredicate(Pattern):
    def __init__(self, variable, code):
        Pattern.__init__(self)
        self.variable = variable
        self.code = code
    
    def ensureRules(self, find):
        pass

    def find(self, proc):
        if proc(self):
            return [self]
        return []

    def isFixed(self):
        return False

    def canBeEmpty(self, peg):
        return True

    def contains(self):
        return 1

    def generate_bnf(self):
        return '<predicate %s> {{%s}}"' % (self.variable, self.code)

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_predicate(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_predicate(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_predicate(self, rule, peg)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_predicate(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_predicate(self, peg, result, stream, failure, tail, peg_args)


class PatternVerbatim(Pattern):
    def __init__(self, letters, options = None):
        Pattern.__init__(self)
        self.letters = letters
        self.options = options

    def ensureRules(self, find):
        pass

    def isFixed(self):
        return True

    def find(self, proc):
        if proc(self):
            return [self]
        return []

    def canBeEmpty(self, peg):
        return False

    def contains(self):
        return 1

    def generate_bnf(self):
        if type(self.letters) == type('x'):
            return '"%s"' % self.letters
        elif type(self.letters) == type(0):
            return '<ascii %d>' % self.letters

    def generate_v1(self, generator, result, previous_result, stream, failure):
        return generator.generate_verbatim(self, result, previous_result, stream, failure)

    def generate_v2(self, generator, peg):
        return generator.generate_verbatim(self, peg)
    
    def generate_v3(self, generator, rule, peg):
        return generator.generate_verbatim(self, rule, peg)

    def generate_python(self, result, previous_result, stream, failure):
        return PythonGenerator().generate_verbatim(self, result, previous_result, stream, failure)
        
    def generate_cpp(self, peg, result, stream, failure, tail, peg_args):
        return CppGenerator().generate_verbatim(self, peg, result, stream, failure, tail, peg_args)
        
class Rule:
    def __init__(self, name, patterns, rules = None, inline = False, parameters = None, fail = None):
        self.name = name
        self.patterns = patterns
        self.inline = inline
        self.fail = fail
        self.rules = rules
        self.parameters = parameters

    def isInline(self):
        return self.inline

    def doInline(self):
        if not self.inline and self.parameters == None:
            ok = True
            for pattern in self.patterns:
                ok = ok and pattern.isFixed()
            if ok:
                # print "%s is now inlined" % self.name
                self.inline = True

    def generate_bnf(self):
        total_length = len(self.name)
        if self.inline:
            total_length += len('inline ')
        data = """
%s = %s
""" % (self.name, (('\n%s | ') % (' ' * total_length)).join([p.generate_bnf() for p in self.patterns]))
        if self.inline:
            return "inline " + data.strip() + "\n"
        return data.strip() + "\n"

    def ensureRules(self, find):
        for pattern in self.patterns:
            pattern.ensureRules(find)

    def generate_ruby(self):
        def newPattern(pattern, stream, position):
            result = newResult()
            rule_id = "RULE_%s" % self.name

            def fail():
                return "raise PegError"
            data = """
begin
    %s = Result.new(%s)
    %s
    %s.update(%s, %s, %s)
    return %s
rescue PegError
end
            """ % (result, position, indent(pattern.generate_v1(RubyGenerator(), result, None, stream, fail).strip()), stream, rule_id, position, result, result)
            return data


        stream = "stream"
        position = "position"
        rule_id = "RULE_%s" % self.name
        rule_parameters = ""
        if self.rules != None:
            rule_parameters = ", " + ", ".join(["%s" % p for p in self.rules])
        parameters = ""
        if self.parameters != None:
            parameters = ", " + ", ".join(["%s" % p for p in self.parameters])

        data = """
def rule_%s(%s, %s%s%s)
    if %s.hasResult(%s, %s)
        return %s.result(%s, %s)
    end
    %s
    %s.update(%s, %s, nil)
    return nil
end
""" % (self.name, stream, position, rule_parameters, parameters, stream, rule_id, position, stream, rule_id, position, indent('\n'.join([newPattern(pattern, stream, position).strip() for pattern in self.patterns])), stream, rule_id, position)
        return data

    def choosePattern(self):
        import random
        return random.choice(self.patterns)

    def hasEmptyRule(self, peg):
        for pattern in self.patterns:
            if pattern.canBeEmpty(peg):
                return True
        return False

    def generate_test(self, generator, peg):
        return self.choosePattern().generate_v2(generator, peg)

    def generate_python(self):
        def newPattern(pattern, stream, position):
            result = newResult()

            def fail():
                return "raise PegError"
            data = """
try:
    %s = Result(%s)
    %s
    %s.update(%s, %s, %s)
    return %s
except PegError:
    pass
            """ % (result, position, indent(pattern.generate_python(result, None, stream, fail).strip()), stream, "RULE_%s" % self.name, position, result, result)
            return data

        stream = "stream"
        position = "position"
        rule_parameters = ""
        if self.rules != None:
            rule_parameters = ", " + ", ".join(["%s" % p for p in self.rules])
        parameters = ""
        if self.parameters != None:
            parameters = ", " + ", ".join(["%s" % p for p in self.parameters])
        data = """
def rule_%s(%s, %s%s%s):
    if %s.hasResult(%s, %s):
        return %s.result(%s, %s)
    %s
    %s.update(%s, %s, %s)
    return None
""" % (self.name, stream, position, rule_parameters, parameters, stream, "RULE_%s" % self.name, position, stream, "RULE_%s" % self.name, position, indent('\n'.join([newPattern(pattern, stream, position).strip() for pattern in self.patterns])), stream, "RULE_%s" % self.name, position, "None")

        return data

    def generate_cpp_interpreter(self, peg, chunk_accessor):
        def updateChunk(new, columnVar, memo):
            if not memo:
                return "%s.update(%s.getPosition());" % (stream, new)
            chunk = chunk_accessor.getChunk(columnVar)
            data = """
if (%s == 0){
    %s = new Peg::%s();
}
%s = %s;
stream.update(%s.getPosition());
""" % (chunk, chunk, chunk_accessor.getType(), chunk_accessor.getValue(chunk), new, new)
            return data
            
        columnVar = gensym("column")

        def hasChunk(memo):
            if memo:
                return """
Column & %s = stream.getColumn(position);
if (%s != 0 && %s.calculated()){
    if (%s.error()){
        throw Peg::Failure();
    }
    return %s;
}
""" % (columnVar, chunk_accessor.getChunk(columnVar), chunk_accessor.getValue(chunk_accessor.getChunk(columnVar)), chunk_accessor.getValue(chunk_accessor.getChunk(columnVar)), chunk_accessor.getValue(chunk_accessor.getChunk(columnVar)))
            else:
                return ""

        generator = CppInterpreterGenerator()
        patterns = "\n".join(["expressions->push_back(%s);" % pattern.generate_v3(generator, self, peg) for pattern in self.patterns])
        extra = '\n'.join(generator.extra_codes)
        data = """
%s
std::vector<Peg::Expression*> * create_rule_%s(){
    std::vector<Peg::Expression*> * expressions = new std::vector<Peg::Expression*>();
    %s
    return expressions;
}

Result rule_%s(Stream & stream, int position, Value ** arguments){
    %s
    std::vector<Peg::Expression*> * expressions = stream.getRule(Peg::Rule_%s);
    for (std::vector<Peg::Expression*>::const_iterator it = expressions->begin(); it != expressions->end(); it++){
        try{
            Peg::Expression * expression = *it;
            Result out = expression->parse(stream, position, arguments);
            %s
            return out;
        } catch (const Peg::Failure & failure){
            /* try next rule.. */
        }
    }
    throw Peg::Failure();
    // return Peg::errorResult;
}
""" % (extra, self.name, indent(patterns), self.name, indent(hasChunk(True)), self.name, indent(indent(indent(updateChunk('out', columnVar, True)))))
        return data

    # find all declared variables by a rule including all declared variables
    # declared by inline rules this rule calls
    # FIXME: if their is a duplicate variable name in an inlined rule things
    # might break
    def findVars(self, peg):
        def isBind(pattern):
            return isinstance(pattern, PatternBind)
        def isInlined(pattern):
            ok = isinstance(pattern, PatternRule)
            if ok:
                if peg.getRule(pattern.rule) == None:
                    closest = peg.findClosestRuleName(pattern.rule)
                    raise Exception("No rule found '%s'. Closest match is '%s'" % (pattern.rule, closest))
                return peg.getRule(pattern.rule).isInline()
            else:
                return False
        inlined = [peg.getRule(rule.rule) for rule in flatten([p.find(isInlined) for p in self.patterns])]
        bind_patterns = flatten([p.find(isBind) for p in self.patterns])
        mine = [p.variable for p in bind_patterns]
        others = flatten([rule.findVars(peg) for rule in inlined])
        return unique(mine + others)

    def generate_cpp(self, peg, chunk_accessor):
        resetGensym()
        rule_number = "RULE_%s" % self.name
        stream = "stream"
        position = "position"
        # tail_loop = [gensym("tail")]
        tail_loop = [False]
        debug = "debug1" in peg.options
        
        def updateChunk(new, columnVar, memo):
            if not memo:
                return "%s.update(%s.getPosition());" % (stream, new)
            chunk = chunk_accessor.getChunk(columnVar)
            data = """
if (%s == 0){
    %s = new %s();
}
%s = %s;
%s.update(%s.getPosition());
""" % (chunk, chunk, chunk_accessor.getType(), chunk_accessor.getValue(chunk), new, stream, new)
            return data
            
        columnVar = gensym("column")

        def hasChunk(memo):
            if memo:
                return """
Column & %s = %s.getColumn(%s);
if (%s != 0 && %s.calculated()){
    return %s;
}
""" % (columnVar, stream, position, chunk_accessor.getChunk(columnVar), chunk_accessor.getValue(chunk_accessor.getChunk(columnVar)), chunk_accessor.getValue(chunk_accessor.getChunk(columnVar)))
            else:
                return ""
        
        def newPattern(pattern, stream, position):
            result = newResult()
            out = [False]

            def label(n):
                if n != False:
                    return "%s:" % n
                return ""

            def failure():
                if out[0] == False:
                    out[0] = newOut()
                return "goto %s;" % out[0]
            
            def invalid_arg(d):
                raise Exception("No results available")

            if pattern.tailRecursive(self):
                tail_vars = self.parameters
                if tail_vars == None:
                    tail_vars = []
                if tail_loop[0] == False:
                    tail_loop[0] = gensym("tail")
                data = """
Result %s(%s);
%s
%s = %s.getPosition();
goto %s;
%s
    """ % (result, position, pattern.generate_cpp(peg, result, stream, failure, tail_vars, invalid_arg).strip(), position, result, tail_loop[0], label(out[0]))
            else:
                # non-tail so dont make the tail label
                debugging = ""
                debug_result = ""
                if debug:
                    debugging = """std::cout << "Trying rule %s at " << %s << " '" << %s.get(%s.getPosition()) << "' alternative: %s" << std::endl;""" % (self.name, position, stream, result, special_escape(pattern.generate_bnf()).replace("\n", "\\n"))
                if 'debug2' in peg.options:
                    debug_result = """std::cout << "Succeeded rule %s at position " << %s.getPosition() << " alternative: %s" << std::endl;""" % (self.name, result, special_escape(pattern.generate_bnf()).replace("\n", "\\n"))
                do_memo = peg.memo and self.rules == None and self.parameters == None
                data = """
Result %s(%s);
%s
%s
%s
%s
return %s;
%s
            """ % (result, position, debugging, pattern.generate_cpp(peg, result, stream, failure, None, invalid_arg).strip(), updateChunk(result, columnVar, do_memo), debug_result, result, label(out[0]))

            return data

        rule_parameters = ""
        if self.rules != None:
            # rule_parameters = ", " + ", ".join(["Result (*%s)(Stream &, const int, ...)" % p for p in self.rules])
            rule_parameters = ", " + ", ".join(["void * %s" % p for p in self.rules])

        parameters = ""
        if self.parameters != None:
            parameters = ", " + ", ".join(["Value %s" % p for p in self.parameters])

        def declareVar(var):
            return "Value %s;" % var

        vars = "\n".join([declareVar(v) for v in self.findVars(peg)])
        my_position = "myposition"

        fail_code = ""
        if self.fail != None:
            fail_code = self.fail

        def label(n):
            if n != False:
                return "%s:" % n
            return ""
        
        pattern_results = indent('\n'.join([newPattern(pattern, stream, my_position).strip() for pattern in self.patterns]))

        # Don't memoize if the rule accepts parameters
        do_memo = peg.memo and self.rules == None and self.parameters == None
        body = """
%s
%s
%s
%s
""" % (label(tail_loop[0]), indent(vars), pattern_results, indent(updateChunk("errorResult", columnVar, do_memo)))

        if self.fail != None:
            body = """
try{
    %s
} catch (...){
    %s
    throw;
}
""" % (body, self.fail)

        data = """
Result rule_%s(Stream & %s, const int %s%s%s){
    %s
    RuleTrace %s(%s, "%s");
    int %s = %s;
    %s
    return errorResult;
}
        """ % (self.name, stream, position, rule_parameters, parameters, indent(hasChunk(do_memo)), gensym("trace"), stream, self.name, my_position, position, indent(body))

        return data
    
class Peg:
    def __init__(self, start, include_code, more_code, module, rules, options):
        self.start = start
        self.rules = rules
        self.include_code = include_code
        self.more_code = more_code
        self.module = module
        self.options = options
        # Whether to memoize or not
        self.memo = True
        if options == None:
            self.options = []
        if self.module == None:
            self.module = ['Parser']
        if 'no-memo' in self.options:
            self.memo = False
        # Default error length
        self.error_size = 15
        for option in self.options:
            import re
            length = re.compile('error-length (\d+)')
            match = length.match(option)
            if match:
                self.error_size = int(match.group(1))

        if self.getRule(self.start) == None:
            raise Exception("No start rule with the name '%s'" % self.start)

        for rule in self.rules:
            rule.ensureRules(lambda r: r in [r2.name for r2 in self.rules])

        # convert some rules to inline if they can be
        for rule in self.rules:
            rule.doInline()

        self.getRule(self.start).inline = False

    def getRule(self, name):
        for rule in self.rules:
            if rule.name == name:
                return rule
        return None

    def findClosestRuleName(self, name):
        return 'not done yet'
    
    def generate_test(self):
        # return self.getRule(self.start).generate_test(TestGenerator(), self)
        work = self.getRule(self.start).generate_test(TestGenerator(), self)
        data = ""
        length = 0
        while work:
            head = work.pop()
            length -= 1
            try:
                more = head(length)
            except DoneGenerating:
                more = ""
                data = ""
            # print "More is %s" % more
            if type(more) == type([]):
                work.extend(more)
                length += len(more)
            else:
                data = more + data
        return data

    def generate_bnf(self):
        more = ""
        if self.include_code != None:
            more = """include: {{%s}}""" % self.include_code
        code = ""
        if self.more_code != None:
            code = """code: {{%s}}""" % self.more_code
        data = """
start-symbol: %s
%s
%s
rules:
    %s
""" % (self.start, more, code, indent('\n'.join([rule.generate_bnf() for rule in self.rules]).strip()))
        return data

    def cppNamespaceStart(self):
        start = ""
        for module in reverse(self.module):
            start = """
namespace %s{
%s
""" % (module, indent(start))
        return start

    def cppNamespaceEnd(self):
        end = ""
        for module in reverse(self.module):
            end = """
%s
} /* %s */
""" % (indent(end), module)
        return end

    def list_files(self, name, directory = '.'):
        use_rules = [rule for rule in self.rules if not rule.isInline()]
        out = []
        for rule in use_rules:
            file = '%s/%s-%s.cpp' % (directory, name, rule.name)
            out.append(file)
        return out

    def print_list_files(self, name):
        return '\n'.join(self.list_files(name))
    
def test():
    s_code = """
printf("parsed cheese\\n");
value = (void *) 2;
"""
    rules = [
        Rule("s", [PatternNot(PatternVerbatim("hello")), PatternAction(PatternVerbatim("cheese"), s_code), PatternRepeatOnce(PatternVerbatim("once"))]),
        Rule("blah", [PatternRepeatMany(PatternRule("s"))]),
        Rule("or", [PatternOr([PatternVerbatim("joe"), PatternVerbatim("bob"), PatternVerbatim("sally")])]),
        Rule("all", [PatternSequence([PatternVerbatim("abc"), PatternVerbatim("def"), PatternVerbatim("ghi")])]),
    ]
    peg = Peg("Peg", "s", rules)
    print cpp_generator.generate(peg)

def create_peg(peg):
    # import imp
    # module = imp.new_module(peg.namespace)
    # exec peg.generate_python() in module.__dict__
    # return module.parse

    name = "peg_" + '_'.join(peg.module)
    out = open(name + ".py", 'w')
    out.write(python_generator.generate(peg))
    out.close()
    module = __import__(name, globals(), locals(), ['parse'])
    # print module
    # print dir(module)
    return module.parseFile

def test2():
    start_code_abc = """
std::cout << "Parsed abc!" << std::endl;
"""
    start_code_def = """
std::cout << "Parsed def!" << std::endl;
"""
    rules = [
        Rule("start", [
            PatternAction(PatternSequence([PatternRule("a"),PatternRule("b"), PatternRule("c")]), start_code_abc),
            PatternAction(PatternSequence([PatternRule("d"),PatternRule("e"), PatternRule("f")]), start_code_def),
        ]),
        Rule("a", [PatternVerbatim("a")]),
        Rule("b", [PatternVerbatim("b")]),
        Rule("c", [PatternVerbatim("c")]),

        Rule("d", [PatternVerbatim("d")]),
        Rule("e", [PatternVerbatim("e")]),
        Rule("f", [PatternVerbatim("f")]),
    ]

    peg = Peg("Peg", "start", rules)
    print cpp_generator.generate(peg)

# BNF for parsing BNF description
# This bootstraps the system so we can write normal BNF rules in a file
def peg_bnf(peg_name):
    rules = [
        Rule("start", [
            PatternSequence([
                PatternRule("newlines"),
                PatternRule("whitespace"),
                PatternBind("start_symbol", PatternRule("start_symbol")),
                PatternRule("newlines"),
                PatternRule("whitespace"),
                PatternBind("options", PatternMaybe(PatternRule("options"))),
                PatternRule("newlines"),
                PatternRule("whitespace"),
                PatternBind("module", PatternMaybe(PatternRule("module"))),
                PatternRule("newlines"),
                PatternRule("whitespace"),
                PatternBind('include', PatternMaybe(PatternRule("include"))),
                PatternRule("newlines"),
                PatternRule("whitespace"),
                PatternBind("code", PatternMaybe(PatternRule("more_code"))),
                PatternRule("newlines"),
                PatternRule("whitespace"),
                PatternBind("rules", PatternRule("rules")),
                PatternRule("newlines"),
                PatternRule("whitespace"),
                PatternEof(),

                PatternCode("""value = peg.Peg(start_symbol, include, code, module, rules, options)""")
                ]),
            ]),
        Rule('module', [
            PatternSequence([
                PatternVerbatim("module:"),
                PatternRule("spaces"),
                PatternBind("name", PatternRule("word")),
                PatternBind("rest", 
                    PatternRepeatMany(PatternSequence([
                        PatternVerbatim("."),
                        PatternRule("word"),
                        PatternCode("""value = $2"""),
                        ]))),
                    PatternCode("""value = [name] + rest"""),
                    ]),
            ]),
        Rule("include", [
            PatternSequence([
                PatternVerbatim("include:"),
                PatternRule("spaces"),
                PatternBind("code", PatternRule("code")),
                PatternCode("value = code.code"),
                ]),
            ]),
        Rule("more_code", [
            PatternSequence([
                PatternVerbatim("code:"),
                PatternRule("spaces"),
                PatternBind("code", PatternRule("code")),
                PatternCode("""value = code.code"""),
                ]),
            ]),
        Rule("options", [
                PatternSequence([
                    PatternVerbatim("options:"),
                    PatternRule("spaces"),
                    PatternBind("option1", PatternRule("option")),
                    PatternBind("option_rest", PatternRepeatMany(PatternSequence([
                        PatternRule("spaces"),
                        PatternVerbatim(","),
                        PatternRule("spaces"),
                        PatternRule("option"),
                        ]))),
                    PatternCode("""
value = []
for option in ([option1] + option_rest):
    import re
    debug = re.compile("debug(\d+)")
    out = debug.match(option)
    if out != None:
        num = int(out.group(1))
        for x in xrange(1,num+1):
            value.append('debug%d' % x)
    elif option == 'no-memo':
        value.append(option)
    else:
        value.append(option)
"""),
                    ]),
            ]),
        Rule("option", [
            PatternSequence([
                PatternVerbatim("debug"),
                PatternBind('number', PatternRule("number")),
                PatternCode("""
value = 'debug%s' % number
"""),
                ]),
            PatternVerbatim('no-memo'),
            PatternRule('error_option')
            ]),
        Rule('error_option', [
            PatternSequence([
                PatternVerbatim('error-length'),
                PatternRule('whitespace'),
                PatternBind('number', PatternRule("number")),
                PatternCode("""
value = 'error-length %s' % number
""")
                ]),
            ]),
        Rule("word", [
            PatternSequence([
                PatternRepeatOnce(PatternRule("any_char")),
                PatternCode("""
# print "all start symbol values " + str(values)
# print "values[0] " + str(values[0])
value = ''.join(values[0]).replace('-', '__')
# print "got word " + value
""")
                ]),
            ]),
        Rule("rules", [
            PatternSequence([
                PatternVerbatim("rules:"),
                PatternRule("whitespace"),
                PatternBind("rules", PatternRepeatMany(PatternSequence([
                    PatternRule("rule"),
                    PatternRule("whitespace"),
                    PatternCode("""value = $1"""),
                    ]))),
                PatternCode("""value = rules"""),
                ]),
            ]),
        Rule("rule", [
            PatternSequence([
                PatternRule("spaces"),
                PatternBind("inline", PatternMaybe(PatternVerbatim("inline"))),
                PatternRule("spaces"),
                PatternBind("name", PatternRule("word")),
                PatternBind("rule_parameters", PatternMaybe(PatternRule('rule_parameters'))),
                PatternBind("parameters", PatternMaybe(PatternRule('value_parameters'))),
                PatternRule("spaces"),
                PatternVerbatim("="),
                PatternRule("spaces"),
                PatternBind("pattern1", PatternRule("pattern_line")),
                PatternRule("whitespace"),
                PatternBind("patterns",
                    PatternRepeatMany(PatternSequence([
                        PatternRule("spaces"),
                        PatternVerbatim("|"),
                        PatternRule("spaces"),
                        PatternBind("pattern", PatternRule("pattern_line")),
                        # PatternBind("pattern", PatternRepeatMany(PatternRule("pattern"))),
                        PatternRule("whitespace"),
                        PatternCode("""value = pattern"""),
                        ]),
                        )
                    ),
                PatternBind("fail", PatternMaybe(PatternRule("failure"))),
                PatternCode("""
value = peg.Rule(name, [pattern1] + patterns, inline = (inline != None), rules = rule_parameters, parameters = parameters, fail = fail)"""),
                ]),
            ]),
        Rule("pattern_line",[
            PatternSequence([
                PatternBind("patterns", PatternRepeatMany(PatternRule("pattern"))),
                PatternCode("""
value = peg.PatternSequence(patterns)
#if code != None:
#    value = code(peg.PatternSequence(patterns))
#else:
#    value = peg.PatternAction(peg.PatternSequence(patterns), "value = values;")
"""),
                ]),
            ]),
        Rule("pattern", [
            PatternSequence([
                PatternBind("bind", PatternMaybe(PatternRule("bind"))),
                PatternBind("item", PatternRule("item")),
                PatternRule("spaces"),
                PatternCode("""
# value = peg.PatternRule(values[0])
if bind != None:
    item = bind(item)
value = item
# print "Pattern is " + str(value)
"""),
                ]),
            ]),
        Rule('raw_code', [PatternSequence([
            PatternVerbatim("("),
            PatternRule("spaces"),
            PatternBind("code", PatternRule('code')),
            PatternRule("spaces"),
            PatternVerbatim(")"),
            PatternCode("""value = code.code"""),
            ]),
        ]),
        Rule("code", [
            PatternSequence([
                PatternVerbatim("{{"),
                PatternRepeatOnce(PatternSequence([
                    PatternNot(PatternVerbatim("}}")),
                    PatternAny(),
                    PatternCode("""value = values[1]""")
                    ])),
                PatternVerbatim("}}"),
                PatternCode("""value = peg.PatternCode(''.join(values[1]))"""),
                ]),
            ]),
        Rule("item", [
            PatternSequence([
                PatternBind("ensure", PatternMaybe(PatternVerbatim("&"))),
                PatternBind("pnot", PatternMaybe(PatternVerbatim("!"))),
                PatternBind("pattern",
                    PatternOr([
                        PatternRule("x_word"),
                        PatternRule("any"),
                        # Actions inside an Or don't work
                        # PatternAction(PatternVerbatim("."), """value = peg.PatternAny()"""),
                        PatternRule("eof"),
                        PatternRule("void"),
                        PatternRule("range"),
                        PatternRule("string"),
                        PatternRule("line"),
                        PatternRule("ascii"),
                        PatternRule("utf8"),
                        PatternRule("predicate"),
                        PatternRule("call_rule"),
                        PatternRule("sub_pattern"),
                        PatternRule("code")])),
                    PatternBind("modifier", PatternMaybe(PatternRule("modifier"))),
                    PatternCode("""
if modifier != None:
    pattern = modifier(pattern)
if pnot != None:
    pattern = peg.PatternNot(pattern)
if ensure != None:
    pattern = peg.PatternEnsure(pattern)
value = pattern
""")]),
            ]),
        Rule("failure", [
            PatternSequence([
                PatternRule("whitespace"),
                PatternVerbatim("<fail>"),
                PatternRule("spaces"),
                PatternBind('code', PatternRule('code')),
                PatternCode("""value = code.code"""),
                ]),
            ]),
        Rule("line", [
            PatternSequence([
                PatternVerbatim("<line>"),
                PatternCode("""value = peg.PatternLine()""")
                ]),
            ]),
        Rule("predicate", [
            PatternSequence([
                PatternVerbatim("<predicate"),
                PatternRule("whitespace"),
                PatternBind('variable', PatternRule('word')),
                PatternRule("whitespace"),
                PatternVerbatim(">"),
                PatternRule("whitespace"),
                PatternBind('code', PatternRule('code')),
                PatternCode("value = peg.PatternPredicate(variable, code.code)"),
            ]),
        ]),
        Rule("utf8", [
            PatternSequence([
                PatternVerbatim("<utf8"),
                PatternRule("spaces"),
                PatternBind('num', PatternRule("hex_number")),
                PatternRule("spaces"),
                PatternVerbatim(">"),
                PatternCode("""value = peg.createUtf8Pattern(num)"""),
                ]),
            ]),
        Rule("ascii", [
            PatternSequence([
                PatternVerbatim("<ascii"),
                PatternRule("spaces"),
                PatternBind('num', PatternRule("number")),
                PatternRule("spaces"),
                PatternVerbatim(">"),
                PatternCode("""value = peg.PatternVerbatim(int(num))"""),
                ]),
            ]),
        Rule("call_rule", [
            PatternSequence([
               PatternVerbatim("@"),
               PatternBind('name', PatternRule("word")),
               PatternBind('rule_parameters', PatternMaybe(PatternRule('parameters_rules'))),
               PatternBind('parameters', PatternMaybe(PatternRule('parameters_values'))),
               PatternCode("""value = peg.PatternCallRule(name, rule_parameters, parameters)"""),
           ]),
        ]),
        Rule("eof", [
            PatternSequence([
                PatternVerbatim("<eof>"),
                PatternCode("""value = peg.PatternEof()""")
                ]),
            ]),
        Rule("void", [PatternSequence([
            PatternVerbatim("<void>"),
            PatternCode("""value = peg.PatternVoid()""")])
            ]),
        Rule("range", [
            PatternSequence([
                PatternVerbatim("["),
                PatternRepeatMany(PatternSequence([
                    PatternNot(PatternVerbatim("]")),
                    PatternAny(),
                    PatternCode("value = values[1]")])),
                PatternVerbatim("]"),
                PatternCode("""
value = peg.PatternRange(''.join(values[1]))
""")]),
            ]),
        Rule("sub_pattern", [
            PatternSequence([
                PatternVerbatim("("),
                PatternRepeatOnce(PatternRule("pattern")),
                PatternVerbatim(")"),
                PatternCode("""
value = peg.PatternSequence(values[1])
""")]),
            ]),
        Rule("bind", [
            PatternSequence([
                PatternBind("name", PatternRule("word")),
                PatternVerbatim(":"),
                PatternCode("""
value = lambda p: peg.PatternBind(name, p)
""")]),
            ]),
        Rule("string", [
            PatternSequence([
                PatternVerbatim("\""),
                PatternRepeatMany(PatternSequence([
                    PatternNot(PatternVerbatim("\"")),
                    PatternAny(),
                    PatternCode("value = values[1]")])),
                PatternVerbatim("\""),
                PatternBind("options", PatternMaybe(PatternVerbatim("{case}"))),
                PatternCode("""
value = peg.PatternVerbatim(''.join(values[1]), options)
""")]),
            PatternSequence([
                PatternVerbatim("<quote>"),
                PatternCode("""value = peg.PatternVerbatim('"')"""),
                ]),
            ]),
        Rule("modifier", [
            PatternSequence([PatternVerbatim("*"),
            PatternCode("""
value = lambda p: peg.PatternRepeatMany(p)
""")]),
            PatternSequence([PatternVerbatim("?"),
            PatternCode("""
value = lambda p: peg.PatternMaybe(p)
""")]),
            PatternSequence([PatternVerbatim("+"),
            PatternCode("""
value = lambda p: peg.PatternRepeatOnce(p)
""")]),
            ]),
        Rule("x_word", [
            PatternSequence([
                PatternBind('name', PatternRule("word")),
                PatternBind('rule_parameters', PatternMaybe(PatternRule('parameters_rules'))),
                PatternBind('parameters', PatternMaybe(PatternRule('parameters_values'))),
                PatternCode("""
value = peg.PatternRule(name, rule_parameters, parameters)
""")]),
            ]),
        Rule('rule_parameters', [
            PatternSequence([
                PatternVerbatim("["),
                PatternRule("spaces"),
                PatternBind('param1', PatternRule('word')),
                PatternBind('params', PatternRepeatMany(PatternSequence([
                    PatternRule('spaces'),
                    PatternVerbatim(','),
                    PatternRule('spaces'),
                    PatternBind('exp', PatternRule('word')),
                    PatternCode("""value = exp""")]))),
                PatternRule("spaces"),
                PatternVerbatim("]"),
                PatternCode("""value = [param1] + params""")]),
            ]),
        Rule('value_parameters', [
            PatternSequence([
                PatternVerbatim("("),
                PatternRule("spaces"),
                PatternBind('param1', PatternRule('word')),
                PatternBind('params', PatternRepeatMany(PatternSequence([
                    PatternRule('spaces'),
                    PatternVerbatim(','),
                    PatternRule('spaces'),
                    PatternBind('exp', PatternRule('word')),
                    PatternCode("""value = exp""")]))),
                PatternRule("spaces"),
                PatternVerbatim(")"),
                PatternCode("""value = [param1] + params""")]),
            ]),
        Rule('parameters_rules', [
            PatternSequence([
                PatternVerbatim("["),
                PatternRule("spaces"),
                PatternBind('param1', PatternRule('word_or_at')),
                PatternBind('params', PatternRepeatMany(PatternSequence([
                    PatternRule('spaces'),
                    PatternVerbatim(','),
                    PatternRule('spaces'),
                    PatternBind('exp', PatternRule('word_or_at')),
                    PatternCode("""value = exp""")]))),
                PatternRule("spaces"),
                PatternVerbatim("]"),
                PatternCode("""value = [param1] + params""")]),
            ]),

        Rule('parameters_values', [
            PatternSequence([
                PatternVerbatim("("),
                PatternRule("spaces"),
                PatternBind('param1', PatternRule('word_or_dollar')),
                PatternBind('params', PatternRepeatMany(PatternSequence([
                    PatternRule('spaces'),
                    PatternVerbatim(','),
                    PatternRule('spaces'),
                    PatternBind('exp', PatternRule('word_or_dollar')),
                    PatternCode("""value = exp""")]))),
                PatternRule("spaces"),
                PatternVerbatim(")"),
                PatternCode("""value = [param1] + params""")]),
            ]),
        Rule('word_or_dollar', [
            PatternRule('word'),
            PatternRule('dollar'),
            ]),
        Rule('word_or_at', [
            PatternRule('word'),
            PatternRule('word_at'),
            ]),
        Rule('word_at', [
            PatternSequence([
                PatternVerbatim("@"),
                PatternBind('word', PatternRule("word")),
                PatternCode("""value = '@%s' % word""")]),
            ]),
        Rule('dollar', [
            PatternSequence([
                PatternVerbatim("$"),
                PatternBind('number', PatternRule("number")),
                PatternCode("""value = "$%s" % number""")]),
            ]),
        Rule('number', [
            PatternSequence([
                PatternRepeatOnce(PatternRule('digit')),
                PatternCode("""value = ''.join(values[0])""")]),
            ]),
        Rule('digit', [PatternRange('0123456789')]),
        Rule('hex_number', [
            PatternSequence([
                PatternRepeatOnce(PatternRule('hex_digit')),
                PatternCode("""value = ''.join(values[0])""")]),
            ]),
        Rule('hex_digit', [PatternRange('0123456789abcdefABCDEF')]),
        Rule("start_symbol", [
            PatternSequence([
                PatternVerbatim("start-symbol:"),
                PatternRepeatMany(PatternRule("space")),
                PatternRule("word"),
                PatternCode("""value = values[2]""")]),
            ]),
        Rule("spaces", [PatternSequence([PatternRepeatMany(PatternRule("space"))])]),
        # Rule("space", [PatternRange(' \t')]),
        Rule("space", [PatternSequence([PatternVerbatim(" ")]), PatternSequence([PatternVerbatim("\\t")])]),
        Rule("any_char", [PatternRange('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890_-')]),
        Rule("any", [PatternSequence([
            PatternVerbatim("."),
            PatternCode("""value = peg.PatternAny()""")]),
            ]),
        Rule("whitespace", [
            PatternSequence([
                PatternRepeatMany(PatternOr([
                    PatternRange(" \\t\\n"),
                    PatternRule("comment"),
                    ])),
                ]),
            ]),
        Rule("comment", [
            PatternSequence([
                PatternVerbatim("#"),
                PatternUntil(PatternVerbatim("\\n")),
                ]),
            ]),
        Rule("newlines_one", [PatternRepeatOnce(PatternVerbatim("\\n"))]),
        Rule("newlines", [PatternRepeatMany(PatternVerbatim("\\n"))]),
    ]

    peg = Peg("start", None, None, [peg_name], rules, [])
    # print peg.generate_python()
    return peg

# Creates a sequence of <ascii #> patterns from a UTF8 code point
def createUtf8Pattern(pattern):
    def toUtf8(hex):
        unicode = int(hex, 16)
        if unicode < 128:
            return [unicode]
        elif unicode < 2047:
            byte1 = 192 + unicode / 64
            byte2 = 128 + unicode % 64
            return [byte1, byte2]
        elif unicode <= 65535:
            byte1 = 224 + unicode / 4096
            byte2 = 128 + (unicode / 64) % 64
            byte3 = 128 + unicode % 64
            return [byte1, byte2, byte3]
        elif unicode <= 2097151:
            byte1 = 240 + unicode / 262144
            byte2 = 128 + (unicode / 4096) % 64
            byte3 = 128 + (unicode / 64) % 64
            byte4 = 128 + (unicode % 64)
            return [byte1, byte2, byte3, byte4]
        elif unicode <= 67108863:
            byte1 = 248 + (unicode / 16777216)
            byte2 = 128 + ((unicode / 262144) % 64)
            byte3 = 128 + ((unicode / 4096) % 64)
            byte4 = 128 + ((unicode / 64) % 64)
            byte5 = 128 + (unicode % 64)
            return [byte1, byte2, byte3, byte4, byte5]
        elif unicode <= 2147483647:
            byte1 = 252 + (unicode / 1073741824)
            byte2 = 128 + ((unicode / 16777216) % 64)
            byte3 = 128 + ((unicode / 262144) % 64)
            byte4 = 128 + ((unicode / 4096) % 64)
            byte5 = 128 + ((unicode / 64) % 64)
            byte6 = 128 + (unicode % 64)
            return [byte1, byte2, byte3, byte4, byte5, byet6]
        raise Exception("Could not decode utf8 '%s'" % hex)

    return PatternSequence([PatternVerbatim(x) for x in toUtf8(pattern)])

def make_peg_parser(name = 'peg'):
    return create_peg(peg_bnf(name))
    # answer = parser('peg.in')
    # print "Got " + str(answer)
    # print answer.generate()
    # module = compile(peg.generate_python(), peg.namespace, 'exec')
    # print module

def convert_regex(regex):
    def writeFile(input, filename):
        out = open(filename, 'w')
        out.write(input)
        out.close()

    def parse_regex(input):
        regex_bnf = """
start-symbol: start
module: regex
code: {{
import regex
}}
rules:
    start = regex <eof> {{ value = $1 }}
    regex = (item:item modifier:modifier* {{ value = reduce(lambda a, b: b(a), modifier, item) }})* {{ value = regex.Sequence($1) }}
    item = "(" sub:regex ")" {{ value = sub }}
         | union
         | character

    modifier = "*" {{ value = lambda x: regex.Repeat(x) }} 
             | "?" {{ value = lambda x: regex.Maybe(x) }}

    union = "|" {{ value = regex.Union() }}
    character = !")" got:. {{ value = regex.Character(got) }} 
"""
        # Get the peg bnf parser
        parser = make_peg_parser()

        tempFile = 'regex_bnf'
        writeFile(regex_bnf, tempFile)
        
        # Parse the regex bnf grammar, get a python representation of the peg
        out = parser(tempFile)
        if out == None:
            raise Exception("Could not create regex parser!")
        # Create a new python module that implements the regex parser
        regex_parser = create_peg(out)
        if regex_parser == None:
            raise Exception("Could not create regex parser!")
        # Run the regex parser on the actual input
        tempFile2 = 'regex_input'
        writeFile(input, tempFile2)
        out = regex_parser(tempFile2)
        print "Parsed %s" % out
        return out

    def convert_to_peg(data):
        rules = []
        def convert_rule(data, combine):
            import regex
            if isinstance(data, regex.Character):
                return combine(PatternVerbatim(data.what))
            if isinstance(data, regex.Sequence):
                if len(data.stuff) == 0:
                    return combine(PatternVoid())
                first = data.stuff[0]
                more = data.stuff[1:]
                return convert_rule(first, lambda what: PatternSequence([what, convert_rule(regex.Sequence(more), combine)]))
            if isinstance(data, regex.Repeat):
                repeat = gensym('repeat')
                more = [convert_rule(data.what, lambda x: x)]
                rules.append(Rule(repeat, more))
                return combine(PatternRepeatMany(PatternRule(repeat)))
            if isinstance(data, regex.Maybe):
                return combine(PatternMaybe(convert_rule(data.what, lambda x: x)))
            if isinstance(data, regex.Union):
                more = [convert_rule(sub, combine) for sub in data.stuff]
                follow = gensym('or')
                rules.append(Rule(follow, more))
                return PatternRule(follow)
            raise Exception("dont know: %s" % data)

        def add_rule(name, data):
            rule = Rule(name, [convert_rule(data, lambda x: x)])
            rules.append(rule)

        start = gensym('start')
        add_rule(start, data)
        return Peg(start, None, None, 'regex1', rules, None)
        
    print "Convert %s" % regex
    parsed = parse_regex(regex)
    peg = convert_to_peg(parsed)
    print peg.generate_bnf()
    exit(0)

# test()
# test2()

def help_syntax():
    print "start-symbol: <name>"
    print "rules:"
    print "  <name> = <pattern> | <pattern> ... "
    print
    print "A Pattern can be:"
    print "  \"<literal>\""
    print "  <name of rule>"
    print "  pattern*"
    print "  pattern?"
    print "  pattern+"
    print "  [<characters>]"
    print "  <eof>"
    print
    print "BNF grammar for a peg grammar"
    print peg_bnf('peg').generate_bnf()

def help():
    print "Options:"
    print "-h,--help,help : Print this help"
    print "--help-syntax : Explain syntax of BNF (Backus-Naur form) for grammar files"
    print "--bnf : Generate BNF description (grammar language)"
    print "--ruby : Generate Ruby parser"
    print "--python : Generate Python parser"
    print "--cpp,--c++ : Generate C++ parser"
    print "--h : Generate C++ header for the C++ functions"
    # print "--c++-interpreter : Generate a C++ parser that uses an interpreter"
    print "--save=filename : Save all generated parser output to a file, 'filename'"
    print "--peg-name=name : Name the peg module 'name'. The intermediate peg module will be written as peg_<name>.py. Defaults to 'peg'."

# make_peg_parser()
if __name__ == '__main__':
    import sys
    import re
    doit = []
    file = None
    helped = 0
    def default_peg():
        return make_peg_parser()
    peg_maker = default_peg
    save_re = re.compile('--save=(.*)')
    separate_rules_re = re.compile('--separate-rules=(.*)')
    list_separate_rules_re = re.compile('--list-separate-rules=(.*)')
    peg_name_re = re.compile('--peg-name=(.*)')
    regex_re = re.compile('--regex=(.*)')
    def print_it(p):
        print p
    do_output = print_it
    do_close = lambda : 0
    return_code = 0
    parallel = [False]
    separate = [None]
    for arg in sys.argv[1:]:
        if arg == '--bnf':
            doit.append(lambda p: p.generate_bnf())
        elif arg == '--cpp' or arg == '--c++':
            doit.append(lambda p: cpp_generator.generate(p, parallel[0], separate[0]))
        elif arg == '--h':
            doit.append(lambda p: cpp_header_generator.generate(p))
        #elif arg == '--c++-interpreter':
        #    doit.append(lambda p: p.generate_cpp_interpreter())
        elif arg == '--ruby':
            doit.append(lambda p: ruby_generator.generate(p))
        elif arg == '--python':
            doit.append(lambda p: python_generator.generate(p))
        elif arg == '--test':
            doit.append(lambda p: p.generate_test())
        elif arg == '--parallel':
            parallel[0] = True
        elif regex_re.match(arg):
            all = regex_re.match(arg)
            regex = all.group(1)
            convert_regex(regex)
        elif arg == "--help-syntax":
            help_syntax()
        elif peg_name_re.match(arg):
            all = peg_name_re.match(arg)
            name = all.group(1)
            def make_peg():
                return make_peg_parser(name)
            peg_maker = make_peg
        elif separate_rules_re.match(arg):
            all = separate_rules_re.match(arg)
            separate[0] = all.group(1)
        elif list_separate_rules_re.match(arg):
            all = list_separate_rules_re.match(arg)
            doit.append(lambda p: p.print_list_files(all.group(1)))
        elif save_re.match(arg):
            all = save_re.match(arg)
            fout = open(all.group(1), 'w')
            do_close = lambda : fout.close()
            def save(p):
                fout.write(p)
            do_output = save
        elif arg == '-h' or arg == '--help' or arg == 'help':
            help()
            helped = 1
        else:
            file = arg
    
    if file != None:
        parser = peg_maker()
        out = parser(file)
        # print out
        if out != None:
            if len(doit) == 0:
                print "Grammar file '%s' looks good!. Use some options to generate a peg parser. -h will list all available options." % file
            else:
                for generate in doit:
                    do_output(generate(out))
        else:
            print "Uh oh, couldn't parse " + file + ". Are you sure its using BNF format?"
            return_code = 1
    else:
        if helped == 0:
            help()
            print "Give a BNF grammar file as an argument"

    do_close()
    exit(return_code)

# Done
# memoize in python parsers
# include arbitrary code at the top of the file (python, c++, bnf)
# fix binding variables in c++ (move declaration to the top of the function)
# make intra-pattern actions work
# add helper function section
# error reporting for c++
# generator for python, ruby, c++
# getter for the current line and column
# custom error reporting length, options: error-length 40
# If a rule has a <fail> then catch a parsing exception and call the fail function
# Predicates: sequences of host code that evaluate to true/false where the current
#   rule stops on false and continues on true.
