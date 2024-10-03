#include <kidneyos.h>

void _start() {
    const char *test_data = "test data";
    int fd = open("/foo", O_CREATE);
    write(fd, test_data, 9);
    if (fd < 0) exit(-1);
    if (close(fd) < 0) exit(-1);
    fd = open("/foo", 0);
    char buf[10] = {0};
    if (read(fd, buf, 10) != 9) exit(-1);
    for (int i = 0; i < 9; i++) {
        if (buf[i] != test_data[i])
            exit(-1);
    }
    if (fd < 0) exit(-1);
    if (close(fd) < 0) exit(-1);
    
    exit(0);
}
