#ifndef SCARPE_TUI_H
#define SCARPE_TUI_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ScarpeTuiContext ScarpeTuiContext;

ScarpeTuiContext *scarpe_tui_init(int use_alternate);
void scarpe_tui_free_context(ScarpeTuiContext *ctx);
int scarpe_tui_create_node(ScarpeTuiContext *ctx, int node_type_code, const char *text);
int scarpe_tui_append_child(ScarpeTuiContext *ctx, int parent_id, int child_id);
int scarpe_tui_set_style(
    ScarpeTuiContext *ctx,
    int node_id,
    int foreground,
    int background,
    int modifier
);
int scarpe_tui_update_text(ScarpeTuiContext *ctx, int node_id, const char *text);
const char *scarpe_tui_get_text(ScarpeTuiContext *ctx, int node_id);
void scarpe_tui_free_string(const char *text);
int scarpe_tui_get_checkbox_state(ScarpeTuiContext *ctx, int node_id);
int scarpe_tui_set_checkbox_state(ScarpeTuiContext *ctx, int node_id, int checked);
int scarpe_tui_poll_events(ScarpeTuiContext *ctx);
int scarpe_tui_render(ScarpeTuiContext *ctx);
int scarpe_tui_get_clicked_button(ScarpeTuiContext *ctx);
int scarpe_tui_scroll_to(ScarpeTuiContext *ctx, int bottom);

#ifdef __cplusplus
}
#endif

#endif