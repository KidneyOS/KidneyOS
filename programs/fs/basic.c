#include <kidneyos.h>

void _start() {
    const char *test_data = "test data";
    int fd = open("/foo", O_CREATE);
    write(fd, test_data, 9);
    if (fd < 0) exit(-1);
    if (close(fd) < 0) exit(-1);
    fd = open("/foo", 0);
    char buf[10] = {0};
    if (lseek(fd, SEEK_SET, 1) != 1) exit(-1);
    if (read(fd, buf, 10) != 8) exit(-1);
    for (int i = 0; i < 8; i++) {
        if (buf[i] != test_data[i + 1])
            exit(~(i << 8 | (uint8_t)buf[i]));
    }
    if (fd < 0) exit(-1);
    if (close(fd) < 0) exit(-1);
    
    exit(0);
}
