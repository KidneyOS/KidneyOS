#include <kidneyos.h>
#include <stddef.h>

static void print(const char *s) {
    size_t len;
    for (len = 0; s[len]; len++);
    write(1, s, len);
}

int check(int status) {
    if (status < 0) exit(status);
    return status;
}

void _start() {
    const char *test_data = "test data";
    char buf[10] = {0};
    int status;
    int fd = check(open("/foo", O_CREATE));
    check(write(fd, test_data, 9));
    check(close(fd));
    fd = check(open("/foo", 0));
    if (check(lseek64(fd, 1, SEEK_SET)) != 1) exit (__LINE__);
    if (check(read(fd, buf, 10)) != 8) exit(__LINE__);
    for (int i = 0; i < 8; i++) {
        if (buf[i] != test_data[i + 1])
            exit(~(i << 8 | (uint8_t)buf[i]));
    }
    check(close(fd));
    check(mkdir("/d"));
    check(chdir("/d"));
    check(getcwd(buf, 3));
    if (buf[0] != '/' || buf[1] != 'd' || buf[2] != 0) exit(__LINE__);
    fd = check(open("file", O_CREATE));
    struct Stat file_info = {0};
    check(fstat(fd, &file_info));
    check(close(fd));
    if (file_info.size != 0) exit(__LINE__);
    if (file_info.type != S_REGULAR_FILE) exit(__LINE__);
    if (unlink("/d/askdfjh") != -ENOENT) exit(__LINE__);
    check(unlink("/d/file"));
    if (open("file", 0) != -ENOENT) exit(__LINE__);
    check(mkdir("/e"));
    check(rmdir("/e"));
    if (open("/e/new", O_CREATE) != -ENOENT) exit(__LINE__);
    check(sync());
    print("success!\n");
    exit(0);
}
