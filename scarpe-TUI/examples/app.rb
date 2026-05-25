# Require the Scarpe-TUI library, which provides the backend functionality for rendering the Text User Interface (TUI)
require_relative '../lib/scarpe_tui'

# Initialize the Scarpe application. The `app` method sets up the TUI environment.
# The `true` argument specifies whether to use the alternate screen buffer (if supported by the terminal).
# You can change this to `false` if you want to use the main screen buffer instead.
# You can also provide a custom title for the application by passing a `title` keyword argument, e.g., `title: "My Custom App"`.
Scarpe.app(true) do
  
  # Create a vertical stack container to organize UI elements.
  # A stack arranges its child elements vertically, one below the other.
  stack do
    para " "
    # Display a welcome message to the user
    para "============ WELCOME TO THE SCARPE-TUI SHOWCASE ============"
    
    # Provide instructions for exiting the application.
    para "              Press 'q' or 'Ctrl+C' to exit"
    para "============================================================"
  end

end