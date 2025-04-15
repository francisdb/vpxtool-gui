# vpxtool-gui

The graphical frontend is tightly integrated with the command line interface. Therefor we suggest you start with
the commandline tool to configure everything.

```shell
# Edit the default configuration. Make sure to set the path to your Visual Pinball executable and your tables folder.
> vpxtool config edit
# Launch the text based frontend to test the configuration.
> vpxtool frontend
# once everything works you can start the GUI 
> vpxtool-gui
```

## Building

The project uses the default [rust](https://www.rust-lang.org/) build tool `cargo`. To get going read the docs on
installation and first steps at https://doc.rust-lang.org/cargo/

In case you are running Linux, the graphical frontend requires some extra operating system dependencies. These are
listed in the [bevy linux dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md)
document.

```
cargo build --release
```
