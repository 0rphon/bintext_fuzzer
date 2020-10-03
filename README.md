# bintext_fuzzer

A live fuzzer for a patched version of bintext.exe\
target.exe is a patched version of bintext.exe that automatically closes after the file has been loaded\
its also patched to not display a dialog box for a "non-standard format" warning\
binaries to test against should be put in /corpus\
after you get some results in /crashes, do --match to see what modifications were done to each binaries in /crashes to cause a crash.\
--isolate attempts to get exactly which byte modification caused the crash for each binary in /crashes\
\
info on patches:\
\
PATCH 1: make bintext close after file is loaded\
31AD jmp to 3860 = jmp 0x6B3 = E9AE060000\
shellcode exits program with exit code 0\
FC33D2B23064FF325A8B520C8B52148B\
722833C9B11833FF33C0AC3C617C022C\
20C1CF0D03F8E2F081FF5BBC4A6A8B5A\
108B1275DA8B533C03D3FF72348B5278\
03D38B722003F333C941AD03C3813847\
65745075F4817804726F634175EB8178\
086464726575E2498B722403F3668B0C\
4E8B721C03F38B148E03D35268657373\
018BDFFE4C24036850726F6368457869\
7454FF742414FF5424146A00FFD0\
\
PATCH 2: stops process from hanging with "nonstandard format" popup box after access violation error.\
2BB4 jmp to 3910 = jmp 0xD5D = E9580D0000\
shellcode exits program with exit code 888\
FC33D2B23064FF325A8B520C8B52148B\
722833C9B11833FF33C0AC3C617C022C\
20C1CF0D03F8E2F081FF5BBC4A6A8B5A\
108B1275DA8B533C03D3FF72348B5278\
03D38B722003F333C941AD03C3813847\
65745075F4817804726F634175EB8178\
086464726575E2498B722403F3668B0C\
4E8B721C03F38B148E03D35268657373\
018BDFFE4C24036850726F6368457869\
7454FF742414FF5424146878030000FF\
D0
