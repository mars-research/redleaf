class Character:
    def __init__(self, what):
        self.what = what

    def __str__(self):
        return self.what

class Sequence:
    def __init__(self, stuff):
        subs = []
        current = []
        has_union = False
        for item in stuff:
            if isinstance(item, Union):
                has_union = True
                subs.append(Sequence(list(current)))
                current = []
            else:
                current.append(item)
        if has_union:
            self.stuff = [Union(subs + [Sequence(list(current))])]
        else:
            self.stuff = current

    def __str__(self):
        return ''.join([str(x) for x in self.stuff])

class Union:
    def __init__(self, stuff = []):
        self.stuff = stuff

    def __str__(self):
        return '|'.join([str(x) for x in self.stuff])

class Repeat:
    def __init__(self, what):
        self.what = what

    def __str__(self):
        return '%s*' % self.what

class Maybe:
    def __init__(self, what):
        self.what = what

    def __str__(self):
        return '%s?' % self.what
