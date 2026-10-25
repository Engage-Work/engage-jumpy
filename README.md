# Detroit Showdown

A hometown arcade arena game built with Engage on the Jumpy engine. A cream-and-gold pixel wordmark and blue-hour riverfront welcome you with **What up doe!**

Choose Rivet the D-cap workshop robot, Pip the courier, Leon the blue-and-silver lion, or Verse the original battle rapper. Detroit Riverfront, Eastern Market with a Marrow tribute, and Belle Isle Gardens each use a different tested arena layout. The scenery celebrates maintained architecture, food, gardens and neighborhood color; fighting stays in the playful foreground arena.

Build with `cargo build --release --locked`. Run the executable beside the `assets` folder. The macOS app bundle keeps its assets in Contents/Resources and opens directly from Finder.

Choose **Local Game**, press **Space** to join, pick a character with **A/D**, press **Space** to confirm, and add a bot or a second player. Continue to choose your Detroit arena. Player one: **A/D** move, **Space** jump, **K** grab/drop, **J** shoot/use, **L** slide, **F** ragdoll. **Escape** pauses. **F11** switches fullscreen. Controllers are supported. **Settings → Gameplay** has difficulty and bot personalities; Easy/Mixed is the default.

The menu chime, lion jump, rapper jump, landing and grab cues are original synthesized sounds. They use the normal effects volume controls. Verse is a fictional battle-rap character, with no sampled recording, copied lyrics or celebrity voice. The Marrow storefront is an artistic neighborhood tribute, not a claim of restaurant endorsement.

Detroit title/landmark artwork and four original animation sheets were created with OpenAI Image Generation. The game retains Fish Folk and Spicy Lobster's code, art, audio, credits and licenses. Included Fish Folk media remain CC BY-NC-4.0, so this edition is distributed for noncommercial use.

---

# Engage Jumpy

Maintained by Engage Work, based on Fish Folk Jumpy v0.12.2. Our maintained branch is `engage`.

Player one uses **WASD** to move, **Space** to jump, **J** to fire/use a weapon,
**K** to grab/throw, **L** to slide, and **F** to ragdoll. Bindings can be changed
in Settings → Controls. Gamepads and the second keyboard player's bindings are unchanged.

In **Settings → Gameplay**, choose **Easy** or **Hard**, then select bot personalities:
Mixed, Balanced, Aggressive, Cautious, or Playful. Easy is the default: slower reactions,
fewer accurate attacks, and deliberate hesitation. Hard keeps strong pursuit and accurate
combat. Personalities change tactics: aggressive bots rush fights, cautious bots sometimes
retreat, and playful bots get distracted. Jump and ledge movement keeps the same physics
on both difficulties.

Remixes can inject defaults in `assets/game.yaml`:

```yaml
bot_settings:
  difficulty: Easy
  personality: Mixed
```

A fighter's `.player.yaml` can set `bot_personality: Aggressive` (or Balanced, Cautious,
Playful). Mixed uses these fighter defaults and otherwise assigns a personality per bot.
An explicit personality chosen in Settings overrides fighter defaults. Saved local choices
also override a remix's game defaults; Reset in Gameplay removes that override. Settings
are stored separately from keyboard/audio settings, so existing bindings are preserved.
These preferences apply through the local match runner; network play does not read local
bot preferences.

