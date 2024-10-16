#include <kidneyos.h>
#include <stddef.h>

static void print(const char *s) {
    size_t len;
    for (len = 0; s[len]; len++);
    write(1, s, len);
}

void _start() {
    const char *test_data = "test data";
    char buf[10] = {0};
    int status;
    int fd = open("/foo", O_CREATE);
    if (fd < 0) exit(fd);
    status = write(fd, test_data, 9);
    if (status < 0) exit(status);
    status = close(fd);
    if (status < 0) exit(status);
    fd = open("/foo", 0);
    status = lseek64(fd, 1, SEEK_SET);
    if (status != 1) exit(status);
    status = read(fd, buf, 10);
    if (status != 8) exit(-1);
    for (int i = 0; i < 8; i++) {
        if (buf[i] != test_data[i + 1])
            exit(~(i << 8 | (uint8_t)buf[i]));
    }
    if (fd < 0) exit(fd);
    status = close(fd);
    if (status < 0) exit(status);
    status = mkdir("/d");
    if (status < 0) exit(status);
    status = chdir("/d");
    if (status < 0) exit(status);
    status = getcwd(buf, 3);
    if (buf[0] != '/' || buf[1] != 'd' || buf[2] != 0) exit(-1);
    if (status < 0) exit(status);
    fd = open("file", O_CREATE);
    if (fd < 0) exit(fd);
    struct Stat statbuf;
    status = fstat(fd, &statbuf);
    status = close(fd);
    if (status < 0) exit(status);
    if (status < 0) exit(status);
    if (statbuf.size != 0) exit(0xf0);
    if (statbuf.type != S_REGULAR_FILE) exit(0xf1);
    status = unlink("file");
    if (status < 0) exit(status);
    status = open("file", 0);
    if (status != -ENOENT) exit(status == 0 ? -1 : status);
    status = rmdir("/d");
    if (status < 0) exit(status);
    status = open("/d/new", O_CREATE);
    if (status != -ENOENT) exit(status == 0 ? -1 : status);
    status = sync();
    if (status < 0) exit(status);
    print("success!\n");
    exit(0);
}