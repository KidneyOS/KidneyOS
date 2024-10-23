/*
 * KidneyOS Syscalls
 *
 * This header contains stubs for all the different syscalls that you can use in your C programs.
 * This file is automatically generated by the kidneyos-syscalls crate.
 */

#ifndef KIDNEYOS_SYSCALLS_H
#define KIDNEYOS_SYSCALLS_H

#include <stdint.h>

#define O_CREATE 64

#define SEEK_SET 0

#define SEEK_CUR 1

#define SEEK_END 2

#define ENOENT 2

#define EIO 5

#define EBADF 9

#define EFAULT 14

#define EBUSY 16

#define EEXIST 17

#define EXDEV 18

#define ENODEV 19

#define ENOTDIR 20

#define EISDIR 21

#define EINVAL 22

#define EMFILE 24

#define ENOSPC 28

#define ESPIPE 29

#define EROFS 30

#define EMLINK 31

#define ERANGE 34

#define ENOSYS 38

#define ENOTEMPTY 39

#define ELOOP 40

#define SYS_EXIT 1

#define SYS_FORK 2

#define SYS_READ 3

#define SYS_WRITE 4

#define SYS_OPEN 5

#define SYS_CLOSE 6

#define SYS_WAITPID 7

#define SYS_LINK 9

#define SYS_UNLINK 10

#define SYS_EXECVE 11

#define SYS_CHDIR 12

#define SYS_GETPID 20

#define SYS_MOUNT 21

#define SYS_UNMOUNT 22

#define SYS_SYNC 36

#define SYS_RENAME 38

#define SYS_MKDIR 39

#define SYS_RMDIR 40

#define SYS_GETPPID 64

#define SYS_SYMLINK 83

#define SYS_FTRUNCATE 93

#define SYS_FSTAT 108

#define SYS_LSEEK64 140

#define SYS_GETDENTS 141

#define SYS_NANOSLEEP 162

#define SYS_SCHED_YIELD 158

#define SYS_GETCWD 183

#define S_REGULAR_FILE 1

#define S_SYMLINK 2

#define S_DIRECTORY 3

typedef uint16_t Pid;

typedef struct Stat {
  uint32_t inode;
  uint32_t nlink;
  uint64_t size;
  uint8_t type;
} Stat;

typedef struct Dirent {
  /**
   * Opaque offset value to be used with seekdir.
   */
  uint64_t offset;
  uint32_t inode;
  /**
   * Length of this directory entry in bytes.
   */
  uint16_t reclen;
  uint8_t type;
  /**
   * Null-terminated file name
   */
  uint8_t name[0];
} Dirent;

typedef struct Timespec {

} Timespec;

void exit(uintptr_t code);

Pid fork(void);

int32_t read(int32_t fd, uint8_t *buffer, uintptr_t count);

int32_t write(int32_t fd, const uint8_t *buffer, uintptr_t count);

int32_t open(const uint8_t *name, uintptr_t flags);

int32_t close(int32_t fd);

int64_t lseek64(int32_t fd, int64_t offset, int32_t whence);

int32_t getcwd(int8_t *buf, uintptr_t size);

int32_t chdir(const int8_t *path);

int32_t mkdir(const int8_t *path);

int32_t fstat(int32_t fd, struct Stat *statbuf);

int32_t unlink(const int8_t *path);

int32_t link(const int8_t *source, const int8_t *dest);

int32_t symlink(const int8_t *source, const int8_t *dest);

int32_t rename(const int8_t *source, const int8_t *dest);

int32_t rmdir(const int8_t *path);

int32_t getdents(int32_t fd, struct Dirent *output, uintptr_t size);

int32_t ftruncate(int32_t fd, uint64_t size);

int32_t sync(void);

int32_t unmount(const int8_t *path);

int32_t mount(const int8_t *device, const int8_t *target, const int8_t *filesystem_type);

Pid waitpid(Pid pid, int32_t *stat, int32_t options);

void execve(const uint8_t *elf_bytes, uintptr_t byte_count);

int32_t nanosleep(const struct Timespec *duration, struct Timespec *remainder);

int32_t scheduler_yield(void);

#endif  /* KIDNEYOS_SYSCALLS_H */
