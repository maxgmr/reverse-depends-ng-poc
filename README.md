# reverse-depends-ng-poc

Proof of concept for a modernized `reverse-depends`.

## Improvements

- Handles virtual packages correctly: `Provides:` fields are
  recognized, allowing things like Rust library virtual dependencies to
  be parsed.
- Directly reads from official archive.
- Displays information about alternatives; e.g.,
  `reverse-depends-ng-poc gawk` will list
  `* auditd                        (for mawk | gawk)`
- Handles transitive dependencies. The original `reverse-depends`
  recursive search is broken.
- Allows filtering by pocket.
- Allows inclusion of the -proposed pocket.

## Downsides

- Slower than original `reverse-depends` (but faster than
  `checkrdepends`)

## Changes to the `reverse-depends` interface

- `-p`/`--with-provides` is a new flag which enables checking
  `Provides:` dependencies
- `-u`/`--service-url` has been removed; it's no longer applicable
  since the archive is queried directly

## Example usage

Broad, deep dependency graph:

```none
$ reverse-depends-ng-poc libglib2.0-0 -xd4
```

Virtual package handling:

```none
$ reverse-depends-ng-poc gawk
$ reverse-depends-ng-poc gawk -p
```

## Planned

- Debian archive querying
- Investigation into further performace improvements
- Caching to make subsequent searches faster
