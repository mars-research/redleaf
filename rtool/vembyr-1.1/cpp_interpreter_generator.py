from core import CodeGenerator

class CppInterpreterGenerator(CodeGenerator):
    start_code = """
namespace Peg{

struct Value{
    typedef std::list<Value>::const_iterator iterator;

    Value():
        which(0),
        value(0),
        values(NULL){
    }

    Value(const Value & him):
    which(him.which),
    value(0),
    values(NULL){
        if (him.isData()){
            value = him.value;
        }

        if (him.isList()){
            values = him.values;
        }
    }

    Value(const void * value):
        which(0),
        value(value),
        values(NULL){
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

    inline std::list<Value*> & getValues(){
        which = 1;
        return values;
    }

    inline const std::list<Value> & getValues() const {
        throw 1;
        // return values;
    }

    ~Value(){
    }

    /*
    inline void setValues(std::list<Value> values){
        which = 1;
        values = values;
    }
    */

    const void * value;
    std::list<Value*> values;
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

    /*
    inline int matches() const {
        if (value.isList()){
            return this->value.values->size();
        } else {
            return 1;
        }
    }
    */

    inline const Value & getValues() const {
        return this->value;
    }

    void addResult(const Result & result){
        std::list<Value*> & mine = this->value.getValues();
        mine.push_back(new Value(result.getValues()));
        this->position = result.getPosition();
        this->value.which = 1;
    }

private:
    int position;
    Value value;
};

%s

class Stream;

enum RuleType{
    %s
};

class Expression{
public:
    virtual Result parse(Stream & stream, int position, Value ** arguments) = 0;
};

class ParseException: std::exception {
public:
    ParseException(const std::string & reason):
    std::exception(),
    message(reason){
    }

    std::string getReason() const;

    virtual ~ParseException() throw(){
    }

protected:
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
        for (int i = 0; i < memo_size; i++){
            memo[i] = new Column();
        }
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
        for (int i = memo_size; i < newSize; i++){
            newMemo[i] = new Column();
        }
        delete[] memo;
        memo = newMemo;
        memo_size = newSize;
    }

    std::vector<Expression*> * getRule(RuleType rule){
        return rules[rule];
    }

    void addRule(RuleType rule, std::vector<Expression*> * expressions){
        rules[rule] = expressions;
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

    std::string reportError(){
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
        out << "Read up till line " << line << " column " << column << std::endl;
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
        return out.str();
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

        for (std::map<RuleType, std::vector<Expression*>*>::iterator it = rules.begin(); it != rules.end(); it++){
            std::vector<Expression*> * expressions = (*it).second;
            for (std::vector<Expression*>::iterator expression_it = expressions->begin(); expression_it != expressions->end(); expression_it++){
                Expression * expression = *expression_it;
                delete expression;
            }
            delete expressions;
        }
    }

private:
    char * temp;
    const char * buffer;
    Column ** memo;
    int memo_size;
    int max;
    int farthest;
    std::vector<std::string> rule_backtrace;
    std::vector<std::string> last_trace;
    int last_line_info;
    std::map<int, LineInfo> line_info;
    std::map<RuleType, std::vector<Expression*>*> rules;
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

std::string ParseException::getReason() const {
    return message;
}

Result errorResult(-1);

class Failure: public std::exception {
public:
    Failure(){
    }

    virtual ~Failure() throw () {
    }
};

class Or: public Expression {
public:
    Or(int elements, ...){
        va_list arguments;
        va_start(arguments, elements);
        for (int i = 0; i < elements; i++){
            all.push_back(va_arg(arguments, Expression*));
        }
        va_end(arguments);
    }

    std::vector<Expression*> all;

    virtual ~Or(){
        for (std::vector<Expression*>::iterator it = all.begin(); it != all.end(); it++){
            delete (*it);
        }
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        for (std::vector<Expression*>::iterator it = all.begin(); it != all.end(); it++){
            try{
                Expression * expression = *it;
                return expression->parse(stream, position, arguments);
            } catch (const Failure & ignore){
            }
        }
        throw Failure();
    }
};

class Sequence: public Expression {
public:
    Sequence(int elements, ...){
        va_list arguments;
        va_start(arguments, elements);
        for (int i = 0; i < elements; i++){
            all.push_back(va_arg(arguments, Expression*));
        }
        va_end(arguments);
    }

    std::vector<Expression*> all;

    virtual ~Sequence(){
        for (std::vector<Expression*>::iterator it = all.begin(); it != all.end(); it++){
            delete (*it);
        }
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        Result out(position);
        for (std::vector<Expression*>::iterator it = all.begin(); it != all.end(); it++){
            Expression * expression = *it;
            out.addResult(expression->parse(stream, out.getPosition(), arguments));
        }
        return out;
    }
};

class Bind: public Expression {
public:
    Bind(int index, Expression * next):
    index(index),
    next(next){
    }

    int index;
    Expression * next;

    virtual ~Bind(){
        delete next;
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        Result out = next->parse(stream, position, arguments);
        // *(arguments[index]) = out.getValues();
        return out;
    }
};

class Verbatim: public Expression {
public:
    Verbatim(const std::string & data):
    data(data),
    length(data.size()){
    }

    std::string data;
    int length;

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        for (int i = 0; i < length; i++){
            if (!compareChar(data[i], stream.get(position + i))){
                throw Failure();
            }
        }
        Result out(position + length);
        out.setValue((void*) data.c_str());
        return out;
    }
};

class RepeatOnce: public Expression {
public:
    RepeatOnce(Expression * next):
    next(next){
    }

    Expression * next;

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        Result out(position);
        out.addResult(next->parse(stream, position, arguments));
        try{
            while (true){
                out.addResult(next->parse(stream, out.getPosition(), arguments));
            }
        } catch (const Failure & ignore){
        }
        return out;
    }

    virtual ~RepeatOnce(){
        delete next;
    }
};

class RepeatMany: public Expression {
public:
    RepeatMany(Expression * next):
    next(next){
    }

    Expression * next;

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        Result out(position);
        try{
            while (true){
                out.addResult(next->parse(stream, out.getPosition(), arguments));
            }
        } catch (const Failure & ignore){
        }
        return out;
    }

    virtual ~RepeatMany(){
        delete next;
    }
};

class Maybe: public Expression {
public:
    Maybe(Expression * next):
    next(next){
    }

    Expression * next;

    virtual ~Maybe(){
        delete next;
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        try{
            return next->parse(stream, position, arguments);
        } catch (const Failure & ignore){
            return Result(position);
        }
    }
};

class Eof: public Expression {
public:
    Eof(){
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        if (stream.get(position) != 0){
            throw Failure();
        }
        return Result(position + 1);
    }
};

class Not: public Expression {
public:
    Not(Expression * next):
    next(next){
    }

    Expression * next;

    virtual ~Not(){
        delete next;
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        try{
            next->parse(stream, position, arguments);
            throw Failure();
        } catch (const Failure & fail){
            return Result(position);
        }
    }
};

class Void: public Expression {
public:
    Void(){
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        return Result(position);
    }
};

class Any: public Expression {
public:
    Any(){
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        if (stream.get(position) != 0){
            Result out(position + 1);
            out.setValue((void*) stream.get(position));
            return out;
        }

        throw Failure();
    }
};

class Ensure: public Expression {
public:
    Ensure(Expression * next):
    next(next){
    }

    Expression * next;

    virtual ~Ensure(){
        delete next;
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        next->parse(stream, position, arguments);
        return Result(position);
    }
};

class Line: public Expression {
public:
    Line(){
    }

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        /* FIXME */
        return Result(position);
    }
};

class Range: public Expression {
public:
    Range(const char * letters):
    letters(letters){
    }

    const char * letters;

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        char get = stream.get(position);
        if (strchr(letters, get) != NULL){
            Result out(position + 1);
            out.setValue((void*) get);
            return out;
        }

        throw Failure();
    }
};

class Code: public Expression {
public:
    typedef Value (*function)(Value ** arguments);

    Code(function call):
    call(call){
    }

    function call;

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        // Value value = call(arguments);
        Result out(position);
        // out.setValue(value);
        return out;
    }
};

class Rule: public Expression {
public:
    typedef Result (*rule_function)(Stream & stream, int position, Value ** arguments);
    Rule(rule_function function, ...):
    function(function){
    }

    rule_function function;

    virtual Result parse(Stream & stream, int position, Value ** arguments){
        return function(stream, position, arguments);
    }
};

} /* Peg */

typedef Peg::Result Result;
typedef Peg::Stream Stream;
typedef Peg::Value Value;
typedef Peg::ParseException ParseException;
typedef Peg::Column Column;
"""

    def __init__(self):
        self.extra_codes = []

    def addCode(self, name, code):
        data = """
Value %s(Value ** arguments){
    Value value;
    %s
    return value;
}
""" % (name,
       ""
       #code
       )
        self.extra_codes.append(data)

    def maybe_inline(self, pattern, rule, peg):
        if isinstance(pattern, PatternRule) and peg.getRule(pattern.rule).inline and pattern.rule != rule.name:
            return PatternOr(peg.getRule(pattern.rule).patterns)
        return pattern

    def generate_sequence(self, pattern, rule, peg):
        real_patterns = []
        for subpattern in pattern.patterns:
            real_patterns.append(self.maybe_inline(subpattern, rule, peg))
        if len(real_patterns) == 1:
            return real_patterns[0].generate_v3(self, rule, peg)
        data = "new Peg::Sequence(%d, %s)" % (len(pattern.patterns), ", ".join([subpattern.generate_v3(self, rule, peg) for subpattern in real_patterns]))
        return data

    def generate_bind(self, pattern, rule, peg):
        variable_index = 0
        data = "new Peg::Bind(%s, %s)" % (variable_index, pattern.pattern.generate_v3(self, rule, peg))
        return data

    def generate_repeat_once(self, pattern, rule, peg):
        data = "new Peg::RepeatOnce(%s)" % self.maybe_inline(pattern.next, rule, peg).generate_v3(self, rule, peg)
        return data

    def generate_repeat_many(self, pattern, rule, peg):
        data = "new Peg::RepeatMany(%s)" % self.maybe_inline(pattern.next, rule, peg).generate_v3(self, rule, peg)
        return data

    def generate_maybe(self, pattern, rule, peg):
        data = "new Peg::Maybe(%s)" % pattern.pattern.generate_v3(self, rule, peg)
        return data

    def generate_eof(self, pattern, rule, peg):
        data = "new Peg::Eof()"
        return data
    
    def generate_void(self, pattern, rule, peg):
        data = "new Peg::Void()"
        return data

    def generate_ensure(self, pattern, rule, peg):
        data = "new Peg::Ensure(%s)" % pattern.next.generate_v3(self, rule, peg)
        return data
    
    def generate_not(self, pattern, rule, peg):
        data = "new Peg::Not(%s)" % self.maybe_inline(pattern.next, rule, peg).generate_v3(self, rule, peg)
        return data
    
    def generate_any(self, pattern, rule, peg):
        data = "new Peg::Any()"
        return data

    def generate_line(self, pattern, rule, peg):
        data = "new Peg::Line()"
        return data

    def generate_range(self, pattern, rule, peg):
        data = 'new Peg::Range("%s")' % pattern.range
        return data

    def generate_verbatim(self, pattern, rule, peg):
        data = 'new Peg::Verbatim("%s")' % pattern.letters.replace('"', '\\"')
        return data

    def generate_code(self, pattern, rule, peg):
        function = gensym('code')
        self.addCode(function, pattern.code)
        data = 'new Peg::Code(%s)' % function
        return data

    def generate_rule(self, pattern, rule, peg):
        data = 'new Peg::Rule(rule_%s)' % pattern.rule
        return data

    def generate_or(self, pattern, rule, peg):
        data = 'new Peg::Or(%d, %s)' % (len(pattern.patterns), ", ".join([p.generate_v3(self, rule, peg) for p in pattern.patterns]))
        return data

