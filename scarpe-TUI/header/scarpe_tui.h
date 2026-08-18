#ifndef SCARPE_TUI_H
#define SCARPE_TUI_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SCARPE_TUI_OK 0
#define SCARPE_TUI_QUIT 1
#define SCARPE_TUI_SUBMIT 2
#define SCARPE_TUI_ERR_NULL_PTR -1
#define SCARPE_TUI_ERR_PANIC -2
#define SCARPE_TUI_ERR_IO -3
#define SCARPE_TUI_ERR_INVALID_ID -4

typedef struct ScarpeTuiContext ScarpeTuiContext;

ScarpeTuiContext *scarpe_tui_init(bool use_alternate);
void scarpe_tui_free_context(ScarpeTuiContext *context);

int scarpe_tui_render(ScarpeTuiContext *context);
int scarpe_tui_poll_events(ScarpeTuiContext *context);

int scarpe_tui_create_node(
    ScarpeTuiContext *context,
    int node_type,
    const char *text
);
int scarpe_tui_append_child(
    ScarpeTuiContext *context,
    int parent_id,
    int child_id
);

char *scarpe_tui_get_text(
    ScarpeTuiContext *context,
    int node_id
);
void scarpe_tui_free_string(char *text);

int scarpe_tui_get_clicked_button(ScarpeTuiContext *context);
int scarpe_tui_get_checkbox_state(
    ScarpeTuiContext *context,
    int node_id
);

int scarpe_tui_set_style(
    ScarpeTuiContext *context,
    int node_id,
    int fg,
    int bg,
    int attrs
);
int scarpe_tui_update_text(
    ScarpeTuiContext *context,
    int node_id,
    const char *text
);

#ifdef __cplusplus
}
#endif

#endif