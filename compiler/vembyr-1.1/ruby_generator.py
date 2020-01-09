from core import CodeGenerator, gensym, newResult, indent, special_char

start_ruby = """

def special_escape(s)
    return s.replace("\\\\n", "\\\\\\\\n").replace("\\\\t", "\\\\\\\\t").replace("\\\"", '\\\\\\\"').replace("\\\\r", "\\\\\\\\r")
end

class PegError < Exception
end

class NotError < Exception
end

class Result
    attr_reader :values, :position

    def initialize(position)
        @position = position
        @values = []
    end

    def getPosition()
        return @position
    end

    def nextPosition(amount = 1)
        @position += amount
    end

    def setValue(value)
        @values = value
    end

    def getLastValue()
        if @values.is_a?(Array)
            if @values.size() > 0
                return @values[-1]
            else
                return nil
            end
        end
        return @values
    end
    
    def matches()
        return @values.size
    end

    def getValues()
        return @values
    end

    def addResult(him)
        @values << him.values
        @position = him.position
    end
    
    #def extendResult(self, him):
    #    self.values.extend(him.values)
    #    self.position = him.position
end

class Stream
    def initialize(filename)
        @file = File.new(filename, 'r')
        @position = 0
        @limit = 100
        @furthest = 0
        @all = @file.read()
        @memo = {}
        # print "Read " + str(len(self.all))
    end

    def close()
        @file.close()
    end

    def get(position, number = 1)
        if position + number > @limit
            # print (position + number)
            @limit += 5000
        end
        if position + number > @all.size
            return 0.chr()
        end
        # print "stream: %s" % self.all[position:position+number]
        return @all[position...position+number]
    end

    def reportError()
        line = 1
        column = 1
        for i in 0..@furthest
            if @all[i] == '\\n'
                line += 1
                column = 1
            else
                column += 1
            end
        end
        context = 10
        left = @furthest - context
        right = @furthest + context
        if left < 0
            left = 0
        end
        if right > @all.size
            right = @all.size
        end
        puts "Read up till line #{line}, column #{column}"
        puts special_escape(@all[left...right])
        puts (' ' * (@furthest - left)) + "^"
    end

    def update(rule, position, result)
        if result != nil and result.getPosition() > @furthest
            @furthest = result.getPosition()
        end

        for_rule = nil
        if @memo.has_key? rule
            for_rule = @memo[rule]
        else
            @memo[rule] = {}
            for_rule = @memo[rule]
        end
        
        for_position = nil
        if for_rule.has_key? position
            for_position = for_rule[position]
        else
            for_rule[position] = nil
        end
        for_rule[position] = result
    end

    def hasResult(rule, position)
        @memo.has_key?(rule) and @memo[rule].has_key?(position)
        # return @memo.has_key?(rule) and @memo[rule].has_key?(position)
    end

    def result(rule, position)
        return @memo[rule][position]
    end
end
"""


