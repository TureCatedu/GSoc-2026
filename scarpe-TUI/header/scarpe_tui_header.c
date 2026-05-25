#ifndef scarpe_tui_header_h
#define scarpe_tui_header_h

#include "stdbool.h"


// Return codes for scarpe_tui functions state 
#define SCARPE_TUI_OK 0
#define SCARPE_TUI_ERR_NULL_PTR -1
#define SCARPE_TUI_ERR_PANIC -2
#define SCARPE_TUI_ERR_IO -3
#define SCARPE_TUI_ERR_UNKNOWN -4


// Forward declaration of the ScarpeTuiContext struct, which will be defined in the implementation file. 
// This allows us to use pointers to this struct in our function declarations without needing to know its internal structure here.
typedef struct ScarpeTuiContext ScarpeTuiContext;


// Initializes the TUI context. If use_alternate is true, it will use an alternate rendering method 
ScarpeTuiContext* scarpe_tui_init(bool use_alternate);


// Renders the TUI based on the current state of the context. Returns a status code indicating success or failure.
int scarpe_tui_render(ScarpeTuiContext* context);

// Frees the resources associated with the TUI context. After calling this function, the context pointer should not be used.
void scarpe_tui_free_context(ScarpeTuiContext* context);

// Appends a child element to a parent element in the TUI context. Returns a status code indicating success or failure.
int scarpe_tui_append_child(ScarpeTuiContext* context, int parent_id, int child_id);

int scape_tui_poll_events(ScarpeTuiContext* context);

#endif 