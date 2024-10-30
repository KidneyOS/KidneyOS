#include <kidneyos.h>

void _start() {
    unsigned char buf[34];
    getrandom(buf, 34, 0);
    exit(1);
}
