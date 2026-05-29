// os_util.cpp
#include <stdio.h>
#include <stdlib.h>

#include "common/os_utils.h"
#include "../../rust/fceux11_rust.h"

int fceu_mkdir(const char *path)
{
	return fceux11_rust_mkdir(path);
}

int fceu_mkpath(const char *path)
{
	return fceux11_rust_mkpath(path);
}

bool fceu_file_exists(const char *filepath)
{
	return fceux11_rust_file_exists(filepath) != 0;
}

int msleep(int ms)
{
	return fceux11_rust_msleep(ms);
}
