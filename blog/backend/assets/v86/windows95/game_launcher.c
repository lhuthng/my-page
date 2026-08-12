#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#define CONFIG_PATH "E:\\V86GAME.INI"
#define MUTEX_NAME "V86_GAME_LAUNCHER_SINGLE_INSTANCE"
#define MAX_TEXT 1024
#define WAIT_STEP_MS 250
#define WAIT_LIMIT_MS 120000
#define MIRROR_POLL_MS 2000
#define MAX_SAVE_FILES 64
#define MAX_SAVE_PATH 260
#define MAX_SECTION 4096

static void zero_memory(void *target, DWORD size)
{
    BYTE *bytes = (BYTE *)target;
    DWORD index;

    for(index = 0; index < size; index++)
    {
        bytes[index] = 0;
    }
}

static DWORD text_length(const char *text)
{
    DWORD length = 0;

    while(text[length] != '\0')
    {
        length++;
    }

    return length;
}

static BOOL append_text(char *target, DWORD capacity, const char *value)
{
    DWORD target_length = text_length(target);
    DWORD value_length = text_length(value);
    DWORD index;

    if(target_length + value_length + 1 > capacity)
    {
        return FALSE;
    }

    for(index = 0; index <= value_length; index++)
    {
        target[target_length + index] = value[index];
    }

    return TRUE;
}

/* Append a trailing backslash to `target` unless it already ends with one. */
static void ensure_trailing_separator(char *target, DWORD capacity)
{
    DWORD length = text_length(target);
    if(length > 0 && target[length - 1] != '\\')
    {
        if(length + 1 < capacity)
        {
            target[length] = '\\';
            target[length + 1] = '\0';
        }
    }
}

static BOOL text_equal_ignore_case(const char *left, const char *right)
{
    DWORD index = 0;

    while(left[index] != '\0' && right[index] != '\0')
    {
        char a = left[index];
        char b = right[index];

        if(a >= 'A' && a <= 'Z')
        {
            a = (char)(a - 'A' + 'a');
        }
        if(b >= 'A' && b <= 'Z')
        {
            b = (char)(b - 'A' + 'a');
        }
        if(a != b)
        {
            return FALSE;
        }
        index++;
    }

    return left[index] == '\0' && right[index] == '\0';
}

static BOOL file_exists(const char *path)
{
    DWORD attributes = GetFileAttributesA(path);

    return attributes != INVALID_FILE_ATTRIBUTES &&
        (attributes & FILE_ATTRIBUTE_DIRECTORY) == 0;
}

