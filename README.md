# dicelint

A command-line linter for dice notation.

Loot tables, damage tables, and random encounter files are usually just
plain text full of expressions like `2d6+3` or `4d4kh1`. They're easy to
write and easy to get wrong: a stray zero turns `1d6` into `1d0`, a copy-paste
leaves `0d8` sitting in a table, a typo turns `2d6` into `2000d6`. None of
that fails until something tries to actually roll the line, and by then it's
in a data file nobody is looking at closely.

dicelint reads a file with one dice expression per line and reports problems
with a line and column number, the way a compiler would, so you catch them
before the file ships.

## Supported notation

```
[N]dM[kh#|kl#][!]
```

- `N` - number of dice (defaults handled per rule, see below)
- `M` - number of sides
- `kh#` / `kl#` - keep the highest/lowest `#` dice
- `!` - exploding dice
- terms can be chained with `+` and `-`, e.g. `4d6kh3 + 2 - 1d4`
- a term can be scaled with `*`, e.g. `2d6*3` or `(1d6+2)*4`
- terms can be grouped with parentheses, including nested groups, e.g.
  `(2d4kh1! - 1) * 2 + 3d6`
- lines that are blank or start with `#` are treated as comments and skipped

## Example

`loot_table.txt`:

```
# tier 1 chest
2d6 + 3
0d6
1d0
5000d8
3d6kh5
```

```
$ dicelint loot_table.txt
loot_table.txt:3:1: error[E001]: dice count is zero, this term always contributes nothing
loot_table.txt:4:1: error[E002]: a die cannot have zero sides
loot_table.txt:5:1: warning[W002]: dice count 5000 is unusually large, check for a typo
loot_table.txt:6:1: warning[W003]: keep modifier keeps all dice, it has no effect here
$ echo $?
1
```

By default dicelint is strict: any warning fails the run, same as an error.
For a file you've already accepted as noisy, or during a migration, pass
`--lenient` so only errors (not warnings) cause a non-zero exit:

```
$ dicelint --lenient loot_table.txt
loot_table.txt:3:1: error[E001]: dice count is zero, this term always contributes nothing
loot_table.txt:4:1: error[E002]: a die cannot have zero sides
loot_table.txt:5:1: warning[W002]: dice count 5000 is unusually large, check for a typo
loot_table.txt:6:1: warning[W003]: keep modifier keeps all dice, it has no effect here
$ echo $?
1
```

(this example still exits 1 because of the two errors on lines 3 and 4 -
`--lenient` only changes how warnings are treated)

## Current rules

| code | severity | meaning |
|------|----------|---------|
| E000 | error | the expression doesn't parse |
| E001 | error | dice count is zero |
| E002 | error | die has zero sides |
| E003 | error | keep modifier keeps zero dice |
| E004 | error | a `*` multiplier is zero |
| W001 | warning | die has exactly one side (`d1`) |
| W002 | warning | dice count is over 1000 |
| W003 | warning | keep modifier keeps all the dice it rolled |

## Building

No dependencies, just the standard library:

```
cargo build --release
./target/release/dicelint some_file.txt
```

## Status

Early. The rule set above is what's implemented, not a promise of what's
final - see the issue tracker for what's planned next.
