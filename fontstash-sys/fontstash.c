// This files removes the need of handling patches!

// Here we include standard libraries without modifying `fontstash.h`:
#include <stdlib.h>
#include <stdio.h>

// Include implementation even in FFI
#define FONTSTASH_IMPLEMENTATION

// We don't use callbacks for drawing (we'll use iterator)
#define FONS_VERTEX_COUNT 0

#include "fontstash/src/fontstash.h"
