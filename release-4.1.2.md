# YuE2 Studio 4.1.2

Linux build of the fork: upstream **4.1.0**, which carries the fork's in-app audio editor, model folders on another disk, the Linux Web Audio fix and the never-retired spawner, plus 4.1.1's memory fix and 4.1.2's own work below.

## New

- **Your own YuE2 GGUF model can stay on any partition.** Settings - Models gains a folder of your own models, read in place beside the studio's `models/yue2-cpp/own`. Every `.gguf` in it is listed and the one you choose is opened where it lies: nothing is moved, nothing is copied. Community models still download into the studio's folder, so both are listed together, and removing a file takes it from whichever folder really holds it.
- **LoRA folders are read in place too.** Adapters gains "Add a folder": a folder you keep for ComfyUI, a trainer or anything else, and its subfolders, is remembered and read again whenever the page opens, so a file added there needs no import. Each LoRA appears as a **symlink** inside the studio's adapter folder - tens of bytes pointing at your file, never a copy of the weights - and the engine reads the weights from where they are. Removing a LoRA in the studio deletes only that link; your file is untouched. Keep the source folder at the same path, since the links are path-based.
- **Agents** reach both through `models_own_folder`, `lora_folder_keep` and `lora_folder_forget`, with the changes journaled as usual.

## Fixed

- **A LoRA taken from another disk is no longer copied.** Linking used to try a hard link and fall back to copying, so a folder on a second partition duplicated every weight. It now tries a symbolic link first and copies only where the filesystem can make neither.

## Notes

- The engine binaries are the pinned yue2.cpp build (`67f59c14`) and the HOT-Step MIDI runtime (`75923e70`), unchanged from 4.1.1.
- Linux packages carry the CUDA 12 engine build only; a Turing or newer card on a driver from 580 uses it as before.
