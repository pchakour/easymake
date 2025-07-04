# Emake

## Getting started


## Structure of Emakefile
  
  ### Variables
 
  ### Credentials 

  ### Targets

## Structure of project

  ### The root emakefile
  
When you launch the emake command, you have to do it inside a directory containing an Emakefile.
This Emakefile will be considered as "the root emakefile" of your project. That means, if you use a path to launch a target
by starting your path with "//", emake will considered that the location of your target start to the root emakefile.

## Cache management

  ### When the cache is rebuild for a target ?

  - In files change
  - Out files change

## Available functions

  - glob
  - credential_username
  - credential_password

## Contribution

## TODO

- On clean remove all outfiles ?
- Add the ability to clean a specific target
- Add public and private keyword for credentials, variables and targets
- Add error management
- Global code refactoring to do something readable