Bots navigate between standable surfaces using full-body Rapier collision checks and the
shipped fish movement stats. They commit to jumps until landing, align launch positions with
wall clearance, drop through platforms, and avoid repeatedly attempting a failed transition.
Navigation follows the [surface-and-trajectory approach used by Surfacer](https://github.com/SnoringCatGames/surfacer), implemented for Jumpy's physics.

Bots can attack while moving and retarget living opponents. Offline matches use a fresh random
seed; online simulation keeps its deterministic seed. The native Mac build includes Finder
and Dock icons using the existing Fish Folk artwork.

This fork stores settings separately under `org.engage-work.jumpy`, so the new defaults
apply immediately and the original game's settings remain available.

Build and check with the pinned Rust toolchain:

```sh
cargo test --release --locked
cargo build --release --locked
python3 scripts/package-engage.py
```

The package script writes a native archive and SHA-256 beside it under `target/engage-release/`.
Its `jumpy-v0.12.2/` directory deliberately preserves Engage's existing managed-app launcher
layout; the archive and game version identify the Engage release. Keep each published release
immutable and update Engage's manifest with its URL, SHA-256, and byte count.

Original code licenses and credits are retained. The source is MIT/Apache-2.0; Fish Folk's
artwork and audio remain **CC BY-NC 4.0**, as described in [LICENSE](LICENSE). This fork does
not relicense those assets.

---

# Fish Folk: Jumpy

[![Build Status](https://img.shields.io/github/actions/workflow/status/fishfolk/jumpy/ci.yml?logo=github&labelColor=1e1c24&color=8bcfcf)](https://github.com/fishfolk/jumpy/actions) [![Documentation](https://img.shields.io/badge/documentation-fishfolk.github.io-green.svg?labelColor=1e1c24&color=f3ee7a)](https://fishfolk.github.io/jumpy/book) [![License](https://img.shields.io/badge/License-MIT%20or%20Apache%202-green.svg?label=license&labelColor=1e1c24&color=34925e)](./LICENSE) [![Discord](https://img.shields.io/badge/chat-on%20discord-green.svg?logo=discord&logoColor=fff&labelColor=1e1c24&color=8d5b3f)](https://discord.gg/4smxjcheE5)

![Fish Folk: Jumpy Preview](https://user-images.githubusercontent.com/24392180/151969075-399e9fea-e2de-4340-96a4-0a0e5b79c281.gif)

👉 **[Live on Kickstarter!](https://www.kickstarter.com/projects/erlendsh/fish-folk)** 👈

## Introduction

Fish Folk: Jumpy is a tactical 2D shooter, played by up to 4 players online or on a shared screen. Aim either left or right; the rest is up to clever movement and positioning in this fish-on-fish brawler! For more information about our origin story (Duck Game et.al.) and big-picture plans, see our [design document](https://www.notion.so/erlendsh/Fish-Fight-1647ed74217e4e38a59bd28f4f5bc81a).

## Web Demo

Jumpy runs in the browser! You can play [web demo][web_demo] to try out the game, without needing to install anything on your computer.

We recommend using the Chrome browser or other derivatives for best performance, or if you have issues with other browsers.

[web_demo]: https://fishfolk.github.io/jumpy/player/latest/

### Key Features (WIP)

- 2 to 4 players in either Local Multiplayer or Online Play
- Easy to pick up, emphasizing strategy over twitch reaction
- Customize characters with hats, saved to your cross-platform profile
- Create & explore user-made weapons, levels, audio and other scripted extensions
- Smart level creation tools
- Tournaments & matchmaking built in

### Status

The game is currently under re-construction as it is being rewritten to use the [Bevy] game engine. There are now 4 stable maps and 4 items in the new version, and the game is nearly ready for another release.

[Bevy]: https://bevyengine.org

## Community

### Contributing

Anyone involved in the Fish Folk community must follow our [code of conduct](https://github.com/fishfolk/jumpy/blob/main/CODE_OF_CONDUCT.md).

If you'd like to make something for Fish Folk, check out our [help-wanted](https://github.com/fishfolk/jumpy/labels/help%20wanted) issues or just ask us on [Discord](https://discord.gg/4smxjcheE5). We'll soon post an updated roadmap for the next month or two of work ahead.

Before committing and opening a PR, please run the following commands and follow their instructions:

1. `cargo clippy -- -W clippy::correctness -D warnings`
2. `cargo fmt`

### Development Build Profiles

By default, Jumpy will build without optimizations for the `jumpy_core` and `jumpy` crates. This helps reduce re-compile times at the expense of some runtime performance. If you need increased runtime performance during development, you can build with the `--profile dev-optimized` option, to optimize all of the crates:

```bash
cargo r --profile dev-optimized
```

### Learning Materials

#### Rust

- [Rusty Engine Tutorial](https://cleancut.github.io/rusty_engine/)
- [Rust sokoban](https://sokoban.iolivia.me/)
- <https://pragprog.com/titles/hwrust/hands-on-rust/>

#### Bevy

- [Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/)

## Download & play

1. Download the latest version from the [releases](https://github.com/fishfolk/jumpy/releases) page.
2. Extract the archive and run the executable. (e.g. `./jumpy` or `jumpy.exe`)

### Launcher

[A cross-platform launcher](https://github.com/spicylobstergames/SpicyLauncher) is also available for downloading and launching the game easily.

### Distro Packages

#### Arch Linux

```sh
pacman -S jumpy
```

## Building

1. Install Rust with [rustup.rs](https://rustup.rs/)
2. Clone this repository: `git clone https://github.com/fishfolk/jumpy.git`
3. `cd jumpy`
4. `cargo run`

## Credits

- [Fish Folk: Jumpy Credits](./CREDITS.md)
- Input Icons: [Kadith's Icons](https://kadith.itch.io/kadiths-free-icons) by Kadith
