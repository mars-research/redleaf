import core
from core import newResult, indent, gensym, special_char, newOut, Accessor

start_cpp_code = """
struct Value{
    typedef std::list<Value>::const_iterator iterator;

    Value():
        which(1),
        value(0){
    }

    Value(const Value & him):
    which(him.which),
    value(0){
        if (him.isData()){
            value = him.value;
        }
        if (him.isList()){
            values = him.values;
        }
    }

    explicit Value(const void * value):
        which(0),
        value(value){
    }

    Value & operator=(const Value & him){
        which = him.which;
        if (him.isData()){
            value = him.value;
        }
        if (him.isList()){
            values = him.values;
        }
        return *this;
    }

    Value & operator=(const void * what){
        this->value = what;
        return *this;
    }

    void reset(){
        this->value = 0;
        this->values.clear();
        this->which = 1;
    }

    int which; // 0 is value, 1 is values

    inline bool isList() const {
        return which == 1;
    }

    inline bool isData() const {
        return which == 0;
    }

    inline const void * getValue() const {
        return value;
    }

    inline void setValue(const void * value){
        which = 0;
        this->value = value;
    }

    inline const std::list<Value> & getValues() const {
        return values;
    }

    /*
    inline void setValues(std::list<Value> values){
        which = 1;
        values = values;
    }
    */

    const void * value;
    std::list<Value> values;
};

class Result{
public:
    Result():
    position(-2){
    }

    Result(const int position):
    position(position){
    }

    Result(const Result & r):
    position(r.position),
    value(r.value){
    }

    Result & operator=(const Result & r){
        position = r.position;
        value = r.value;
        return *this;
    }

    void reset(){
        value.reset();
    }

    void setPosition(int position){
        this->position = position;
    }

    inline int getPosition() const {
        return position;
    }

    inline bool error(){
        return position == -1;
    }

    inline bool calculated(){
        return position != -2;
    }

    inline void nextPosition(){
        position += 1;
    }

    void setError(){
        position = -1;
    }

    inline void setValue(const Value & value){
        this->value = value;
    }

    /*
    Value getLastValue() const {
        if (value.isList()){
            if (value.values.size() == 0){
                std::cout << "[peg] No last value to get!" << std::endl;
            }
            return value.values[value.values.size()-1];
        } else {
            return value;
        }
    }
    */

    inline int matches() const {
        if (value.isList()){
            return this->value.values.size();
        } else {
            return 1;
        }
    }

    inline const Value & getValues() const {
        return this->value;
    }

    void addResult(const Result & result){
        std::list<Value> & mine = this->value.values;
        mine.push_back(result.getValues());
        this->position = result.getPosition();
        this->value.which = 1;
    }

private:
    int position;
    Value value;
};

%s

class ParseException: std::exception {
public:
    ParseException(const std::string & reason):
    std::exception(),
    line(-1), column(-1),
    message(reason){
    }

    ParseException(const std::string & reason, int line, int column):
    std::exception(),
    line(line), column(column),
    message(reason){
    }

    std::string getReason() const;
    int getLine() const;
    int getColumn() const;

    virtual ~ParseException() throw(){
    }

protected:
    int line, column;
    std::string message;
};

class Stream{
public:
    struct LineInfo{
        LineInfo(int line, int column):
        line(line),
        column(column){
        }

        LineInfo(const LineInfo & copy):
        line(copy.line),
        column(copy.column){
        }

        LineInfo():
        line(-1),
        column(-1){
        }

        int line;
        int column;
    };

public:
    /* read from a file */
    Stream(const std::string & filename):
    temp(0),
    buffer(0),
    farthest(0),
    last_line_info(-1){
        std::ifstream stream;
        /* ios::binary is needed on windows */
        stream.open(filename.c_str(), std::ios::in | std::ios::binary);
        if (stream.fail()){
            std::ostringstream out;
            out << __FILE__  << " cannot open '" << filename << "'";
            throw ParseException(out.str());
        }
        stream.seekg(0, std::ios_base::end);
        max = stream.tellg();
        stream.seekg(0, std::ios_base::beg);
        temp = new char[max];
        stream.read(temp, max);
        buffer = temp;
        stream.close();

        line_info[-1] = LineInfo(1, 1);

        createMemo();
    }

    /* for null-terminated strings */
    Stream(const char * in):
    temp(0),
    buffer(in),
    farthest(0),
    last_line_info(-1){
        max = strlen(buffer);
        line_info[-1] = LineInfo(1, 1);
        createMemo();
    }

    /* user-defined length */
    Stream(const char * in, int length):
    temp(0),
    buffer(in),
    farthest(0),
    last_line_info(-1){
        max = length;
        line_info[-1] = LineInfo(1, 1);
        createMemo();
    }

    void createMemo(){
        memo_size = 1024 * 2;
        memo = new Column*[memo_size];
        /* dont create column objects before they are needed because transient
         * productions will never call for them so we can save some space by
         * not allocating columns at all.
         */
        memset(memo, 0, sizeof(Column*) * memo_size);
        /*
        for (int i = 0; i < memo_size; i++){
            memo[i] = new Column();
        }
        */
    }

    int length(){
        return max;
    }

    /* prints statistics about how often rules were fired and how
     * likely rules are to succeed
     */
    void printStats(){
        double min = 1;
        double max = 0;
        double average = 0;
        int count = 0;
        for (int i = 0; i < length(); i++){
            Column & c = getColumn(i);
            double rate = (double) c.hitCount() / (double) c.maxHits();
            if (rate != 0 && rate < min){
                min = rate;
            }
            if (rate > max){
                max = rate;
            }
            if (rate != 0){
                average += rate;
                count += 1;
            }
        }
        std::cout << "Min " << (100 * min) << " Max " << (100 * max) << " Average " << (100 * average / count) << " Count " << count << " Length " << length() << " Rule rate " << (100.0 * (double)count / (double) length()) << std::endl;
    }

    char get(const int position){
        if (position >= max || position < 0){
            return '\\0';
        }

        // std::cout << "Read char '" << buffer[position] << "'" << std::endl;

        return buffer[position];
        /*
        char z;
        stream.seekg(position, std::ios_base::beg);
        stream >> z;
        return z;
        */
    }

    bool find(const char * str, const int position){
        if (position >= max || position < 0){
            return false;
        }
        return strncmp(&buffer[position], str, max - position) == 0;
    }

    void growMemo(){
        int newSize = memo_size * 2;
        Column ** newMemo = new Column*[newSize];
        memcpy(newMemo, memo, sizeof(Column*) * memo_size);
        memset(&newMemo[memo_size], 0, sizeof(Column*) * (newSize - memo_size));
        /*
        for (int i = memo_size; i < newSize; i++){
            newMemo[i] = new Column();
        }
        */
        delete[] memo;
        memo = newMemo;
        memo_size = newSize;
    }

    /* I'm sure this can be optimized. It only takes into account
     * the last position used to get line information rather than
     * finding a position closest to the one asked for.
     * So if the last position is 20 and the current position being requested
     * is 15 then this function will compute the information starting from 0.
     * If the information for 10 was computed then that should be used instead.
     * Maybe something like, sort the positions, find closest match lower
     * than the position and start from there.
     */
    LineInfo makeLineInfo(int last_line_position, int position){
        int line = line_info[last_line_position].line;
        int column = line_info[last_line_position].column;
        for (int i = last_line_position + 1; i < position; i++){
            if (buffer[i] == '\\n'){
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        return LineInfo(line, column);
    }

    void updateLineInfo(int position){
        if (line_info.find(position) == line_info.end()){
            if (position > last_line_info){
                line_info[position] = makeLineInfo(last_line_info, position);
            } else {
                line_info[position] = makeLineInfo(0, position);
            }
            last_line_info = position;
        }
    }

    const LineInfo & getLineInfo(int position){
        updateLineInfo(position);
        return line_info[position];
    }

    /* throws a ParseException */
    void reportError(const std::string & parsingContext){
        std::ostringstream out;
        int line = 1;
        int column = 1;
        for (int i = 0; i < farthest; i++){
            if (buffer[i] == '\\n'){
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        int context = %d;
        int left = farthest - context;
        int right = farthest + context;
        if (left < 0){
            left = 0;
        }
        if (right >= max){
            right = max;
        }
        out << "Error while parsing " << parsingContext << ". Read up till line " << line << " column " << column << std::endl;
        std::ostringstream show;
        for (int i = left; i < right; i++){
            char c = buffer[i];
            switch (buffer[i]){
                case '\\n' : {
                    show << '\\\\';
                    show << 'n';
                    break;
                }
                case '\\r' : {
                    show << '\\\\';
                    show << 'r';
                    break;
                }
                case '\\t' : {
                    show << '\\\\';
                    show << 't';
                    break;
                }
                default : show << c; break;
            }
        }
        out << "'" << show.str() << "'" << std::endl;
        for (int i = 0; i < farthest - left; i++){
            out << " ";
        }
        out << "^" << std::endl;
        out << "Last successful rule trace" << std::endl;
        out << makeBacktrace() << std::endl;
        throw ParseException(out.str(), line, column);
    }

    std::string makeBacktrace(){
        std::ostringstream out;

        bool first = true;
        for (std::vector<std::string>::iterator it = last_trace.begin(); it != last_trace.end(); it++){
            if (!first){
                out << " -> ";
            } else {
                first = false;
            }
            out << *it;
        }

        return out.str();
    }

    inline Column & getColumn(const int position){
        while (position >= memo_size){
            growMemo();
        }
        /* create columns lazily because not every position will have a column. */
        if (memo[position] == NULL){
            memo[position] = new Column();
        }
        return *(memo[position]);
    }

    void update(const int position){
        if (position > farthest){
            farthest = position;
            last_trace = rule_backtrace;
        }
    }

    void push_rule(const char * name){
        rule_backtrace.push_back(name);
    }

    void pop_rule(){
        rule_backtrace.pop_back();
    }

    ~Stream(){
        delete[] temp;
        for (int i = 0; i < memo_size; i++){
            delete memo[i];
        }
        delete[] memo;
    }

private:
    char * temp;
    const char * buffer;
    /* an array is faster and uses less memory than std::map */
    Column ** memo;
    int memo_size;
    int max;
    int farthest;
    std::vector<std::string> rule_backtrace;
    std::vector<std::string> last_trace;
    int last_line_info;
    std::map<int, LineInfo> line_info;
};

static int getCurrentLine(const Value & value){
    Stream::LineInfo * info = (Stream::LineInfo*) value.getValue();
    return info->line;
}

static int getCurrentColumn(const Value & value){
    Stream::LineInfo * info = (Stream::LineInfo*) value.getValue();
    return info->column;
}

class RuleTrace{
public:
    RuleTrace(Stream & stream, const char * name):
    stream(stream){
        stream.push_rule(name);
    }

    ~RuleTrace(){
        stream.pop_rule();
    }

    Stream & stream;
};

static inline bool compareChar(const char a, const char b){
    return a == b;
}

static inline char lower(const char x){
    if (x >= 'A' && x <= 'Z'){
        return x - 'A' + 'a';
    }
    return x;
}

static inline bool compareCharCase(const char a, const char b){
    return lower(a) == lower(b);
}
"""


