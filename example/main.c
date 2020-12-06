#include "thin_trait_objects.h"
#include <errno.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv)
{
    const char *output_file = NULL;

    for (int i = 1; i < argc; i++)
    {
        if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0)
        {
            fprintf(stderr, "Usage: %s [input]\n", argv[0]);
            return 0;
        }
    }

    if (argc > 1)
    {
        output_file = argv[1];
    }

    // change how we construct the FileHandle based on the destination
    FileHandle *handle = output_file ? new_file_handle_from_path(output_file)
                                     : new_stdout_file_handle();

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
    }

    file_handle_destroy(handle);
    return 0;
}
