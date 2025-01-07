#include <kidneyos.h>

void _start() {
    int fd=open("/a", O_CREATE);
    if (fd < 0) exit(-fd);
    ftruncate(fd, 4096);
    const char *string = "hello world!\n";
    write(fd, string, 13);
    close(fd);
    fd = open("/a", 0);
    char *addr = (char *)0x12345000;
    char *result = mmap(addr, 4096, PROT_READ, 0, fd, 0);
    if (result != addr) exit(-(intptr_t)result);
    int len = 0;
    while (result[len]) {
        if (result[len] != string[len]) exit(-1);
        len++;
    }
    write(1, result, len);
    exit(0);
}