static BOOL directory_exists(const char *path)
{
    DWORD attributes = GetFileAttributesA(path);

    return attributes != INVALID_FILE_ATTRIBUTES &&
        (attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
}

static BOOL wait_for_file(const char *path)
{
    DWORD elapsed = 0;

    while(elapsed < WAIT_LIMIT_MS)
    {
        if(file_exists(path))
        {
            return TRUE;
        }

        Sleep(WAIT_STEP_MS);
        elapsed += WAIT_STEP_MS;
    }

    return FALSE;
}

static DWORD parse_delay(const char *text)
{
    DWORD value = 0;
    DWORD index = 0;

    while(text[index] >= '0' && text[index] <= '9')
    {
        value = value * 10 + (DWORD)(text[index] - '0');
        index++;

        if(value > 60000)
        {
            return 60000;
        }
    }

    return value;
}

static void derive_working_directory(const char *executable, char *directory)
{
    DWORD index;
    DWORD last_separator = 0;
    DWORD length = text_length(executable);

    for(index = 0; index < length; index++)
    {
        if(executable[index] == '\\' || executable[index] == '/')
        {
            last_separator = index;
        }
    }

    if(last_separator == 0 || last_separator + 1 >= MAX_TEXT)
    {
        directory[0] = '\0';
        return;
    }

    for(index = 0; index < last_separator; index++)
    {
        directory[index] = executable[index];
    }

    if(last_separator == 2 && executable[1] == ':')
    {
        directory[last_separator] = '\\';
        directory[last_separator + 1] = '\0';
    }
    else
    {
        directory[last_separator] = '\0';
    }
}

static void ensure_directory(const char *path)
{
    if(!directory_exists(path))
    {
        CreateDirectoryA(path, NULL);
    }
}

static char g_save_files[MAX_SAVE_FILES][MAX_SAVE_PATH];
static DWORD g_save_file_count = 0;
static volatile LONG g_stop_mirror = 0;

/* Does `entry` contain a path separator? */
static BOOL entry_has_separator(const char *entry)
{
    DWORD index;

    for(index = 0; entry[index] != '\0'; index++)
    {
        if(entry[index] == '\\')
        {
            return TRUE;
        }
    }

    return FALSE;
}

/* `rel` is a file's path relative to the walk root (backslash-separated);
   `name` is its basename. An entry with no separator matches the basename
   anywhere; an entry with a folder matches the exact relative path. */
static BOOL is_save_entry(const char *entry, const char *rel, const char *name)
{
    if(entry_has_separator(entry))
    {
        return text_equal_ignore_case(entry, rel);
    }

    return text_equal_ignore_case(entry, name);
}

static BOOL is_save_file(const char *rel, const char *name)
{
    DWORD index;

    for(index = 0; index < g_save_file_count; index++)
    {
        if(is_save_entry(g_save_files[index], rel, name))
        {
            return TRUE;
        }
    }

    return FALSE;
}

/* Recursively copies files under src_dir into dst_root + "\" + rel_prefix,
   preserving relative structure and creating destination directories only for
   subdirectories (mirror) or as encountered (restore). When match_only is TRUE,
   only files that match a configured save entry are copied. */
static DWORD walk_copy(const char *src_dir, const char *dst_root, const char *rel_prefix, BOOL match_only)
{
    WIN32_FIND_DATAA found;
    HANDLE finder;
    char pattern[MAX_SAVE_PATH * 2];
    char src_base[MAX_SAVE_PATH * 2];
    char dst_base[MAX_SAVE_PATH * 2];
    char src_path[MAX_SAVE_PATH * 2];
    char dst_path[MAX_SAVE_PATH * 2];
    char rel[MAX_SAVE_PATH * 2];
    char child_src[MAX_SAVE_PATH * 2];
    char child_dst[MAX_SAVE_PATH * 2];
    DWORD copied = 0;

    /* Normalize the roots so we always have a clean trailing backslash. */
    src_base[0] = '\0';
    dst_base[0] = '\0';
    if(!append_text(src_base, sizeof(src_base), src_dir) ||
        !append_text(dst_base, sizeof(dst_base), dst_root))
    {
        return 0;
    }
    ensure_trailing_separator(src_base, sizeof(src_base));
    ensure_trailing_separator(dst_base, sizeof(dst_base));

    pattern[0] = '\0';
    if(!append_text(pattern, sizeof(pattern), src_base) ||
        !append_text(pattern, sizeof(pattern), "*"))
    {
        return 0;
    }

    finder = FindFirstFileA(pattern, &found);
    if(finder == INVALID_HANDLE_VALUE)
    {
        return 0;
    }

    do
    {
        rel[0] = '\0';
        if(rel_prefix[0] != '\0')
        {
            if(!append_text(rel, sizeof(rel), rel_prefix) ||
                !append_text(rel, sizeof(rel), "\\"))
            {
                continue;
            }
        }
        if(!append_text(rel, sizeof(rel), found.cFileName))
        {
            continue;
        }

        if((found.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0)
        {
            child_src[0] = '\0';
            child_dst[0] = '\0';
            if(append_text(child_src, sizeof(child_src), src_base) &&
                append_text(child_src, sizeof(child_src), found.cFileName) &&
                append_text(child_dst, sizeof(child_dst), dst_base) &&
                append_text(child_dst, sizeof(child_dst), rel))
            {
                ensure_directory(child_dst);
                copied += walk_copy(child_src, dst_root, rel, match_only);
            }
            continue;
        }

        if(match_only && !is_save_file(rel, found.cFileName))
        {
            continue;
        }

        src_path[0] = '\0';
        dst_path[0] = '\0';
        if(append_text(src_path, sizeof(src_path), src_base) &&
            append_text(src_path, sizeof(src_path), found.cFileName) &&
            append_text(dst_path, sizeof(dst_path), dst_base) &&
            append_text(dst_path, sizeof(dst_path), rel))
        {
            if(CopyFileA(src_path, dst_path, FALSE))
            {
                copied++;
            }
        }
    } while(FindNextFileA(finder, &found));

    FindClose(finder);
    return copied;
}

/* The floppy (A:) is the save transport: it only ever holds files that match a
   configured save entry. Mirror pushes matched files from the game disk to A:;
   restore pulls them back (recreating any folders). */
static void restore_all_saves(void)
{
    DWORD copied = walk_copy("A:", "D:\\", "", TRUE);
    (void)copied;
}

static void mirror_all_saves(void)
{
    DWORD copied = walk_copy("D:\\", "A:", "", TRUE);
    (void)copied;
}

static DWORD WINAPI mirror_thread(void *ignored)
{
    (void)ignored;

    while(InterlockedCompareExchange(&g_stop_mirror, 0, 0) == 0)
    {
        mirror_all_saves();
        Sleep(MIRROR_POLL_MS);
    }

    return 0;
}

static void parse_save_files(void)
{
    static char section[MAX_SECTION];
    static char key_value[MAX_SAVE_PATH + 2];
    DWORD index = 0;

    g_save_file_count = 0;
    section[0] = '\0';

    GetPrivateProfileSectionA("saves", section, sizeof(section), CONFIG_PATH);

    while(section[index] != '\0' && g_save_file_count < MAX_SAVE_FILES)
    {
        DWORD key_value_index = 0;
        char *equal = NULL;
        char *value = NULL;
        DWORD scan = 0;

        while(section[index] != '\0' &&
            key_value_index + 1 < sizeof(key_value))
        {
            key_value[key_value_index] = section[index];
            if(section[index] == '=')
            {
                equal = &key_value[key_value_index];
            }
            key_value_index++;
            index++;
        }
        key_value[key_value_index] = '\0';

        if(equal != NULL)
        {
            *equal = '\0';
            if(text_equal_ignore_case(key_value, "file"))
            {
                value = equal + 1;
            }
        }

        if(value != NULL && value[0] != '\0' && text_length(value) < MAX_SAVE_PATH)
        {
            for(scan = 0; scan < text_length(value); scan++)
            {
                g_save_files[g_save_file_count][scan] = value[scan];
            }
            g_save_files[g_save_file_count][text_length(value)] = '\0';
            g_save_file_count++;
        }

        if(section[index] == '\0')
        {
            /* A double null ends the section; a single null separates entries. */
            if(section[index + 1] == '\0')
            {
                break;
            }
            index++;
        }
    }
}

static void fail(const char *message)
{
    MessageBoxA(NULL, message, "v86 game launcher", MB_OK | MB_ICONERROR);
}

void WINAPI WinMainCRTStartup(void)
{
    HANDLE mutex;
    DWORD mutex_error;
    static char executable[MAX_TEXT];
    static char working_directory[MAX_TEXT];
    static char arguments[MAX_TEXT];
    static char audio_primer[MAX_TEXT];
    static char delay_text[32];
    static char primer_command_line[MAX_TEXT * 2];
    static char command_line[MAX_TEXT * 2];
    static STARTUPINFOA startup;
    static PROCESS_INFORMATION process;
    DWORD primer_exit_code;
    BOOL launched;
    HANDLE mirror = NULL;

    mutex = CreateMutexA(NULL, TRUE, MUTEX_NAME);
    mutex_error = GetLastError();

    if(mutex == NULL || mutex_error == ERROR_ALREADY_EXISTS)
    {
        if(mutex != NULL)
        {
            CloseHandle(mutex);
        }
        ExitProcess(0);
    }

    if(!wait_for_file(CONFIG_PATH))
    {
        fail("E:\\V86GAME.INI was not found within 120 seconds.");
        CloseHandle(mutex);
        ExitProcess(1);
    }

    executable[0] = '\0';
    working_directory[0] = '\0';
    arguments[0] = '\0';
    audio_primer[0] = '\0';
    delay_text[0] = '\0';

    GetPrivateProfileStringA(
        "game", "executable", "", executable, MAX_TEXT, CONFIG_PATH);
    GetPrivateProfileStringA(
        "game", "working_directory", "", working_directory, MAX_TEXT, CONFIG_PATH);
    GetPrivateProfileStringA(
        "game", "arguments", "", arguments, MAX_TEXT, CONFIG_PATH);
    GetPrivateProfileStringA(
        "game", "audio_primer", "", audio_primer, MAX_TEXT, CONFIG_PATH);
    GetPrivateProfileStringA(
        "game", "delay_ms", "3000", delay_text, sizeof(delay_text), CONFIG_PATH);

    if(executable[0] == '\0')
    {
        fail("The game executable is missing from E:\\V86GAME.INI.");
        CloseHandle(mutex);
        ExitProcess(2);
    }

    if(!wait_for_file(executable))
    {
        fail("The configured game executable was not found within 120 seconds.");
        CloseHandle(mutex);
        ExitProcess(3);
    }

    if(working_directory[0] == '\0')
    {
        derive_working_directory(executable, working_directory);
    }

    parse_save_files();

    /* The delay runs before the saves are touched so a v86 snapshot can be
       captured here: the launcher is up and idle but has not read A: yet, so
       restoring that snapshot still copies the visitor's own save rather than
       replaying whatever the snapshot was captured with. */
    Sleep(parse_delay(delay_text));

    if(g_save_file_count > 0)
    {
        restore_all_saves();
        mirror_all_saves();
    }

    if(audio_primer[0] != '\0')
    {
        if(!wait_for_file(audio_primer))
        {
            fail("The configured audio primer was not found.");
            CloseHandle(mutex);
            ExitProcess(4);
        }

        primer_command_line[0] = '\0';
        if(!append_text(primer_command_line, sizeof(primer_command_line), "\"") ||
            !append_text(primer_command_line, sizeof(primer_command_line), audio_primer) ||
            !append_text(primer_command_line, sizeof(primer_command_line), "\""))
        {
            fail("The audio primer path is too long.");
            CloseHandle(mutex);
            ExitProcess(5);
        }

        zero_memory(&startup, sizeof(startup));
        zero_memory(&process, sizeof(process));
        startup.cb = sizeof(startup);

        launched = CreateProcessA(
            audio_primer,
            primer_command_line,
            NULL,
            NULL,
            FALSE,
            0,
            NULL,
            working_directory[0] == '\0' ? NULL : working_directory,
            &startup,
            &process);

        if(!launched)
        {
            fail("Windows could not start the audio primer.");
            CloseHandle(mutex);
            ExitProcess(6);
        }

        CloseHandle(process.hThread);
        WaitForSingleObject(process.hProcess, 10000);
        primer_exit_code = 1;
        GetExitCodeProcess(process.hProcess, &primer_exit_code);
        CloseHandle(process.hProcess);

        if(primer_exit_code != 0)
        {
            fail("DirectSound audio priming failed.");
            CloseHandle(mutex);
            ExitProcess(7);
        }
    }

    command_line[0] = '\0';
    if(!append_text(command_line, sizeof(command_line), "\"") ||
        !append_text(command_line, sizeof(command_line), executable) ||
        !append_text(command_line, sizeof(command_line), "\""))
    {
        fail("The configured executable path is too long.");
        CloseHandle(mutex);
        ExitProcess(8);
    }

    if(arguments[0] != '\0')
    {
        if(!append_text(command_line, sizeof(command_line), " ") ||
            !append_text(command_line, sizeof(command_line), arguments))
        {
            fail("The configured game arguments are too long.");
            CloseHandle(mutex);
            ExitProcess(9);
        }
    }

    if(g_save_file_count > 0)
    {
        mirror = CreateThread(NULL, 0, mirror_thread, NULL, 0, NULL);
    }

    zero_memory(&startup, sizeof(startup));
    zero_memory(&process, sizeof(process));
    startup.cb = sizeof(startup);

    launched = CreateProcessA(
        executable,
        command_line,
        NULL,
        NULL,
        FALSE,
        0,
        NULL,
        working_directory[0] == '\0' ? NULL : working_directory,
        &startup,
        &process);

    if(!launched)
    {
        fail("Windows could not start the configured game.");
        if(mirror != NULL)
        {
            InterlockedExchange(&g_stop_mirror, 1);
            WaitForSingleObject(mirror, 2000);
            CloseHandle(mirror);
        }
        CloseHandle(mutex);
        ExitProcess(10);
    }

    CloseHandle(process.hThread);
    WaitForSingleObject(process.hProcess, INFINITE);
    CloseHandle(process.hProcess);

    if(mirror != NULL)
    {
        InterlockedExchange(&g_stop_mirror, 1);
        WaitForSingleObject(mirror, 2000);
        CloseHandle(mirror);
        mirror_all_saves();
    }

    CloseHandle(mutex);
    ExitProcess(0);
}
