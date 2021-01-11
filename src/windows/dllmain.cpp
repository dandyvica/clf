// dllmain.cpp : Définit le point d'entrée de l'application DLL.
#include "pch.h"

BOOL APIENTRY DllMain( HMODULE hModule,
                       DWORD  ul_reason_for_call,
                       LPVOID lpReserved
                     )
{
    //switch (ul_reason_for_call)
    //{
    //case DLL_PROCESS_ATTACH:
    //case DLL_THREAD_ATTACH:
    //case DLL_THREAD_DETACH:
    //case DLL_PROCESS_DETACH:
    //    break;
    //}
    return TRUE;
}

// libsignature.cpp : Définit les fonctions de la bibliothèque statique.
//

#include <windows.h>
#include <fileapi.h>
#include <stdint.h>
#include <iostream>

typedef struct Signature {
    DWORDLONG inode;
    DWORDLONG dev;
} Signature;

extern "C" __declspec(dllexport) DWORD get_signature_a(char* file_name, Signature * signature);
extern "C" __declspec(dllexport) DWORD get_signature_w(wchar_t* file_name, Signature * signature);

// private function to get inode & dev from a file HANDLE create from either 
// CreateFileA() API (ASCII) or CreateFileW (unicode)
DWORD get_signature_from_handle(HANDLE fh, Signature* signature) {
    // call specific API to get inod & dev Windows equivalent
    BY_HANDLE_FILE_INFORMATION info;
    BOOL success = GetFileInformationByHandle(fh, &info);
    if (!success) {
        return GetLastError();
    }

    // as GetFileInformationByHandle() APIs returns 2 32-bits integers, new to combine into a 64-bits one
    DWORDLONG _inode;
    _inode = info.nFileIndexHigh;
    _inode <<= 32;
    _inode |= info.nFileIndexLow;

    signature->inode = _inode;
    signature->dev = info.dwVolumeSerialNumber;

    // close file
    DWORD rc = CloseHandle(fh);
    if (rc != 0) {
        return GetLastError();
    }

    return 0;
}

// Windows specific signature for a file for ASCII paths
DWORD get_signature_a(char* file_name, Signature* signature) {
    // first, get file handle for Ascii path
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

    return get_signature_from_handle(fh, signature);
}

// Windows specific signature for a file for ASCII paths
DWORD get_signature_w(wchar_t* file_name, Signature* signature) {
    //std::wcout << "received utf16 string = <" << file_name << ">" << std::endl;
    //std::cout << "length = " << wcslen(file_name) << std::endl;

    // first, get file handle for Ascii path
    HANDLE fh = CreateFileW(
        (LPCWSTR)file_name,
        GENERIC_READ,
        FILE_SHARE_READ,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL);

    if (fh == INVALID_HANDLE_VALUE) {
        return GetLastError();
    }

    return get_signature_from_handle(fh, signature);
}