# all the self parameters are named me because the code was originally
# copied from another class and to ensure that copy/paste errors don't
# occur I have changed the name from 'self' to 'me'
# that is, 'self' in the original code is now the parameter 'pattern'
class CppGenerator(core.CodeGenerator):
    def fixup_cpp(self, code, args):
        import re
        fix = re.compile("\$(\d+)")
        # return re.sub(fix, r"values.getValues()[\1-1]", code)
        return re.sub(fix, lambda obj: args(int(obj.group(1))) + ".getValues()", code)

    def generate_not(me, pattern, peg, result, stream, failure, tail, peg_args):
        not_label = gensym("not")
        my_result = newResult()
        my_fail = lambda : "goto %s;" % not_label
        data = """
Result %s(%s);
%s
%s
%s:
%s.setValue(Value((void*)0));
        """ % (my_result, result, pattern.next.generate_cpp(peg, my_result, stream, my_fail, None, peg_args).strip(), failure(), not_label, result)

        return data

    def generate_ensure(me, pattern, peg, result, stream, failure, tail, peg_args):
        my_result = newResult()
        data = """
Result %s(%s.getPosition());
%s
""" % (my_result, result, pattern.next.generate_cpp(peg, my_result, stream, failure, None, peg_args).strip())
        return data

    def generate_call_rule(me, pattern, peg, result, stream, failure, tail, peg_args):
        def change(arg):
            if arg.startswith('@'):
                return arg[1:]
            return 'rule_%s' % arg
        rule_parameters = ""
        if pattern.rules != None:
            rule_parameters = ", %s" % ", ".join([change(f) for f in pattern.rules])

        parameters = ""
        if pattern.values != None:
            parameters = ", %s" % ", ".join([me.fixup_cpp(p, peg_args) for p in pattern.values])
            # parameters = ", %s" % fix_param(pattern.parameters)

        def argify(name, many):
            if many == None or len(many) == 0:
                return ""
            return ", " + ",".join([name] * len(many))

        cast = "Result (*)(Stream &, const int%s%s)" % (argify('void *', pattern.rules), argify('Value', pattern.values))

        data = """
%s = ((%s) %s)(%s, %s.getPosition()%s%s);
if (%s.error()){
    %s
}
""" % (result, cast, pattern.name, stream, result, rule_parameters, parameters, result, indent(failure()))

        return data

    def generate_predicate(me, pattern, peg, result, stream, failure, tail, peg_args):
        data = """
{
    bool %s = true;
    %s
    if (!%s){
        %s
    }
}
""" % (pattern.variable, me.fixup_cpp(indent(pattern.code.strip()), peg_args), pattern.variable, failure())
        return data

    def generate_rule(me, pattern, peg, result, stream, failure, tail, peg_args):
        rule = peg.getRule(pattern.rule)
        if rule != None and rule.isInline():
            # TODO: add rule parameters and regular parameters for inlined rules
            if tail != None:
                raise Exception("Do not combine inlined rules that use tail recursion")
            def newPattern(pattern, stream, result, success):
                my_result = newResult()
                previous_position = gensym('position')
                out = [False]
                def label(n):
                    if n != False:
                        return "%s:" % n
                    return ""

                def fail():
                    if out[0] == False:
                        out[0] = newOut()
                    return "%s.setPosition(%s);\ngoto %s;" % (result, previous_position, out[0])
                pattern_result = pattern.generate_cpp(peg, my_result, stream, fail, tail, peg_args).strip()

                old_data = """
{
Result %s(%s.getPosition());
%s
%s = %s;
}
%s
%s
                """ % (my_result, result, pattern_result, result, my_result, success, label(out[0]))

                data = """
{
    int %s = %s.getPosition();
    %s
}
%s
%s
""" % (previous_position, result, indent(pattern.generate_cpp(peg, result, stream, fail, tail, peg_args)), success, label(out[0]))
                return data

            success_out = gensym('success')
            data = """
%s
%s
%s:
;
""" % ('\n'.join([newPattern(pattern, stream, result, "goto %s;" % success_out).strip() for pattern in rule.patterns]), failure(), success_out)
            return data
        else:
            # TODO: add rule parameters here
            if tail != None:
                if len(tail) == 0:
                    return ""
                else:
                    if pattern.parameters == None or len(tail) != len(pattern.parameters):
                        raise Exception("Expected parameters %s but got %s while calling rule '%s'" % (tail, pattern.parameters, pattern.rule))
                    return '\n'.join(["%s = %s;" % (q[0], me.fixup_cpp(q[1], peg_args)) for q in zip(tail, pattern.parameters)])
            else:
                def change(arg):
                    if arg.startswith('@'):
                        return arg[1:]
                    if peg.getRule(arg) == None:
                        raise Exception("Cannot find rule '%s' while trying to call rule '%s'" % (arg, pattern.rule))
                    return '(void*) rule_%s' % arg
                rule_parameters = ""
                if pattern.rules != None:
                    rule_parameters = ", %s" % ", ".join([change(f) for f in pattern.rules])
                parameters = ""
                if pattern.parameters != None:
                    parameters = ", %s" % ", ".join([me.fixup_cpp(p, peg_args) for p in pattern.parameters])
                    # parameters = ", %s" % fix_param(pattern.parameters)
                data = """
%s = rule_%s(%s, %s.getPosition()%s%s);
if (%s.error()){
    %s
}
""" % (result, pattern.rule, stream, result, rule_parameters, parameters, result, indent(failure()))

                return data

    def generate_void(me, pattern, peg, result, stream, failure, tail, peg_args):
        return ""

    def generate_eof(me, pattern, peg, result, stream, failure, tail, peg_args):
        data = """
if ('\\0' == %s.get(%s.getPosition())){
    %s.nextPosition();
    %s.setValue(Value((void *) '\\0'));
} else {
    %s
}
""" % (stream, result, result, result, indent(failure()))
        return data

    def generate_sequence(me, pattern, peg, result, stream, failure, tail, peg_args):
        if len(pattern.patterns) == 1:
            return pattern.patterns[0].generate_cpp(peg, result, stream, failure, tail, peg_args)
        else:
            # for each pattern, save the result in a temporary variable. only create
            # temporaries if the result is used. looking up a variable through the
            # 'args' accessor tells the code generator to generate the variable
            data = []
            def invalid(d):
                raise Exception("Invalid result %s" % d)
            args = invalid
            use_args = []
            arg_num = 0

            fail = False
            for apattern in pattern.patterns:
                use_args.append("")
                do_tail = None
                if apattern == pattern.patterns[-1]:
                    do_tail = tail
                else:
                    # lexical scope is broken so we need another function here
                    def make(n, old_arg, my_result):
                        def get(d):
                            # print "Looking for %s arg_num is %d result is %s. previous is %s" % (d, n, my_result, old_arg)
                            if d == n:
                                use_args[n-1] = "Result %s = %s;" % (my_result, result)
                                return my_result
                            return old_arg(d)
                        return get
                    arg_num += 1
                    args = make(arg_num, args, newResult())

                data.append("""
%s
""" % (indent(apattern.generate_cpp(peg, result, stream, failure, do_tail, args).strip())))

            return "{\n%s\n}" % indent('\n'.join(["%s\n%s" % (x[0], x[1]) for x in zip(data, use_args)]))

    def generate_repeat_once(me, pattern, peg, result, stream, failure, tail, peg_args):
        loop_done = gensym("loop")
        my_fail = lambda : "goto %s;" % loop_done
        my_result = newResult()
        data = """
%s.reset();
do{
    Result %s(%s.getPosition());
    %s
    %s.addResult(%s);
} while (true);
%s:
if (%s.matches() == 0){
    %s
}
""" % (result, my_result, result, indent(pattern.next.generate_cpp(peg, my_result, stream, my_fail, tail, peg_args).strip()), result, my_result, loop_done, result, indent(failure()))

        return data

    def generate_code(me, pattern, peg, result, stream, failure, tail, peg_args):
        data = """
{
    Value value((void*) 0);
    %s
    %s.setValue(value);
}
        """ % (me.fixup_cpp(indent(pattern.code.strip()), peg_args), result)

        return data

    def generate_repeat_many(me, pattern, peg, result, stream, failure, tail, peg_args):
        loop_done = gensym("loop")
        my_fail = lambda : "goto %s;" % loop_done
        my_result = newResult()
        data = """
%s.reset();
do{
    Result %s(%s.getPosition());
    %s
    %s.addResult(%s);
} while (true);
%s:
;
        """ % (result, my_result, result, indent(pattern.next.generate_cpp(peg, my_result, stream, my_fail, tail, peg_args).strip()), result, my_result, loop_done)
        return data

    def generate_any(me, pattern, peg, result, stream, failure, tail, peg_args):
        temp = gensym()
        data = """
char %s = %s.get(%s.getPosition());
if (%s != '\\0'){
    %s.setValue(Value((void*) (long) %s));
    %s.nextPosition();
} else {
    %s
}
""" % (temp, stream, result, temp, result, temp, result, indent(failure()))
        return data

    def generate_maybe(me, pattern, peg, result, stream, failure, tail, peg_args):
        save = gensym("save")
        fail = lambda : """
%s = Result(%s);
%s.setValue(Value((void*) 0));
""" % (result, save, result)
        data = """
int %s = %s.getPosition();
%s
""" % (save, result, pattern.pattern.generate_cpp(peg, result, stream, fail, tail, peg_args))
        return data

    def generate_or(me, pattern, peg, result, stream, failure, tail, peg_args):
        data = ""
        success = gensym("success")
        for pattern in pattern.patterns:
            # TODO: lazily create this
            out = gensym("or")
            my_result = newResult()
            fail = lambda : "goto %s;" % out
            if pattern == pattern.patterns[-1]:
                fail = failure
            data += """
{
Result %s(%s.getPosition());
%s
%s = %s;
}
goto %s;
%s:
""" % (my_result, result, pattern.generate_cpp(peg, my_result, stream, fail, tail, peg_args).strip(), result, my_result, success, out)
        data += "%s:\n" % success
        return data

    def generate_bind(me, pattern, peg, result, stream, failure, tail, peg_args):
        if pattern.pattern.isLineInfo():
            name = gensym("line_info");
            data = """
Stream::LineInfo %s = %s.getLineInfo(%s.getPosition());
%s = &%s;
""" % (name, stream, result, pattern.variable, name)
        else:
            data = """
%s
%s = %s.getValues();
""" % (pattern.pattern.generate_cpp(peg, result, stream, failure, tail, peg_args).strip(), pattern.variable, result)
        return data

    def generate_range(me, pattern, peg, result, stream, failure, tail, peg_args):
        letter = gensym("letter")
        data = """
char %s = %s.get(%s.getPosition());
if (%s != '\\0' && strchr("%s", %s) != NULL){
    %s.nextPosition();
    %s.setValue(Value((void*) (long) %s));
} else {
    %s
}
""" % (letter, stream, result, letter, pattern.range, letter, result, result, letter, indent(failure()))
        return data

    def generate_verbatim(me, pattern, peg, result, stream, failure, tail, peg_args):
        def doString():
            length = len(pattern.letters)
            if special_char(pattern.letters):
                length = 1
            comparison = "compareChar"
            if pattern.options == "{case}":
                comparison = "compareCharCase"
            data = """
%s.setValue(Value((void*) "%s"));
for (int i = 0; i < %d; i++){
    if (%s("%s"[i], %s.get(%s.getPosition()))){
        %s.nextPosition();
    } else {
        %s
    }
}
    """ % (result, pattern.letters.replace('"', '\\"'), length, comparison, pattern.letters.replace('"', '\\"'), stream, result, result, indent(indent(failure())))
            return data
        def doAscii():
            data = """
%s.setValue(Value((void*) %s));
if ((unsigned char) %s.get(%s.getPosition()) == (unsigned char) %s){
    %s.nextPosition();
} else {
    %s
}
"""
            return data % (result, pattern.letters, stream, result, pattern.letters, result, indent(failure()))

        if type(pattern.letters) == type('x'):
            return doString()
        elif type(pattern.letters) == type(0):
            return doAscii()
        else:
            raise Exception("unknown verbatim value %s" % pattern.letters)

