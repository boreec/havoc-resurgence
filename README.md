# Havoc Resurgence

This project is an ongoing 2D turn-based roguelike game that is still in its
early stages of development. The scope is not well-defined at the moment, but I
view it as a game prototype, serving as an opportunity to enhance my skills in
Rust, Bevy, and game development in general.

# Inspirations

- [Dofus](https://en.wikipedia.org/wiki/Dofus)
- [Dwarf Fortress](https://en.wikipedia.org/wiki/Dwarf_Fortress)
- [Final Fantasy Tactics Advance](https://en.wikipedia.org/wiki/Final_Fantasy_Tactics_Advance)
- [Into the Breach](https://en.wikipedia.org/wiki/Into_the_Breach)
- [NetHack](https://en.wikipedia.org/wiki/NetHack)
- [Tales of Maj'Eyal](https://en.wikipedia.org/wiki/Tales_of_Maj%27Eyal)
- [Triangle Strategy](https://en.wikipedia.org/wiki/Triangle_Strategy)

# Running the game

## Cloning repository

First of all, clone the repository:

```console
git clone https://github.com/boreec/havoc-resurgence.git
```

Move to the cloned repository folder:

```console
cd havoc-resurgence
```

## Fetching the assets

Assets are provided in a zip archive aside from the repository. It is available
via the following link: https://mega.nz/folder/h68WgZgS#wVV9Hj5B5O265B3UC4DYpQ

Once downloaded, move `assets.zip` to the current directory and unzip it:

```console
mv path/to/assets.zip .
unzip assets.zip
```

## Building the game

Requirements:

- [Rust](https://www.rust-lang.org/)

The source code can be compiled with cargo (be patient!):

```console
cargo run -r
```

Note: XUbuntu is the only tested system so far, others may require further
adjustments (see Rust and Bevy documentation).

## Unit tests (optional)

You can also run unit tests with:

```console
cargo test
```

# Credits

The following people participated to this project directly or indirectly.

- Fonts:
    - "GABOED" by Greentrik6789
