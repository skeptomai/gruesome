## Stream of bytes starting at 0xbb8, the properties of the first object, which should be the text description

```
04 54 CE 5C 01 29 A6 CD 38 B2 46 DC 42 C2 42 B4
```

## First byte means 4 two byte words make up the text:

```
54 CE 5C 01 29 A6 CD 38
```

## Splitting them into 5 bit characters

```
0 10101 00110 01110 0 10111 00000 00001 0 01010 01101 00110 1 10011 01001 11000
    p     a     i       r    <spc>  <abbrev 32(1-1)+5 = 5>   h      a       n     d     s
```

## Character table for translation reference

```
Z-char    6789abcdef0123456789abcdef
current   --------------------------
  A0      abcdefghijklmnopqrstuvwxyz
  A1      ABCDEFGHIJKLMNOPQRSTUVWXYZ
  A2       ^0123456789.,!?_#'"/\-:()
          --------------------------
```

## Abbreviations Data
```
65 AA 80 A5 13 2D A8 05
0 11001 01101 01010 1 00000 00101 00101 | 0 00100 11001 01101 1 01010 00000  | 00101
    t     h     e     <spc> <pad> <pad>     <A1>    T     h       e   <spc>
```

## State Machine
at entry :  no state, read 2 bytes
read 3 chars from 2 bytes, remove padding, append to string
if top bit of first byte set, you are done. if not, loop to read

D1 60 

1 10100 01011 00000 
    o     f    <spc>    