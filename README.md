# bintext_fuzzer
A fuzzer for a patched version of bintext.exe\
target.exe is a patched version of bintext.exe that automatically closes after the file has been loaded\
its also patched to not display a dialog box for a "non-standard format" warning\
the fuzzer requires a crashes/ folder to store crashes and corpus/ folder to pull your target exe's from\
i should have coded checks for these folders i know i know...
