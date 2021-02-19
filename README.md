# ROMST

## Intro

I started this project as an exercise for learning Rust. I have several emulators with MAME using a special place among all of them, and one of the most tedious tasks is the Rom management. There are several tools such as ClrMamePro, Romcenter or Romulus, but all of them are Windows only (or cross platform using Wine). I wanted something fast to use, from the command line (maybe a UI in the future?) and cross-platofrm and as I said, since I was elarning Rust, this seemed a cool project.

It is still a work in progress, I'll fill the documentation as I add features.

## Features and no no-features

* Gets rom database information such as:
    * Roms used across different sets
    * Sets that can be generated from another one
* Checks your files to detect missing roms, roms to be renamed, sets that can be fixed, etc.
* Only zip file support for the moment.

## Usage

You need to import a `.dat` file first, then you can do your queries and checkfor files.

### Import a Dat file

First step is to import the Mame data to a **Romst** database.

For now, **Romst** supports xml dat files only. You can find them in many places over the Internet, or use MAME to generate it using the command `mame.exe -listxml >mame.dat` (in Windows). You need to import the `.dat` file to a **Romst** database, this is done using the `import` command:

```bash
> romst import --file mame.dat
```

The command above will generate a `mame.rst` file. That's the **Romst** database. That file is basically a sqlite database with the rom information. Once you have that, you can check your romfiles or query the database.

### Info

Romst command to extract indormation from the database is, surprisingly, `info`.

#### DB Info

When executing

```bash
> romst info data -db mame.rst
```

It returns general information from the `mame.rst` database, like the numbers of sets there, unique roms, etc.

