victory-linux-nvidia: README FIRST
===================================

Runs your own Xbox Series dump of GTA VI on LINUX with an NVIDIA RTX card,
through Wine. A fork of cool_fox/VIctory: https://github.com/dfmbrollins/victory-linux-nvidia

  * LINUX ONLY. This download does not run on Windows. Double-clicking
    victory.exe, or copying it into the game folder, will not work. On Windows,
    use VIctory or VIctory-NVIDIA instead.
  * NO GAME FILES are included. You need your own dump: the folder that holds
    game_scarlett_master.exe and shaders.rpf.
  * This is a test build: ~20 fps in our testing (RTX 5070), with visual bugs.


WHAT YOU NEED
-------------
  * Linux, 64-bit, with an NVIDIA RTX card (10 GB of VRAM or more recommended)
    and a recent NVIDIA driver with Vulkan.
  * Wine 10 or newer (tested: wine-staging 11.22) and winetricks.
      Arch / Manjaro / EndeavourOS:  sudo pacman -S wine-staging winetricks
      Fedora:                        sudo dnf install wine winetricks
      Ubuntu / Debian / Mint:        install a recent Wine from WineHQ
                                     (https://gitlab.winehq.org/wine/wine/-/wikis/Download)
                                     and: sudo apt install winetricks
  * About 15 GB of free disk space for caches, and an internet connection the
    first time (winetricks downloads Microsoft's d3dcompiler_47).
  * Your dump of the game.


HOW TO PLAY
-----------
1. Unpack this archive anywhere, e.g.:
       tar -xzf victory-linux-nvidia-*.tar.gz

2. Open a terminal in the unpacked folder and run, with the path to YOUR dump:
       ./start.sh /path/to/your/dump
   (With no path, it asks for the folder.)

   The first launch does the one-time setup by itself:
     - creates a Wine prefix for the game
     - installs the bundled vkd3d-proton and DXVK DLLs into it
     - installs d3dcompiler_47 with winetricks
     - translates the game's shaders (several minutes; ~11 GB of cache)
   Then the game starts.

3. The FIRST RUN STUTTERS a lot, because graphics pipelines are compiled while
   you play. Watch the opening cutscene for a few minutes, then quit (close
   the window, or press Ctrl+C in the terminal).

4. Run ./start.sh again. Now it builds every pipeline the first run found
   BEFORE starting the game. This takes up to ~30 minutes, only once (and again
   after updating to a new release). Later launches only take a few seconds
   before the game starts and run much more smoothly.

5. From then on, just run:
       ./start.sh
   It remembers your dump folder.


MAKE IT FASTER (OPTIONAL, RECOMMENDED)
--------------------------------------
Wine's stock server spends 35-40% of the CPU on this game's memory mappings.
A patched server fixes that. Build it once (~5-10 minutes):
    ./linux/build-wineserver.sh
It needs git, gcc, make, flex and bison:
    Arch: sudo pacman -S base-devel git    Fedora: sudo dnf install gcc make flex bison git
    Ubuntu/Debian: sudo apt install build-essential flex bison git
start.sh uses it automatically. Run it again whenever you update Wine.

Also: close your browser and other GPU-heavy programs before playing. The
game only gets the video memory the desktop leaves free.


WHERE YOUR FILES GO
-------------------
Everything the game writes (Wine prefix, shader and pipeline caches, saves,
logs) is kept in:
    ~/.local/share/victory-linux-nvidia
so a newer release can be unpacked anywhere and reuses it. Delete that folder
to start over from scratch. Set VICTORY_LINUX_WORK to use another folder.


TROUBLESHOOTING
---------------
* "Permission denied" when running ./start.sh
      chmod +x start.sh linux/*.sh     (or run: bash start.sh /path/to/dump)
* "has no game_scarlett_master.exe"
      Point start.sh at the folder that directly contains game_scarlett_master.exe.
      To change the remembered folder, run ./start.sh /new/path
* winetricks fails
      Run:  WINEPREFIX=~/.local/share/victory-linux-nvidia/.wineprefix winetricks d3dcompiler_47
      to see the error (usually no internet connection or an old winetricks).
* The game is very slow or stutters on every run
      Make sure you ran ./start.sh a second time after the first run (step 4),
      build the patched server (above), and close the browser.
* It crashes on start
      Check your NVIDIA driver is recent and Vulkan works (vulkaninfo --summary).
      Only NVIDIA RTX cards are tested. AMD and Intel are not supported by this fork.
* Skip the pipeline build for one launch:   SKIP_WARM=1 ./start.sh
* Series S profile instead of Series X:     CONSOLE=series-s ./start.sh


WHAT'S IN HERE
--------------
  start.sh                      the one command to run
  victory.exe                   the runtime (a Windows program run through Wine)
  dlls/                         vkd3d-proton and DXVK, unmodified (see dlls/SOURCES.txt)
  linux/play.sh                 the launcher start.sh calls, with the tuned settings
  linux/build-wineserver.sh     builds the faster Wine server
  linux/README.md               details for developers
  README.md                     the project page

License: GPL-3.0 (LICENSE). The source of this build is the matching tag at
https://github.com/dfmbrollins/victory-linux-nvidia
Support the project: https://ko-fi.com/danf490581
