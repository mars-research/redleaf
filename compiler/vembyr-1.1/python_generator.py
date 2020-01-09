from core import CodeGenerator, gensym, newResult, indent, special_char

start_python = """

def special_escape(s):
    return s.replace("\\\\n", "\\\\\\\\n").replace("\\\\t", "\\\\\\\\t").replace("\\\"", '\\\\\\\"').replace("\\\\r", "\\\\\\\\r")

class PegError(Exception):
    def __init__(self):
        Exception.__init__(self)

class NotError(Exception):
    def __init__(self):
        Exception.__init__(self)

class Result:
    def __init__(self, position):
        self.position = position
        self.values = []

    def getPosition(self):
        return self.position

    def nextPosition(self, amount = 1):
        self.position += amount

    def setValue(self, value):
        self.values = value

    def getLastValue(self):
        if type(self.values) is list:
            if len(self.values) > 0:
                return self.values[-1]
            else:
                return None
        return self.values
    
    def matches(self):
        return len(self.values)

    def getValues(self):
        return self.values

    def addResult(self, him):
        self.values.append(him.values)
        self.position = him.position
    
    #def extendResult(self, him):
    #    self.values.extend(him.values)
    #    self.position = him.position

class Stream:
    def __init__(self, filename = None, input = None):
        def read():
            file = open(filename, 'r')
            out = file.read()
            file.close()
            return out
        self.position = 0
        self.limit = 100
        self.furthest = 0
        self.memo = {}
        if filename != None:
            self.all = read()
        elif input != None:
            self.all = input
        else:
            raise PegError("Pass a filename or input")
        # print "Read " + str(len(self.all))

    def get(self, position, number = 1):
        if position + number > self.limit:
            # print (position + number)
            self.limit += 5000
        if position + number > len(self.all):
            return chr(0)
        # print "stream: %s" % self.all[position:position+number]
        return self.all[position:position+number]

    def get2(self, position):
        if position != self.position:
            self.file.seek(position)
        self.position = position + 1
        if position > self.limit:
            print position
            self.limit += 5000
        return self.file.read(1)

    def reportError(self):
        line = 1
        column = 1
        for i in xrange(0, self.furthest):
            if self.all[i] == '\\n':
                line += 1
                column = 1
            else:
                column += 1
        context = 10
        left = self.furthest - context
        right = self.furthest + context
        if left < 0:
            left = 0
        if right > len(self.all):
            right = len(self.all)
        print "Read up till line %d, column %d" % (line, column)
        print "'%s'" % special_escape(self.all[left:right])
        print "%s^" % (' ' * (self.furthest - left))

    def update(self, rule, position, result):
        if result != None and result.getPosition() > self.furthest:
            self.furthest = result.getPosition()

        for_rule = None
        try:
            for_rule = self.memo[rule]
        except KeyError:
            self.memo[rule] = {}
            for_rule = self.memo[rule]
        
        for_position = None
        try:
            for_position = for_rule[position]
        except KeyError:
            for_rule[position] = None

        for_rule[position] = result

    def hasResult(self, rule, position):
        try:
            x = self.memo[rule][position]
            return True
        except KeyError:
            return False

    def result(self, rule, position):
        return self.memo[rule][position]

"""


