# Glitch

This is a simple port of https://github.com/sug0/go-glitch to rust.

Below is copied 1:1 of what is in the go project.

## What is the deal with the expressions?

You can think of the image as a functor that you map an expression to, for each pixel's component colors,
returning a new one. The allowed operators are:

* `+` plus
* `-` minus
* `*` multiplication
* `/` division
* `%` modulo
* `#` power of operator
* `&` bit and
* `|` bit or
* `:` bit and not
* `^` bit xor
* `<` bit left shift
* `>` bit right shift
* `?` returns 255 if left side is greater otherwise 0
* `@` attributes a weight in the range `[0, 255]` to the value on the left

The expressions are made up of operators, numbers, parenthesis, and a set of parameters:

* `c` the current value of each pixel component color
* `b` the blurred version of `c`
* `h` the horizontally flipped version of `c`
* `v` the vertically flipped version of `c`
* `d` the diagonally flipped version of `c`
* `Y` the luminosity, or grayscale component of each pixel
* `N` a noise pixel (i.e. a pixel where each component is a random value)
* `R` the red color (i.e. rgb(255, 0, 0))
* `G` the green color (i.e. rgb(0, 255, 0))
* `B` the blue color (i.e. rgb(0, 0, 255))
* `s` the value of each pixel's last saved evaluated expression
* `r` a pixel made up of a random color component from the neighboring 8 pixels
* `e` the difference of all pixels in a box, creating an edge-like effect
* `x` the current x coordinate being evaluated normalized in the range `[0, 255]`
* `y` the current y coordinate being evaluated normalized in the range `[0, 255]`
* `H` the highest valued color component in the neighboring 8 pixels
* `L` the lowest valued color component in the neighboring 8 pixels

## Examples

* `128 & (c - ((c - 150 + s) > 5 < s))`
* `(c & (c ^ 55)) + 25`
* `128 & (c + 255) : (s ^ (c ^ 255)) + 25`
