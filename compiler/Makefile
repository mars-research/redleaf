compiler: main.o parser.o
	g++ -g main.o parser.o -o compiler

main.o: main.cpp
	g++ -c -g main.cpp -o main.o

parser.o: parser.cpp
	g++ -c -g parser.cpp -o parser.o

main.cpp: parser.h

parser.cpp: grammar.peg
	cd vembyr-1.1/; python2.7 peg.py ../grammar.peg --cpp > ../parser.cpp; cd ..

parser.h: grammar.peg
	cd vembyr-1.1/; python2.7 peg.py ../grammar.peg --h > ../parser.h; cd ..