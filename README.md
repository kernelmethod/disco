# disco

Construct a named pipe for cryptographically secure random number generation
(CRNG).

## Installation

Install the `disco` binary with

```bash
$ cargo install --git https://github.com/kernelmethod/disco.git
```

## Usage

Create a new FIFO pipe with

```bash
$ disco /path/to/pipe
```

You can then read data from the pipe using e.g.

```bash
$ head -c 1000000 /path/to/pipe > data
```

