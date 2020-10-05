// This files removes the need of handling patches!

// Here we include standard libraries without modifying `fontstash.h`:
#include <stdlib.h>
#include <stdio.h>

// Define it in `build.rs` when actually building:
// #define FONTSTASH_IMPLEMENTATION

#include "fontstash/src/fontstash.h"
