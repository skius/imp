# IMP
This repository provides implementations for various semantics for the IMP language of [ETHZ's Formal Methods](https://infsec.ethz.ch/education/ss2021/fmfp.html) course.

## Implemented Language

Currently only core IMP is supported.

## Implemented Semantics

The following semantics are supported:
- big-step semantics (aka. natural semantics)
- small-step semantics (aka.  structural operational semantics)
- axiomatic semantics


## Installation

To install the interpreters, clone the git repo, grab the [Z3 Prover's](https://github.com/Z3Prover/z3/releases) library
files corresponding to your OS (e.g. for Windows `libz3.dll` and `libz3.lib` from the `*-win.zip` archive)  and build
the project.

```shell
$ git clone git@github.com:skius/imp
$ cd imp/
$ # put the z3 files here
$ cargo build
```

## Usage

```
./imp <filename> <true/false: run big-step> <true/false: run small-step> <true/false: run axiomatic>
```
For example, `./imp examples/square.imp true true true` evaluates `examples/square.imp` with both big-step and
small-step semantics and verifies the given derivations.

### IMP Syntax
The core IMP syntax is precisely the same as introduced in the lecture (incl. shorthands). 
The syntax to verify axiomatic derivations is the same as the introduced "Proof Outline", except that there
are no semi-colons.

Example (`square.imp`, squares `a` and stores it in `b` using only addition):
```
{a >= 0}
⊨
{a >= 0 and 0 = 0 and 0 = 0}
b := 0
{a >= 0 and b = 0 and 0 = 0}
i := 0
{a >= 0 and b = 0 and i = 0}
⊨
{i <= a and b = a * i}
while (i # a) do
    {i # a and (i <= a and b = a * i)}
    ⊨
    {i # a and (i <= a and b + a = a * (i + 1))}
    b := b + a
    {i # a and (i <= a and b = a * (i + 1))}
    ⊨
    {i + 1 <= a and b = a * (i + 1)}
    i := i + 1
    {i <= a and b = a * i}
end
{not (i # a) and (i <= a and b = a * i)}
⊨
{b = a * a}
```

Note that you may use `⊨` or `|=` for rules of consequence.  
Additionally, one must be very careful
writing the conditions for `if` and `while` - their conditions must be ANDed at the highest level: 
```
# this is okay (note the parentheses that force precedence)
{i <= a and b = a * i}
while (i # a) do
    {i # a and (i <= a and b = a * i)}
    
# this is not
{i <= a and b = a * i}
while (i # a) do
    {i # a and i <= a and b = a * i}
    
# because it is equivalent to
{i <= a and b = a * i}
while (i # a) do
    {(((i # a) and i <= a) and b = a * i)}
# where the condition (i # a) is at the innermost/lowest level instead of the highest.
```