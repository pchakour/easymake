# Easymake

## Emake folder

When the emake build is started, emake will create an `.emake` folder inside your project.

This is the structure of the folder:

- cache => Cache inputs and outputs files of your target. Used to detect change.
- footprints => Containing footprints of executed targets. Used to detect the change of target to rerun it.
- out => User reserved folder to pput generated files. You can reach it with the global variable EMAKE_OUT_DIR
- workspace => This folder is used by emake to put downloaded files. You can reach it with the global variable EMAKE_WORKING_DIR