# self is the peg
def generate(self, parallel = False, separate = None, directory = '.', main = False):
    def prototype(rule):
        rule_parameters = ""
        if rule.rules != None:
            # rule_parameters = ", " + ", ".join(['Result (*%s)(Stream &, const int, ...)' % name for name in rule.rules])
            rule_parameters = ", " + ", ".join(['void *%s' % name for name in rule.rules])
        parameters = ""
        if rule.parameters != None:
            parameters = ", " + ", ".join(["Value %s" % p for p in rule.parameters])
        return "Result rule_%s(Stream &, const int%s%s);" % (rule.name, rule_parameters, parameters)

    # r = 0
    use_rules = [rule for rule in self.rules if not rule.isInline()]
    # rule_numbers = '\n'.join(["const int RULE_%s = %d;" % (x[0].name, x[1]) for x in zip(use_rules, range(0, len(use_rules)))])


    chunk_accessors = []
    def findAccessor(rule):
        for accessor in chunk_accessors:
            if accessor.rule == rule:
                return accessor
        raise Exception("Cannot find accessor for " + rule.name)

    def makeChunks(rules):
        import math

        values_per_chunk = 5
        #values_per_chunk = int(math.sqrt(len(rules)))
        #if values_per_chunk < 5:
        #    values_per_chunk = 5
        all = []
        pre = ""
        chunk_to_rules = {}
        for i in xrange(0,int(math.ceil(float(len(rules)) / values_per_chunk))):
            values = rules[i*values_per_chunk:(i+1)*values_per_chunk]
            name = "Chunk%d" % i
            chunk_to_rules[name.lower()] = values
            chunk_accessors.extend([Accessor(".%s" % name.lower(), "->chunk_%s" % rule.name, name, rule) for rule in values])

            value_data = """
struct %s{
%s
};
""" % (name, indent("\n".join(["Result chunk_%s;" % rule.name for rule in values])))
            all.append(name)
            pre += value_data

        def sumChunk(chunk):
            data = """
(%s != NULL ? (%s) : 0)
""" % (chunk, '\n+ '.join(["(%s->chunk_%s.calculated() ? 1 : 0)" % (chunk, rule.name) for rule in chunk_to_rules[chunk]]))
            return data

        hit_count = '+'.join([sumChunk(chunk) for chunk in chunk_to_rules.keys()])
        # Disable for now
        hit_count = "0"

        data = """
%s
struct Column{
Column():
    %s{
}

%s

int hitCount(){
    return %s;
}

int maxHits(){
    return %s;
}

~Column(){
    %s
}
};
""" % (pre, indent(indent("\n,".join(["%s(0)" % x.lower() for x in all]))), indent("\n".join(["%s * %s;" % (x, x.lower()) for x in all])), hit_count, len(rules), indent(indent("\n".join(["delete %s;" % x.lower() for x in all]))))

        return data

    chunks = makeChunks(use_rules)

    top_code = ""
    if self.include_code != None:
        top_code = self.include_code
    more_code = ""
    if self.more_code != None:
        more_code = self.more_code

    namespace_start = self.cppNamespaceStart()
    namespace_end = self.cppNamespaceEnd()

    def singleFile():
        data = """
%s

#include <list>
#include <string>
#include <vector>
#include <map>
#include <fstream>
#include <sstream>
#include <iostream>
#include <string.h>

%s
%s

std::string ParseException::getReason() const {
return message;
}

int ParseException::getLine() const {
return line;
}

int ParseException::getColumn() const {
return column;
}

Result errorResult(-1);

%s

%s

%s

static const void * doParse(Stream & stream, bool stats, const std::string & context){
errorResult.setError();
Result done = rule_%s(stream, 0);
if (done.error()){
    stream.reportError(context);
}
if (stats){
    stream.printStats();
}
return done.getValues().getValue();
}

const void * parse(const std::string & filename, bool stats = false){
Stream stream(filename);
return doParse(stream, stats, filename);
}

const void * parse(const char * in, bool stats = false){
Stream stream(in);
return doParse(stream, stats, "memory");
}

const void * parse(const char * in, int length, bool stats = false){
Stream stream(in, length);
return doParse(stream, stats, "memory");
}

%s
    """ % (top_code, namespace_start, start_cpp_code % (chunks, self.error_size), '\n'.join([prototype(rule) for rule in use_rules]), more_code, '\n'.join([rule.generate_cpp(self, findAccessor(rule)) for rule in use_rules]), self.start, namespace_end)
        return data

    def multipleFiles(name):
        prototypes = '\n'.join([prototype(rule) for rule in use_rules])
        if not main:
            for rule in use_rules:
                rule_data = rule.generate_cpp(self, findAccessor(rule))
                out = """
%s
#include "%s.h"
%s
%s
%s
""" % (top_code, name, namespace_start, rule_data, namespace_end)
                file = open('%s/%s-%s.cpp' % (directory, name, rule.name), 'w')
                file.write(out)
                file.close()

        header_guard = "_peg_%s_h_" % name
        header_data = """
#ifndef %s
#define %s

#include <list>
#include <string>
#include <vector>
#include <map>
#include <fstream>
#include <sstream>
#include <iostream>
#include <string.h>

%s
%s
%s

extern Result errorResult;
%s
#endif
""" % (header_guard, header_guard, namespace_start, start_cpp_code % (chunks, self.error_size), prototypes, namespace_end)
        if not main:
            header_file = open('%s/%s.h' % (directory, name), 'w')
            header_file.write(header_data)
            header_file.close()

        data = """
%s

#include <list>
#include <string>
#include <vector>
#include <map>
#include <fstream>
#include <sstream>
#include <iostream>
#include <string.h>
#include "%s.h"

%s
%s

std::string ParseException::getReason() const {
return message;
}

Result errorResult(-1);

static const void * doParse(Stream & stream, bool stats, const std::string & context){
errorResult.setError();
Result done = rule_%s(stream, 0);
if (done.error()){
    stream.reportError(context);
}
if (stats){
    stream.printStats();
}
return done.getValues().getValue();
}

const void * parse(const std::string & filename, bool stats = false){
Stream stream(filename);
return doParse(stream, stats, filename);
}

const void * parse(const char * in, bool stats = false){
Stream stream(in);
return doParse(stream, stats, "memory");
}

const void * parse(const char * in, int length, bool stats = false){
Stream stream(in, length);
return doParse(stream, stats, "memory");
}

%s
"""

        return data % (top_code, name, namespace_start, more_code, self.start, namespace_end)

    if separate == None:
        return singleFile()
    else:
        return multipleFiles(separate)