class PythonGenerator(CodeGenerator):
    def fixup_python(self, code, how):
        import re
        fix = re.compile("\$(\d+)")
        # return re.sub(fix, r"values[\1-1]", code)
        # return re.sub(fix, r"(\1-1)", code)
        return re.sub(fix, how, code)

    def generate_ensure(me, pattern, result, previous_result, stream, failure):
        my_result = newResult()
        data = """
%s = Result(%s.getPosition())
%s
""" % (my_result, result, pattern.next.generate_python(my_result, result, stream, failure).strip())
        return data

    def generate_not(me, pattern, result, previous_result, stream, failure):
        my_result = newResult()
        my_fail = lambda : "raise NotError"
        data = """
%s = Result(%s.getPosition());
try:
    %s
    %s
except NotError:
    %s.setValue(None)
        """ % (my_result, result, indent(pattern.next.generate_python(my_result, result, stream, my_fail).strip()), failure(), result)

        return data

    def generate_rule(me, pattern, result, previous_result, stream, failure):
        def fix(v):
            return "%s.getValues()[%s]" % (previous_result, int(v.group(1)) - 1)
        def change(arg):
            if arg.startswith('@'):
                return arg[1:]
            return 'rule_%s' % arg
        rule_parameters = ""
        if pattern.rules != None:
            rule_parameters = ", %s" % ", ".join([change(f) for f in pattern.rules])
        parameters = ""
        if pattern.parameters != None:
            parameters = ", %s" % ", ".join([me.fixup_python(p, fix) for p in pattern.parameters])
        data = """
# print "Trying rule " + '%s'
%s = rule_%s(%s, %s.getPosition()%s%s)
if %s == None:
    %s
""" % (pattern.rule, result, pattern.rule, stream, result, rule_parameters, parameters, result, indent(failure()))

        return data

    def generate_eof(me, pattern, result, previous_result, stream, failure):
        data = """
if chr(0) == %s.get(%s.getPosition()):
    %s.nextPosition()
    %s.setValue(chr(0))
else:
    %s
""" % (stream, result, result, result, indent(failure()))
        return data
    
    def generate_call_rule(me, pattern, result, previous_result, stream, failure):
        def fix(v):
            return "%s.getValues()[%s]" % (previous_result, int(v.group(1)) - 1)
        def change(arg):
            if arg.startswith('@'):
                return arg[1:]
            return 'rule_%s' % arg
        rule_parameters = ""
        if pattern.rules != None:
            rule_parameters = ", %s" % ", ".join([change(f) for f in pattern.rules])

        parameters = ""
        if pattern.values != None:
            parameters = ", %s" % ",".join([me.fixup_python(p, fix) for p in pattern.values])
        data = """
# print "Trying rule " + '%s'
%s = %s(%s, %s.getPosition()%s%s)
if %s == None:
    %s
""" % (pattern.name, result, pattern.name, stream, result, rule_parameters, parameters, result, indent(failure()))

        return data


    def generate_sequence(me, pattern, result, previous_result, stream, failure):
        data = ""
        for apattern in pattern.patterns:
            my_result = newResult()
            data += """
%s = Result(%s.getPosition())
%s
%s.addResult(%s);
""" % (my_result, result, apattern.generate_python(my_result, result, stream, failure), result, my_result)

        return data + """
%s.setValue(%s.getLastValue())
""" % (result, result)

    def generate_repeat_once(me, pattern, result, previous_result, stream, failure):
        my_fail = lambda : "raise PegError"
        my_result = newResult()
        my_result2 = newResult()
        data = """
try:
    while True:
        %s = Result(%s.getPosition());
        %s
        %s.addResult(%s);
except PegError:
    if %s.matches() == 0:
        %s
        """ % (my_result, result, indent(indent(pattern.next.generate_python(my_result, result, stream, my_fail).strip())), result, my_result, result, failure())

        return data

    def generate_code(me, pattern, result, previous_result, stream, failure):
        data = """
value = None
values = %s.getValues()
%s
%s.setValue(value)
""" % (previous_result, me.fixup_python(pattern.code.strip(), lambda v: "values[%s]" % (int(v.group(1)) - 1)), result)

        return data

    def generate_repeat_many(me, pattern, result, previous_result, stream, failure):
        my_fail = lambda : "raise PegError"
        my_result = newResult()
        data = """
try:
    while True:
        %s = Result(%s.getPosition());
        %s
        %s.addResult(%s);
except PegError:
    pass
        """ % (my_result, result, indent(indent(pattern.next.generate_python(my_result, result, stream, my_fail).strip())), result, my_result)

        return data

    def generate_any(me, pattern, result, previous_result, stream, failure):
        temp = gensym()
        data = """
%s = %s.get(%s.getPosition())
if %s != chr(0):
    %s.setValue(%s)
    %s.nextPosition()
else:
    %s
""" % (temp, stream, result, temp, result, temp, result, indent(failure()))
        return data

    # this breaks when the sub-pattern is a PatternSequence, todo: fix it
    def generate_maybe(me, pattern, result, previous_result, stream, failure):
        save = gensym("save")
        fail = lambda : """
%s = Result(%s)
%s.setValue(None)
""" % (result, save, result)

        data = """
%s = %s.getPosition()
%s
""" % (save, result, pattern.pattern.generate_python(result, previous_result, stream, fail))
        return data

    def generate_or(me, pattern, result, previous_result, stream, failure):
        data = ""
        fail = failure
        save = gensym("save")
        for next_pattern in pattern.patterns[::-1]:
            my_result = newResult()
            data = """
%s = Result(%s)
%s
""" % (result, save, next_pattern.generate_python(result, previous_result, stream, fail).strip())
            fail = lambda : data
        return """
%s = %s.getPosition()
%s
""" % (save, result, data)

    def generate_bind(me, pattern, result, previous_result, stream, failure):
        data = """
%s
%s = %s.getValues()
""" % (pattern.pattern.generate_python(result, previous_result, stream, failure).strip(), pattern.variable, result)
        return data

    def generate_range(me, pattern, result, previous_result, stream, failure):
        letter = gensym("letter")
        data = """
%s = %s.get(%s.getPosition())
if %s in '%s':
    %s.nextPosition()
    %s.setValue(%s)
else:
    %s
""" % (letter, stream, result, letter, pattern.range, result, result, letter, indent(failure()))

        return data

    def generate_verbatim(me, pattern, result, previous_result, stream, failure):
        def doString():
            length = len(pattern.letters)
            if special_char(pattern.letters):
                length = 1
            import re
            letters = re.sub(r"'", "\\'", pattern.letters)
            data = """
if '%s' == %s.get(%s.getPosition(), %s):
    %s.nextPosition(%s)
    %s.setValue('%s')
else:
    %s
""" % (letters, stream, result, length, result, length, result, letters, indent(failure()))
            return data
        def doAscii():
            data = """
if ord(%s.get(%s.getPosition())) == %s:
    %s.nextPosition()
    %s.setValue(%s);
else:
    %s
"""
            return data % (stream, result, pattern.letters, result, result, pattern.letters, indent(failure()))
        if type(pattern.letters) == type('x'):
            return doString()
        elif type(pattern.letters) == type(0):
            return doAscii()
        else:
            raise Exception("unknown verbatim value %s" % pattern.letters)

def generate(self):
    # use_rules = [rule for rule in self.rules if not rule.isInline()]
    use_rules = self.rules
    rule_numbers = '\n'.join(["RULE_%s = %d" % (x[0].name, x[1]) for x in zip(use_rules, range(0, len(use_rules)))])

    top_code = ""
    if self.include_code != None:
        top_code = self.include_code

    more_code = ""
    if self.more_code != None:
        more_code = self.more_code

    data = """
import peg

%s

%s

%s

%s

%s

def doParse(stream):
    done = rule_%s(stream, 0)
    if done == None:
        # print "Error parsing " + file
        stream.reportError()
        return None
    else:
        return done.getValues()

def parseFile(file):
    # print "Parsing " + file
    return doParse(Stream(filename = file))

def parseString(value):
    return doParse(Stream(input = value))

""" % (top_code, start_python, rule_numbers, more_code, '\n'.join([rule.generate_python() for rule in self.rules]), self.start)

    return data