class RubyGenerator(CodeGenerator):
    def fixup_ruby(self, code, how):
        import re
        fix = re.compile("\$(\d+)")
        return re.sub(fix, how, code)

    def generate_sequence(me, pattern, result, previous_result, stream, failure):
        data = ""
        for apattern in pattern.patterns:
            my_result = newResult()
            data += """
%s = Result.new(%s.getPosition())
%s
%s.addResult(%s)
""" % (my_result, result, apattern.generate_v1(me, my_result, result, stream, failure), result, my_result)

        return data + """
%s.setValue(%s.getLastValue())
""" % (result, result)

    # this breaks when the sub-pattern is a PatternSequence, todo: fix it
    def generate_maybe(me, pattern, result, previous_result, stream, failure):
        save = gensym("save")
        fail = lambda : """
%s = Result.new(%s)
%s.setValue(nil)
""" % (result, save, result)

        data = """
%s = %s.getPosition()
%s
""" % (save, result, pattern.pattern.generate_v1(me, result, previous_result, stream, fail))
        return data

    def generate_repeat_many(me, pattern, result, previous_result, stream, failure):
        my_fail = lambda : "raise PegError"
        my_result = newResult()
        data = """
begin
    while true
        %s = Result.new(%s.getPosition())
        %s
        %s.addResult(%s)
    end
rescue PegError
end
        """ % (my_result, result, indent(indent(pattern.next.generate_v1(me, my_result, result, stream, my_fail).strip())), result, my_result)

        return data

    def generate_rule(me, pattern, result, previous_result, stream, failure):
        def fix(v):
            return "%s.getValues()[%s]" % (previous_result, int(v.group(1)) - 1)
        def change(arg):
            if arg.startswith('@'):
                return arg[1:]
            return 'lambda{|*args| rule_%s(*args)}' % arg
        rule_parameters = ""
        if pattern.rules != None:
            rule_parameters = ", %s" % ", ".join([change(f) for f in pattern.rules])

        parameters = ""
        if pattern.parameters != None:
            parameters = ", %s" % ",".join([me.fixup_ruby(p, fix) for p in pattern.parameters])
        data = """
# puts "Trying rule '%s'"
%s = rule_%s(%s, %s.getPosition()%s%s)
if %s == nil
    %s
end
""" % (pattern.rule, result, pattern.rule, stream, result, rule_parameters, parameters, result, indent(failure()))

        return data

    def generate_repeat_once(me, pattern, result, previous_result, stream, failure):
        my_fail = lambda : "raise PegError"
        my_result = newResult()
        my_result2 = newResult()
        data = """
begin
    while (true)
        %s = Result.new(%s.getPosition())
        %s
        %s.addResult(%s)
    end
rescue PegError
    if %s.matches() == 0
        %s
    end
end
        """ % (my_result, result, indent(indent(pattern.next.generate_v1(me, my_result, result, stream, my_fail).strip())), result, my_result, result, failure())

        return data

    def generate_void(me, pattern, result, previous_result, stream, failure):
        return ""

    def generate_verbatim(me, pattern, result, previous_result, stream, failure):
        def doString():
            length = len(pattern.letters)
            if special_char(pattern.letters):
                length = 1
            data = """
if '%s' == %s.get(%s.getPosition(), %s) then
    %s.nextPosition(%s)
    %s.setValue('%s')
else
    %s
end
""" % (pattern.letters, stream, result, length, result, length, result, pattern.letters, indent(failure()))
            return data
        def doAscii():
            data = """
if %s.get(%s.getPosition()).ord() == %s then
    %s.nextPosition()
    %s.setValue(%s)
else
    %s
end
"""
            return data % (stream, result, pattern.letters, result, result, pattern.letters, indent(failure()))
        if type(pattern.letters) == type('x'):
            return doString()
        elif type(pattern.letters) == type(0):
            return doAscii()
        else:
            raise Exception("unknown verbatim value %s" % pattern.letters)

    def generate_ensure(me, pattern, result, previous_result, stream, failure):
        my_result = newResult()
        data = """
%s = Result.new(%s.getPosition())
%s
""" % (my_result, result, pattern.next.generate_v1(me, my_result, result, stream, failure).strip())
        return data

    def generate_not(me, pattern, result, previous_result, stream, failure):
        my_result = newResult()
        my_fail = lambda : "raise NotError"
        data = """
%s = Result.new(%s.getPosition())
begin
    %s
    %s
rescue NotError
    %s.setValue(nil)
end
        """ % (my_result, result, indent(pattern.next.generate_v1(my_result, result, stream, my_fail).strip()), failure(), result)

        return data

    def generate_any(me, pattern, result, previous_result, stream, failure):
        temp = gensym()
        data = """
%s = %s.get(%s.getPosition())
if %s != 0.chr() then
    %s.setValue(%s)
    %s.nextPosition()
else
    %s
end
""" % (temp, stream, result, temp, result, temp, result, indent(failure()))
        return data

    def generate_range(me, pattern, result, previous_result, stream, failure):
        letter = gensym("letter")
        data = """
%s = %s.get(%s.getPosition())
if '%s'.index(%s) != nil then
    %s.nextPosition()
    %s.setValue(%s)
else
    %s
end
""" % (letter, stream, result, pattern.range, letter, result, result, letter, indent(failure()))
        return data

    def generate_eof(me, pattern, result, previous_result, stream, failure):
        data = """
if 0.chr() == %s.get(%s.getPosition()) then
    %s.nextPosition()
    %s.setValue(0.chr())
else
    %s
end
""" % (stream, result, result, result, indent(failure()))
        return data

    def generate_code(me, pattern, result, previous_result, stream, failure):
        data = """
value = nil
values = %s.getValues()
%s
%s.setValue(value)
""" % (previous_result, me.fixup_ruby(pattern.code.strip(), lambda v: "values[%s]" % (int(v.group(1)) - 1)), result)

        return data


    def generate_bind(me, pattern, result, previous_result, stream, failure):
        data = """
%s
%s = %s.getValues()
""" % (pattern.pattern.generate_v1(me, result, previous_result, stream, failure).strip(), pattern.variable, result)
        return data

    def generate_call_rule(me, pattern, result, previous_result, stream, failure):
        def fix(v):
            return "%s.getValues()[%s]" % (previous_result, int(v.group(1)) - 1)
        def change(arg):
            if arg.startswith('@'):
                return arg[1:]
            return 'lambda{|*args| rule_%s(*args)}' % arg
        rule_parameters = ""
        if pattern.rules != None:
            rule_parameters = ", %s" % ", ".join([change(f) for f in pattern.rules])

        parameters = ""
        if pattern.values != None:
            parameters = ", %s" % ",".join([me.fixup_ruby(p, fix) for p in pattern.values])
        data = """
# print "Trying rule " + '%s'
%s = %s.call(%s, %s.getPosition()%s%s)
if %s == nil
    %s
end
""" % (pattern.name, result, pattern.name, stream, result, rule_parameters, parameters, result, indent(failure()))

        return data

def generate(self):
    use_rules = self.rules
    rule_numbers = '\n'.join(["RULE_%s = %d" % (x[0].name, x[1]) for x in zip(use_rules, range(0, len(use_rules)))])

    data = """
%s

%s

%s

def parse(file)
    stream = Stream.new(file)
    out = rule_%s(stream, 0)
    stream.close()
    return out.getValues()
end
""" % (start_ruby, rule_numbers, '\n'.join([rule.generate_ruby() for rule in self.rules]), self.start)
    return data
