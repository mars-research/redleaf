#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/types.h>
#include <unistd.h>

int main(int argc , char * argv [], char * envp []) {
	if (argc < 4) { return 1; }

	uid_t uid = atoi(argv[1]);
	gid_t gid = atoi(argv[2]);

	if (setgid(gid) != 0) {
		fprintf(stderr, "setgid failed: %d", errno);
		return 2;
	}

	if (setuid(uid) != 0) {
		fprintf(stderr, "setuid failed: %d", errno);
		return 3;
	}

	char ** eargv = argv + 3;
	execve(eargv[0], eargv, envp);
	fprintf(stderr, "exec failed: %d\n", errno);

	return 4;
}
