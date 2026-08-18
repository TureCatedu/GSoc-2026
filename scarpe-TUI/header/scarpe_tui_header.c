#ifndef SCARPE_TUI_HEADER_H
#define SCARPE_TUI_HEADER_H

#include <stdbool.h>

#define SCARPE_TUI_OK 0
#define SCARPE_TUI_ERR_NULL_PTR -1
#define SCARPE_TUI_ERR_PANIC -2
#define SCARPE_TUI_ERR_IO -3
#define SCARPE_TUI_ERR_INVALID_ID -4

typedef struct ScarpeTuiContext ScarpeTuiContext;

ScarpeTuiContext *scarpe_tui_init(bool use_alternate);
int scarpe_tui_render(ScarpeTuiContext *context);
void scarpe_tui_free_context(ScarpeTuiContext *context);
int scarpe_tui_append_child(ScarpeTuiContext *context, int parent_id, int child_id);
int scarpe_tui_poll_events(ScarpeTuiContext *context);

#endif