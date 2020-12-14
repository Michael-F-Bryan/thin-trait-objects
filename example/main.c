#include "thin_trait_objects.h"
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <stdbool.h>
#include <stdalign.h>
#include <assert.h>

FileHandle *custom_file_handle();

int main(int argc, char **argv)
{
    const char *output_file = NULL;
    bool custom = false;

    for (int i = 1; i < argc; i++)
    {
        if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0)
        {
            fprintf(stderr, "Usage: %s [input] [-c|--custom]\n", argv[0]);
            return 0;
        }
        else if (strcmp(argv[i], "-c") == 0 || strcmp(argv[i], "--custom"))
        {
            custom = true;
        }
    }

    if (argc > 1)
    {
        output_file = argv[1];
    }

    // change how we construct the FileHandle based on the destination
    FileHandle *handle;

    if (custom)
    {
        handle = custom_file_handle();
    }
    else if (output_file)
    {
        handle = new_file_handle_from_path(output_file);
    }
    else
    {
        handle = new_stdout_file_handle();
    }

    if (!handle)
    {
        perror("Unable to open the file handle");
        return 1;
    }

    // Print out a nice message
    const char *msg = "Hello, World\n";
    const int len = strlen(msg);
    int bytes_written = file_handle_write(handle, msg, len);

    if (bytes_written != len)
    {
        perror("Unable to write a nice message");
        file_handle_destroy(handle);
        return 1;
    }

    // then just keep copying stdin to the file handle until we reach EOF
    char buffer[1024];

    while (fgets(buffer, sizeof buffer, stdin) != NULL)
    {
        int len = strlen(buffer);
        int bytes_written = file_handle_write(handle, buffer, len);

        if (bytes_written < 0)
        {
            errno = bytes_written;
            perror("Unable to copy from stdin to the file handle");
            file_handle_destroy(handle);
            return 1;
        }

        if (strstr(buffer, "flush"))
        {
            int ret = file_handle_flush(handle);
            if (ret != 0)
            {
                perror("Flushing failed");
                return ret;
            }
        }
    }

    file_handle_destroy(handle);
    return 0;
}

// CustomFileHandle will write data to the screen prefixed with the cummulative
// number of bytes written.
//
// Flushing a CustomFileHandle will print a message as well as all data printed
// since the last flush.
typedef struct
{
    int total_bytes_written;
    int capacity;
    char *buffer;
} CustomFileHandle;

void custom_destroy(void *handle)
{
    CustomFileHandle *custom = handle;
    free(custom->buffer);
}

int next_power_of_two(int value)
{
    int power = 1;

    while (power < value)
    {
        power *= 2;
    }
    return power;
}

int custom_write(void *handle, const char *data, int len)
{
    CustomFileHandle *custom = handle;
    custom->total_bytes_written += len;

    printf("[%d] %s", custom->total_bytes_written, data);

    // check if we need to resize our buffer.
    if (custom->total_bytes_written >= custom->capacity)
    {
        custom->capacity = next_power_of_two(custom->total_bytes_written);
        custom->buffer = realloc(custom->buffer, custom->capacity);
    }

    // append the written data to our buffer
    strncat(custom->buffer, data, len);
    custom->buffer[custom->total_bytes_written] = 0;

    return len;
}

int custom_flush(void *handle)
{
    CustomFileHandle *custom = handle;

    printf("[BEGIN FLUSH]%s[END FLUSH]", custom->buffer);

    free(custom->buffer);
    custom->total_bytes_written = 0;
    custom->buffer = malloc(16);
    custom->capacity = 16;

    return 0;
}

FileHandle *custom_file_handle()
{
    // Allocate our custom file handle
    FileHandleBuilder builder = new_file_handle_builder(
        sizeof(CustomFileHandle),
        alignof(CustomFileHandle),
        custom_destroy,
        custom_write,
        custom_flush);

    // and initialize it
    CustomFileHandle *custom = builder.place;
    custom->total_bytes_written = 0;
    custom->capacity = 16;
    custom->buffer = malloc(16);
    custom->buffer[0] = 0;

    return builder.file_handle;
}
