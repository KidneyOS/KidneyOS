#include <kidneyos.h>
#include <stddef.h>

static void print(const char *s) {
    size_t len;
    for (len = 0; s[len]; len++);
    write(1, s, len);
}

void _start() {
    const char *test_data = "test data";
    int fd = open("/foo", O_CREATE);
    write(fd, test_data, 9);
    if (fd < 0) exit(-1);
    if (close(fd) < 0) exit(-1);
    fd = open("/foo", 0);
    char buf[10] = {0};
    if (lseek(fd, 1, SEEK_SET) != 1) exit(-1);
    if (read(fd, buf, 10) != 8) exit(-1);
    for (int i = 0; i < 8; i++) {
        if (buf[i] != test_data[i + 1])
            exit(~(i << 8 | (uint8_t)buf[i]));
    }
    if (fd < 0) exit(-1);
    if (close(fd) < 0) exit(-1);
    print("success!\n");
    exit(0);
}
