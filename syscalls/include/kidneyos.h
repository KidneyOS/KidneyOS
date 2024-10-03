/*
 * KidneyOS Syscalls
 *
 * This header contains stubs for all the different syscalls that you can use in your C programs.
 * This file is automatically generated by the kidneyos-syscalls crate.
 */

#ifndef KIDNEYOS_SYSCALLS_H
#define KIDNEYOS_SYSCALLS_H

#include <stdint.h>

typedef uint16_t Pid;

typedef struct Timespec {

} Timespec;

void exit(uintptr_t code);

void fork(void);

void read(uint32_t fd, uint8_t *buffer, uintptr_t count);

void waitpid(Pid pid, int32_t *stat, int32_t options);

void execve(const int8_t *filename, const int8_t *const *argv, const int8_t *const *envp);

void nanosleep(const struct Timespec *duration, struct Timespec *remainder);

void scheduler_yield(void);

#endif  /* KIDNEYOS_SYSCALLS_H */
