from collections import defaultdict

def return_zero():
    return 0

d = defaultdict(return_zero)
r = open("serial.log","r")
w = open("out.kern_folded","w")


in_perf_stats = False
for l in r:
    if l.__contains__("End Displaying Perf stats"):
        break
    if in_perf_stats:
        #d[l]+=1
        w.write(l)
    if l.__contains__("Displaying Perf stats"):
        in_perf_stats = True
        continue

#for (k,v) in d.items():
#    w.write(k[:-1] + " " +  str(v) + "\n")

r.close()
w.close()