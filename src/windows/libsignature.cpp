// libsignature.cpp : Définit les fonctions de la bibliothèque statique.
//

#include "pch.h"
#include "framework.h"

#include <iostream>
#include <windows.h>
#include <fileapi.h>
#include <stdint.h>

typedef struct Signature {
    uint64_t inode;
    uint64_t dev;
} Signature;

DWORD WindowsFileSignature(char* file_name, uint64_t inode, uint64_t dev);

// Windows specific signature for a file
DWORD WindowsFileSignature(char* file_name, uint64_t inode, uint64_t dev) {
    // first, get file handle
    HANDLE fh = CreateFileA(
        (LPCSTR)file_name,
        GENERIC_READ,
        FILE_SHARE_READ,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL);

    if (fh == INVALID_HANDLE_VALUE) {
        return GetLastError();
    }

    // now call specific API to get inod & dev Windows equivalent
    BY_HANDLE_FILE_INFORMATION info;
    BOOL success = GetFileInformationByHandle(fh, &info);
    if (!success) {
        return GetLastError();
    }

    // as GetFileInformationByHandle() APIs returns 2 32-bits integers, new to combine into a 64-bits one
    inode = info.nFileIndexHigh;
    inode <<= 32;
    inode |= info.nFileIndexLow;

    dev = info.dwVolumeSerialNumber;

    return 0;
}