def generate(self):
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

    chunks = makeChunks(self.rules)

    rule_types = ",\n".join(["Rule_%s" % rule.name for rule in self.rules])

    prototypes = "\n".join(["Result rule_%s(Stream &, int, Value ** arguments);" % rule.name for rule in self.rules])

    data = """
%s
%s
""" % (CppInterpreterGenerator.start_code % (chunks, rule_types, self.error_size), prototypes)
    rules = "\n".join([rule.generate_cpp_interpreter(self, findAccessor(rule)) for rule in self.rules])

    setup_rules = "\n".join(["stream.addRule(Peg::Rule_%s, create_rule_%s());" % (rule.name, rule.name) for rule in self.rules])

    main = """
static const void * doParse(Stream & stream, bool stats, const std::string & context){
    %s
    Peg::errorResult.setError();
    Result done = rule_%s(stream, 0, NULL);
    if (done.error()){
        std::ostringstream out;
        out << "Error while parsing " << context << " " << stream.reportError();
        throw ParseException(out.str());
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

""" % (indent(setup_rules), self.start)

    top_code = ""
    if self.include_code != None:
        top_code = self.include_code
    more_code = ""
    if self.more_code != None:
        more_code = self.more_code

    namespace_start = self.cppNamespaceStart()
    namespace_end = self.cppNamespaceEnd()

    data = """
#include <list>
#include <stdarg.h>
#include <string>
#include <map>
#include <fstream>
#include <sstream>
#include <vector>
#include <string.h>
#include <iostream>

%s
%s
%s
%s
%s
%s
%s
""" % (top_code, namespace_start, data, more_code, rules, main, namespace_end)
    return data
