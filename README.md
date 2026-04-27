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
- Caches results from the archive, only updating if the archive changed
  since the last request.

## Downsides

- Slower than original `reverse-depends` (but faster than
  `checkrdepends`)
- Larger codebase than both `reverse-depends` and `checkrdepends`
- More dependencies than `reverse-depends` and `checkrdepends`

## Changes to the `reverse-depends` interface

- `-p`/`--with-provides` is a new flag which enables checking
  `Provides:` dependencies
- `-u`/`--service-url` has been removed; it's no longer applicable
  since the archive is queried directly
- `--no-ports` is a new flag which allows for skipping secondary
  architectures on ports.ubuntu.com.
- `-k`/`--pocket` is a new flag which allows for filtering to certain
  pockets of the archive
- `--proposed` is a new flag which allows for inclusion of the -proposed
  pocket when considering reverse dependencies
- `-C`/`--no-cache` is a new flag which allows refreshing the collected
  archive data even if the local cache would otherwise be used
- Searches now only query the -release pocket by default if the queried
  release is the devel release, significantly reducing the number of
  HTTP requests sent to the archive by default.

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
- Local apt cache querying
- Investigation into further performance improvements
- Ensure cache misses are minimized
- Return 1 when nothing is matched
