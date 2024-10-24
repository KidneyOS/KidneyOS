#include <kidneyos.h>

void _start() {
    Timespec ts;

    clock_gettime(0, &ts);
    exit(0);
}
