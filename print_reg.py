import sys

def print_reg(n):
    for i, r in enumerate(reversed(str(bin(n))[2:])):
        print(i, r)

if __name__ == '__main__':
    print_reg(int(eval(sys.argv[1])))