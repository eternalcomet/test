#include <sys/stat.h>
#include <fcntl.h>
#include <linux/time.h>

int main() {
    struct timespec ts;
    strerror();
    clock_gettime(CLOCK_REALTIME, &ts);
}