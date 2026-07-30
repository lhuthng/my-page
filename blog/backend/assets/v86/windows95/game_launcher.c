#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#define CONFIG_PATH "D:\\V86GAME.INI"
#define MUTEX_NAME "V86_GAME_LAUNCHER_SINGLE_INSTANCE"
#define MAX_TEXT 1024
#define WAIT_STEP_MS 250
#define WAIT_LIMIT_MS 120000

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

static BOOL file_exists(const char *path)
{
    DWORD attributes = GetFileAttributesA(path);

    return attributes != INVALID_FILE_ATTRIBUTES &&
        (attributes & FILE_ATTRIBUTE_DIRECTORY) == 0;
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
        fail("D:\\V86GAME.INI was not found within 120 seconds.");
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
        fail("The game executable is missing from D:\\V86GAME.INI.");
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

    Sleep(parse_delay(delay_text));

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
        CloseHandle(mutex);
        ExitProcess(10);
    }

    CloseHandle(process.hThread);
    WaitForSingleObject(process.hProcess, INFINITE);
    CloseHandle(process.hProcess);
    CloseHandle(mutex);
    ExitProcess(0);
}
