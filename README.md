# disco

Generate cryptographically secure random streams.

## Installation

Install the `disco` binary with

```bash
$ cargo install --git https://github.com/kernelmethod/disco.git
```

## Usage

### FIFO pipes

To create a `/dev/urandom`-like interface for accessing random streams, run

```bash
$ mkfifo ./my-urandom
$ disco -o ./my-urandom &
```

You can then read data from the pipe using e.g.

```bash
$ head -c 1000000 ./my-urandom > data
```

