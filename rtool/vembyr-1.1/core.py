# Default class that generates code given some Peg description. Sub-classes
# should implement every generate_* method
class CodeGenerator:
    def __init__(self):
        pass

    def fail(self):
        raise Exception("this method has not been implemented yet")

    def generate_not(self, *args):
        self.fail()

    def generate_ensure(self, *args):
        self.fail()

    def generate_rule(self, *args):
        self.fail()

    def generate_void(self, *args):
        self.fail()
    
    def generate_predicate(me, *args):
        self.fail()

    def generate_eof(self, *args):
        self.fail()

    def generate_sequence(self, *args):
        self.fail()

    def generate_repeat_once(self, *args):
        self.fail()

    def generate_code(self, *args):
        self.fail()

    def generate_repeat_many(self, *args):
        self.fail()

    def generate_any(self, *args):
        self.fail()

    def generate_maybe(self, *args):
        self.fail()

    def generate_or(self, *args):
        self.fail()

    def generate_bind(self, *args):
        self.fail()

    def generate_range(self, *args):
        self.fail()

    def generate_verbatim(self, *args):
        self.fail()

    def generate_line(self, *args):
        self.fail()
    
    def generate_call_rule(self, *args):
        self.fail()

# create a variable name
next_var = 0
def nextVar():
    global next_var;
    next_var += 1;
    return "peg_%d" % next_var

def resetGensym():
    global next_var
    next_var = 0

# create a variable using the argument as a prefix
def gensym(what = "temp"):
    return "%s_%s" % (what, nextVar())

def newResult():
    return gensym("result")

def newOut():
    return gensym("out")

def indent(s):
    space = '    '
    return s.replace('\n', '\n%s' % space)

def special_char(s):
    return s in ["\\n", "\\t", "\\r"]

# Getting values from chunks
class Accessor:
    def __init__(self, chunk, value, type, rule):
        self.chunk = chunk
        self.value = value
        self.rule = rule
        self.type = type

    def getChunk(self, code):
        return code + self.chunk

    def getType(self):
        return self.type

    def getValue(self, code):
        return code + self.value

