# Kazeta+
Fork of the original [Kazeta](https://github.com/kazetaos/kazeta) project that adds several features:
- multi-cart support
- multi-resolution support
- multi-audio sink support, with adjustable volume controls
- battery monitoring and clock display
- easier troubleshooting thanks to a button on the main menu to copy the session logs over to the user's SD card, as opposed to having to copy them manually via the terminal
- customization of the BIOS, from the fonts, backgrounds, logos, and everything in-between
- Steam Deck volume control and brightness control support

![Kazeta+ About page](https://i.imgur.com/kQiAVvc.png)

## Installation
I recommend installing Kazeta+ on a fresh hard drive/flash drive/SD card, as this will involve unlocking the immutable file system. I also don't want to risk having anyone's system being accidentally broken, or having their save data messed up, on their existing Kazeta install, due to how many new features have been added to Kazeta+.

Grab the vanilla ISO from the [official website](https://kazeta.org/), flash it to a USB drive, and install it. From there, head over to the [Releases](https://github.com/the-outcaster/kazeta-plus/releases), download and extract the upgrade kit tarball, and place your custom assets in the `custom_assets_template` folder:
- both the backgrounds and logos need to be in `.png` format
- background music needs to be either `.ogg` or `.wav`
- fonts need to be `.ttf`
- sfx need to be placed in a folder, and have the following:
  - `back.wav` -- for going back to a previous menu
  - `move.wav` -- navigating between options
  - `reject.wav` -- the sound that's played when the user selects "PLAY" and the text is greyed out
  - `select.wav` -- for entering the DATA menu, SETTINGS menu, etc
  - if one or more of these files are missing, the BIOS will revert to the default sound effect
  
Tips for keeping BIOS loading times to a minimum:
- use backgrounds that don't surpass the resolution of the display that you're using
- have no more than one or two background tracks, as these take up the bulk of the loading time
  - `.wav` files take up more space than `.ogg`, but this will further reduce loading times
  
Once you're done adding your custom assets, copy the upgrade kit folder to a removable media, such as a USB flash drive or SD card. Connect said media to your Kazeta console, along with a keyboard and Ethernet cable.

Once you've booted into the BIOS, press `CTRL+ALT+F3` to bring up the terminal, and login with a username and password of `gamer`. Unlock the filesystem with:
`sudo frzr-unlock`

Then connect to the Internet, as we will need to install a few Arch packages:
`sudo ethernet-connect`

Change into the directory of your removable media, and copy the upgrade kit folder to somewhere on your Kazeta install. A suitable place would be in `/home/gamer/`:
`cd /run/media/<name-of-removable-media-label>`
`cp -r kazeta-plus-upgrade-kit-1.0 ~`

Navigate to where you copied the upgrade kit folder, change into the directory, and run the upgrade script as `sudo`:
`cd ~/kazeta-plus-upgrade-kit-1.0`
`sudo ./upgrade-to-plus.sh`

Reboot when finished, and if all is well, you should see the Kazeta+ logo appear in a splash screen sequence. From here, you can change your custom assets in the Settings menu.

## Mult-Cart Logic
![Multi-cart](https://i.imgur.com/sMcHoTI.png)

To have a cart with multiple games, you'll need to have a `.kzi` file for each game. For example...
```
fedora@fedora:/run/media/fedora/multi-cart$ ls
boomerang_fu      dudelings      ex-zodiac      Fraymakers      kazeta         logs        windows-1.0.kzr
boomerang_fu.kzi  dudelings.kzi  ex-zodiac.kzi  fraymakers.kzi  linux-1.0.kzr  lost+found
```

Both your `.kzr` runtime files and `.kzi` files will need to be on the root of the removable media. If you insert a multi-game cart in your console prior to turning it on, it will immediately boot into the *first* `.kzi` file that it finds. For example, in the case above, *Dudelings* would be the game that will run (don't ask me how it works).
