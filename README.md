# brew-helper

A little brew helper to remove non needed formulas

## Usage

Show all formulas not used as deps:
```
brew-helper list
```

Drop formula with all deps that are not used by other formulas:
```
brew-helper rm-dep <formula>
```
